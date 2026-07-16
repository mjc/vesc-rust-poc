#![cfg(all(not(test), target_arch = "arm"))]

use vesc_protocol::ble_loopback::handle_loopback_frame;
use vescpkg_rs::{AppDataPacket, Firmware, PackageStart, StatefulAppDataCallback};

struct LoopbackAppData;

impl StatefulAppDataCallback for LoopbackAppData {
    type State = crate::LoopbackState;

    fn handle(_state: &mut Self::State, packet: AppDataPacket<'_>) {
        let firmware = Firmware::new();
        let app_data = firmware.app_data();
        let now_ms = u64::from(app_data.system_time_ticks().as_ticks()) / 10;
        if let Ok((response, response_len)) = handle_loopback_frame(packet.as_bytes(), now_ms) {
            let _ = response
                .get(..response_len)
                .is_some_and(|response| app_data.send(response).is_ok());
        }
    }
}

vescpkg_rs::firmware_stateful_app_data_callback!(loopback_handle_app_data, LoopbackAppData);

/// Register the package-local callback that VESC stores in
/// `third_party/vesc/comm/commands.c:1820-1828`.
#[inline(always)]
pub(crate) fn register(start: &mut PackageStart) -> bool {
    start
        .app_data_callback::<LoopbackAppData>()
        .is_some_and(|callback| callback.register().is_ok())
}
