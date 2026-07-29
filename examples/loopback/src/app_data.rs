#![cfg(all(not(test), target_arch = "arm"))]

use vesc_protocol::ble_loopback::handle_loopback_frame;
use vescpkg_rs::{
    AppDataHandler, AppDataPacket, AppDataReply, Firmware, PackageStart, StatefulCallbackContext,
};

struct LoopbackAppData;

impl AppDataHandler for LoopbackAppData {
    type State = crate::LoopbackState;

    fn handle(
        _context: &mut StatefulCallbackContext<'_, Self::State>,
        packet: AppDataPacket<'_>,
        reply: &mut AppDataReply<'_>,
    ) {
        let firmware = Firmware::new();
        let now_ms = u64::from(firmware.clock().now().as_ticks()) / 10;
        if let Ok((bytes, response_len)) = handle_loopback_frame(packet.as_bytes(), now_ms) {
            let _ = bytes
                .get(..response_len)
                .is_some_and(|bytes| reply.write(bytes).is_ok());
        }
    }
}

vescpkg_rs::firmware_stateful_app_data_callback!(loopback_handle_app_data, LoopbackAppData);

/// Register the package-local callback that VESC stores in
/// `third_party/vesc/comm/commands.c:1820-1828`.
pub(crate) fn register(start: &mut PackageStart) -> Result<(), vescpkg_rs::PackageStartError> {
    start
        .app_data_callback::<LoopbackAppData>()
        .ok_or(vescpkg_rs::PackageStartError::StateTypeMismatch)?
        .register()
}
