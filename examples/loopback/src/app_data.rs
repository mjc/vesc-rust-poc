#![cfg(all(not(test), target_arch = "arm"))]

use vesc_protocol::ble_loopback::handle_loopback_frame;
use vescpkg_rs::{Firmware, PackageAppDataCallback, PackageStart};

struct LoopbackAppData;

impl PackageAppDataCallback for LoopbackAppData {
    fn image_address() -> usize {
        loopback_handle_app_data as *const () as usize
    }
}

/// Register the package-local callback that VESC stores in
/// `third_party/vesc/comm/commands.c:1820-1828`.
#[inline(always)]
pub(crate) fn register(start: &mut PackageStart) -> bool {
    start
        .app_data_callback::<LoopbackAppData>()
        .is_some_and(|callback| callback.register().is_ok())
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// # Safety
///
/// `data` must be null with `len == 0` or point to `len` readable bytes that
/// remain valid for this call.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn loopback_handle_app_data(data: *mut u8, len: u32) {
    let Some(bytes) = (!data.is_null() && len != 0)
        .then(|| unsafe { core::slice::from_raw_parts(data.cast_const(), len as usize) })
    else {
        return;
    };

    let firmware = Firmware::new();
    let app_data = firmware.app_data();
    let now_ms = u64::from(app_data.system_time_ticks().as_ticks()) / 10;
    if let Ok((response, response_len)) = handle_loopback_frame(bytes, now_ms) {
        let _ = response
            .get(..response_len)
            .is_some_and(|response| app_data.send(response).is_ok());
    }
}
