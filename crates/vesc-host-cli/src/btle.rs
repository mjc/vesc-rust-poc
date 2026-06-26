use crate::ble_scan::vesc_tool_scan_filter;
use crate::loopback::{LoopbackTarget, LoopbackTransport, LoopbackTransportError};
use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_util::StreamExt;
use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::time;
use uuid::Uuid;

const VESC_BLE_UART_SERVICE_UUID: Uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_RX_UUID: Uuid = Uuid::from_u128(0x6e400002b5a3f393e0a9e50e24dcca9e);
const VESC_BLE_UART_TX_UUID: Uuid = Uuid::from_u128(0x6e400003b5a3f393e0a9e50e24dcca9e);

const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug)]
struct BtleSession {
    peripheral: Peripheral,
    rx_char: Characteristic,
    responses: Receiver<Vec<u8>>,
}

#[derive(Debug)]
pub struct BtleLoopbackTransport {
    runtime: Runtime,
    session: RefCell<Option<BtleSession>>,
}

impl BtleLoopbackTransport {
    pub fn new() -> Result<Self, LoopbackTransportError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .map_err(|_| LoopbackTransportError::Device("failed to start the BLE runtime"))?;

        Ok(Self {
            runtime,
            session: RefCell::new(None),
        })
    }

    fn open_session(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        let session = self
            .runtime
            .block_on(async move { open_session(target).await })?;
        *self.session.borrow_mut() = Some(session);
        Ok(())
    }

    fn session(
        &self,
    ) -> Result<std::cell::RefMut<'_, Option<BtleSession>>, LoopbackTransportError> {
        if self.session.borrow().is_none() {
            return Err(LoopbackTransportError::Device(
                "BLE transport has not been opened",
            ));
        }

        Ok(self.session.borrow_mut())
    }
}

impl LoopbackTransport for BtleLoopbackTransport {
    fn open(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        self.open_session(target)
    }

    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError> {
        let session = self.session()?;
        let session = session.as_ref().expect("session checked above");
        self.runtime
            .block_on(session.peripheral.write(
                &session.rx_char,
                request,
                WriteType::WithoutResponse,
            ))
            .map_err(|_| LoopbackTransportError::Device("failed to write BLE request"))?;
        session.runtime_receive()
    }
}

impl BtleSession {
    fn runtime_receive(&self) -> Result<Vec<u8>, LoopbackTransportError> {
        self.responses
            .recv_timeout(RESPONSE_TIMEOUT)
            .map_err(|_| LoopbackTransportError::Device("timed out waiting for a BLE reply"))
    }
}

async fn open_session(target: LoopbackTarget) -> Result<BtleSession, LoopbackTransportError> {
    let manager = Manager::new()
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to initialize Bluetooth"))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to enumerate Bluetooth adapters"))?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or(LoopbackTransportError::ScanTimeout)?;

    adapter
        .start_scan(vesc_tool_scan_filter())
        .await
        .map_err(|_| LoopbackTransportError::Device("failed to start BLE scan"))?;

    let discovered = time::timeout(SCAN_TIMEOUT, find_matching_peripheral(&adapter, &target))
        .await
        .map_err(|_| LoopbackTransportError::ScanTimeout)??;

    let peripheral = discovered;
    time::timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| LoopbackTransportError::ConnectFailed)?
        .map_err(|_| LoopbackTransportError::ConnectFailed)?;
    time::timeout(CONNECT_TIMEOUT, peripheral.discover_services())
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?
        .map_err(|_| LoopbackTransportError::MissingService)?;

    let characteristics = peripheral.characteristics();
    let rx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_RX_UUID)
        .cloned()
        .ok_or(LoopbackTransportError::MissingService)?;
    let tx_char = characteristics
        .iter()
        .find(|characteristic| characteristic.uuid == VESC_BLE_UART_TX_UUID)
        .cloned()
        .ok_or(LoopbackTransportError::MissingService)?;

    peripheral
        .subscribe(&tx_char)
        .await
        .map_err(|_| LoopbackTransportError::MissingService)?;

    let (responses_tx, responses_rx) = mpsc::channel();
    let notification_peripheral = peripheral.clone();
    let notification_uuid = tx_char.uuid;

    tokio::spawn(async move {
        let Ok(mut notifications) = notification_peripheral.notifications().await else {
            return;
        };

        while let Some(notification) = notifications.next().await {
            if notification.uuid == notification_uuid
                && responses_tx.send(notification.value).is_err()
            {
                break;
            }
        }
    });

    Ok(BtleSession {
        peripheral,
        rx_char,
        responses: responses_rx,
    })
}

async fn find_matching_peripheral(
    adapter: &Adapter,
    target: &LoopbackTarget,
) -> Result<Peripheral, LoopbackTransportError> {
    loop {
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|_| LoopbackTransportError::Device("failed to inspect BLE peripherals"))?;

        for peripheral in peripherals {
            let properties = match peripheral.properties().await {
                Ok(Some(properties)) => properties,
                _ => continue,
            };

            if target_matches_properties(
                target,
                Some(&properties.address.to_string()),
                properties.local_name.as_deref(),
                &properties.services,
            ) {
                return Ok(peripheral);
            }
        }

        time::sleep(Duration::from_millis(250)).await;
    }
}

fn target_matches_properties(
    target: &LoopbackTarget,
    address: Option<&str>,
    local_name: Option<&str>,
    services: &[Uuid],
) -> bool {
    let address_matches = target
        .address()
        .zip(address)
        .map(|(expected, actual)| expected.eq_ignore_ascii_case(actual))
        .unwrap_or(false);
    let name_matches = local_name
        .map(|name| {
            name.eq_ignore_ascii_case(target.device_name_hint())
                || name.eq_ignore_ascii_case(target.service_name_hint())
        })
        .unwrap_or(false);
    let service_matches =
        !target.requires_explicit_match() && services.contains(&VESC_BLE_UART_SERVICE_UUID);

    address_matches || name_matches || service_matches
}

pub fn vesc_ble_uart_service_uuid() -> Uuid {
    VESC_BLE_UART_SERVICE_UUID
}

pub fn vesc_ble_uart_rx_uuid() -> Uuid {
    VESC_BLE_UART_RX_UUID
}

pub fn vesc_ble_uart_tx_uuid() -> Uuid {
    VESC_BLE_UART_TX_UUID
}

#[cfg(test)]
mod tests {
    use super::{
        target_matches_properties, vesc_ble_uart_rx_uuid, vesc_ble_uart_service_uuid,
        vesc_ble_uart_tx_uuid,
    };
    use crate::loopback::LoopbackTarget;

    #[test]
    fn exports_the_vesc_ble_uart_profile_uuids() {
        assert_eq!(
            vesc_ble_uart_service_uuid().to_string(),
            "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
        );
        assert_eq!(
            vesc_ble_uart_rx_uuid().to_string(),
            "6e400002-b5a3-f393-e0a9-e50e24dcca9e"
        );
        assert_eq!(
            vesc_ble_uart_tx_uuid().to_string(),
            "6e400003-b5a3-f393-e0a9-e50e24dcca9e"
        );
    }

    #[test]
    fn matches_a_known_vesc_service_uuid() {
        assert!(target_matches_properties(
            &LoopbackTarget::default(),
            Some("AA:BB:CC:DD:EE:FF"),
            Some("something-else"),
            &[vesc_ble_uart_service_uuid()]
        ));
    }

    #[test]
    fn falls_back_to_the_target_name_hint() {
        assert!(target_matches_properties(
            &LoopbackTarget::default(),
            Some("AA:BB:CC:DD:EE:FF"),
            Some("vesc-loopback-test"),
            &[]
        ));
    }

    #[test]
    fn rejects_unrelated_devices() {
        assert!(!target_matches_properties(
            &LoopbackTarget::default(),
            Some("AA:BB:CC:DD:EE:FF"),
            Some("other-device"),
            &[]
        ));
    }

    #[test]
    fn explicit_name_target_does_not_fall_back_to_service_uuid() {
        let target = LoopbackTarget::named("Floatwheel PintV");

        assert!(target_matches_properties(
            &target,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("Floatwheel PintV"),
            &[]
        ));
        assert!(!target_matches_properties(
            &target,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("something-else"),
            &[vesc_ble_uart_service_uuid()]
        ));
    }

    #[test]
    fn explicit_address_target_matches_address() {
        let target = LoopbackTarget::addressed("AA:BB:CC:DD:EE:FF");

        assert!(target_matches_properties(
            &target,
            Some("aa:bb:cc:dd:ee:ff"),
            Some("something-else"),
            &[]
        ));
        assert!(!target_matches_properties(
            &target,
            Some("11:22:33:44:55:66"),
            Some("something-else"),
            &[vesc_ble_uart_service_uuid()]
        ));
    }
}
