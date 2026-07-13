use btleplug::api::{Central, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Peripheral};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::loopback::LoopbackTarget;

const VESC_BLE_UART_SERVICE_UUID: Uuid = Uuid::from_u128(0x6e400001b5a3f393e0a9e50e24dcca9e);

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
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|_| DiscoveryError::InspectFailed)?;
        for peripheral in peripherals {
            let properties = match peripheral.properties().await {
                Ok(Some(properties)) => properties,
                Ok(None) | Err(_) => continue,
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
        time::sleep(Duration::from_millis(100)).await;
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
