use btleplug::api::{Central, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Peripheral};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::loopback::LoopbackTarget;

const VESC_BLE_UART_SERVICE_UUID: Uuid = Uuid::from_u128(0x6e40_0001_b5a3_f393_e0a9_e50e_24dc_ca9e);

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
            #[allow(clippy::manual_let_else)]
            let properties = match peripheral.properties().await.ok().flatten() {
                Some(properties) => properties,
                None => continue,
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
        .is_some_and(|(expected, actual)| expected.eq_ignore_ascii_case(actual));
    let name_matches = local_name.is_some_and(|name| {
        name.eq_ignore_ascii_case(target.device_name_hint())
            || name.eq_ignore_ascii_case(target.service_name_hint())
    });
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

        let service_uuid = Uuid::from_u128(0x6e40_0001_b5a3_f393_e0a9_e50e_24dc_ca9e);
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
