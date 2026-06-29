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
    find_matching_peripheral_with_progress(adapter, target, |_, _| {}).await
}

pub(crate) async fn find_matching_peripheral_with_progress(
    adapter: &Adapter,
    target: &LoopbackTarget,
    mut progress: impl FnMut(DiscoveredPeripheral, bool),
) -> Result<Peripheral, DiscoveryError> {
    loop {
        if let Some(peripheral) =
            find_matching_cached_peripheral_with_progress(adapter, target, &mut progress).await?
        {
            return Ok(peripheral);
        }

        time::sleep(Duration::from_millis(100)).await;
    }
}

pub fn describe_loopback_target(target: &LoopbackTarget) -> String {
    match (target.address(), target.requires_explicit_match()) {
        (Some(address), _) => format!("address={address}"),
        (None, true) => format!("device={}", target.device_name_hint()),
        (None, false) => format!(
            "default (name={}, service={}, or VESC BLE UART service)",
            target.device_name_hint(),
            target.service_name_hint()
        ),
    }
}

pub fn describe_discovered_peripheral(device: &DiscoveredPeripheral) -> String {
    let name = device.local_name.as_deref().unwrap_or("<unnamed>");
    format!("{} name={name}", device.identifier)
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

async fn find_matching_cached_peripheral_with_progress(
    adapter: &Adapter,
    target: &LoopbackTarget,
    progress: &mut impl FnMut(DiscoveredPeripheral, bool),
) -> Result<Option<Peripheral>, DiscoveryError> {
    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|_| DiscoveryError::InspectFailed)?;

    for peripheral in peripherals {
        let Some(properties) = peripheral
            .properties()
            .await
            .map_err(|_| DiscoveryError::InspectFailed)?
        else {
            continue;
        };

        let device = DiscoveredPeripheral {
            identifier: properties.address.to_string(),
            local_name: properties.local_name,
            services: properties.services,
        };
        let matched = target_matches_properties(
            target,
            Some(&device.identifier),
            device.local_name.as_deref(),
            &device.services,
        );
        progress(device.clone(), matched);

        if matched {
            return Ok(Some(peripheral));
        }
    }

    Ok(None)
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
    fn target_matching_covers_scan_filter_and_selectors() {
        assert!(vesc_tool_scan_filter().services.is_empty());

        let service_uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);
        let named = LoopbackTarget::named("Floatwheel PintV");
        assert!(target_matches_properties(
            &named,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("Floatwheel PintV"),
            &[]
        ));
        assert!(!target_matches_properties(
            &named,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("something-else"),
            &[service_uuid]
        ));

        let addressed = LoopbackTarget::addressed("AA:BB:CC:DD:EE:FF");
        assert!(target_matches_properties(
            &addressed,
            Some("aa:bb:cc:dd:ee:ff"),
            Some("something-else"),
            &[]
        ));

        let default_target = LoopbackTarget::default();
        assert!(target_matches_properties(
            &default_target,
            Some("AA:BB:CC:DD:EE:FF"),
            Some("something-else"),
            &[service_uuid]
        ));
    }
}
