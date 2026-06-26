use btleplug::api::{Central, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Peripheral};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::loopback::LoopbackTarget;

const VESC_BLE_UART_SERVICE_UUID: Uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeripheral {
    pub identifier: String,
    pub local_name: Option<String>,
    pub services: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiscoveryError {
    InspectFailed,
}

pub(crate) fn vesc_tool_scan_filter() -> ScanFilter {
    ScanFilter::default()
}

pub(crate) async fn find_matching_peripheral(
    adapter: &Adapter,
    target: &LoopbackTarget,
) -> Result<Peripheral, DiscoveryError> {
    loop {
        if let Some(peripheral) = find_matching_cached_peripheral(adapter, target).await? {
            return Ok(peripheral);
        }

        time::sleep(Duration::from_millis(100)).await;
    }
}

pub(crate) async fn collect_discovered_peripherals(
    adapter: &Adapter,
) -> Result<Vec<DiscoveredPeripheral>, DiscoveryError> {
    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|_| DiscoveryError::InspectFailed)?;

    let mut devices = Vec::new();
    for peripheral in peripherals {
        let Some(properties) = peripheral
            .properties()
            .await
            .map_err(|_| DiscoveryError::InspectFailed)?
        else {
            continue;
        };

        devices.push(DiscoveredPeripheral {
            identifier: properties.address.to_string(),
            local_name: properties.local_name,
            services: properties.services,
        });
    }

    Ok(devices)
}

async fn find_matching_cached_peripheral(
    adapter: &Adapter,
    target: &LoopbackTarget,
) -> Result<Option<Peripheral>, DiscoveryError> {
    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|_| DiscoveryError::InspectFailed)?;

    for peripheral in peripherals {
        if peripheral_matches_target(&peripheral, target).await? {
            return Ok(Some(peripheral));
        }
    }

    Ok(None)
}

async fn peripheral_matches_target(
    peripheral: &Peripheral,
    target: &LoopbackTarget,
) -> Result<bool, DiscoveryError> {
    let Some(properties) = peripheral
        .properties()
        .await
        .map_err(|_| DiscoveryError::InspectFailed)?
    else {
        return Ok(false);
    };

    Ok(target_matches_properties(
        target,
        Some(&properties.address.to_string()),
        properties.local_name.as_deref(),
        &properties.services,
    ))
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

#[cfg(test)]
mod tests {
    use super::{target_matches_properties, vesc_tool_scan_filter};
    use crate::loopback::LoopbackTarget;
    use uuid::Uuid;

    #[test]
    fn does_not_filter_by_service_uuid() {
        assert!(vesc_tool_scan_filter().services.is_empty());
    }

    #[test]
    fn explicit_name_target_does_not_fall_back_to_service_uuid() {
        let target = LoopbackTarget::named("Floatwheel PintV");
        let service_uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);

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
            &[service_uuid]
        ));
    }

    #[test]
    fn explicit_address_target_matches_address_case_insensitively() {
        let target = LoopbackTarget::addressed("AA:BB:CC:DD:EE:FF");

        assert!(target_matches_properties(
            &target,
            Some("aa:bb:cc:dd:ee:ff"),
            Some("something-else"),
            &[]
        ));
    }

    #[test]
    fn default_target_allows_service_uuid_fallback() {
        let target = LoopbackTarget::default();
        let service_uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);

        assert!(target_matches_properties(
            &target,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("something-else"),
            &[service_uuid]
        ));
    }
}
