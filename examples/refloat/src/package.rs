//! Refloat package app-data boundary.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) anchors:
//! - `third_party/refloat/src/main.c:2143-2295` handles incoming app-data commands.
//! - `third_party/refloat/src/main.c:2334-2403` owns custom config get/set/XML and stop cleanup.
//! - `third_party/refloat/src/main.c:2456-2457` registers custom config and app-data handlers.
//!
//! The Rust state here is still a narrow `RefloatPackageState`, not upstream's
//! full `Data`; upstream shares `Data *` through `ARG` for app-data, custom
//! config, BMS, threads, and stop cleanup.

#![cfg_attr(all(not(test), target_arch = "arm"), allow(dead_code))]

#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::ffi;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{AppDataBindings, ImuApi, ImuBindings, MotorTelemetryApi, MotorTelemetryBindings};

mod balance_filter;
mod custom_config;
mod imu_callback;
mod lifecycle;
mod protocol;
mod startup;
mod state;

pub use self::custom_config::register_refloat_custom_config;
#[cfg(all(not(test), target_arch = "arm"))]
pub use self::imu_callback::register_refloat_imu_callback;
pub use self::lifecycle::RefloatPackageLifecycle;
#[cfg(all(not(test), target_arch = "arm"))]
pub use self::startup::{
    install_refloat_app_data, install_refloat_package_state, register_refloat_app_data_callbacks,
};
pub use self::state::RefloatPackageState;

fn refloat_ticks_elapsed(now: u32, then: u32, seconds: u32) -> bool {
    now.wrapping_sub(then) >= seconds.saturating_mul(10_000)
}

fn refloat_ticks_elapsed_ms(now: u32, then: u32, milliseconds: u32) -> bool {
    now.wrapping_sub(then) > milliseconds.saturating_mul(10)
}

fn refloat_ticks_elapsed_f32(now: u32, then: u32, seconds: f32) -> bool {
    now.wrapping_sub(then) > (seconds * 10_000.0) as u32
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn handle_refloat_app_data_packet<
    B: AppDataBindings,
    M: MotorTelemetryBindings,
    I: ImuBindings,
>(
    state: &mut RefloatPackageState,
    lifecycle: &RefloatPackageLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
    data: *mut u8,
    len: u32,
) -> bool {
    let Some(data) = core::ptr::NonNull::new(data) else {
        return false;
    };
    let Ok(len) = usize::try_from(len) else {
        return false;
    };
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr().cast_const(), len) };
    state.handle_packet_with_runtime(lifecycle, telemetry, imu, bytes)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn loaded_image_base() -> u32 {
    let loaded_handler: usize;
    unsafe {
        core::arch::asm!(
            "adr {loaded_handler}, {handler}",
            loaded_handler = out(reg) loaded_handler,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
    }
    let loaded_handler = loaded_handler & !1;
    let image_handler = refloat_handle_app_data as *const () as usize & !1;
    (loaded_handler - image_handler) as u32
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_app_data_handler() -> ffi::AppDataHandler {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {handler}",
            address = out(reg) address,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
        core::mem::transmute::<usize, ffi::AppDataHandler>(address | 1)
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn refloat_state_from_arg() -> Option<&'static mut RefloatPackageState> {
    // C map: closest visible state compatibility edge is `state_compat` at
    // Refloat v1.2.1 `third_party/refloat/src/state.c:50`; loader ARG storage happens at
    // `third_party/refloat/src/main.c:2432`.
    let state = vescpkg_rs::RealBindings
        .app_data_arg(loaded_image_base())?
        .cast::<RefloatPackageState>();
    unsafe { state.as_ptr().as_mut() }
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// C map: upstream `on_command_received` starts at `third_party/refloat/src/main.c:2143` and is
/// registered in `third_party/refloat/src/main.c:2457`.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn refloat_handle_app_data(data: *mut u8, len: u32) {
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    let lifecycle = RefloatPackageLifecycle::new(vescpkg_rs::RealBindings);
    let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
    let imu = ImuApi::new(vescpkg_rs::RealImuBindings);
    let _ =
        unsafe { handle_refloat_app_data_packet(state, &lifecycle, &telemetry, &imu, data, len) };
}

unsafe extern "C" fn stop_refloat_app_data(_arg: *mut core::ffi::c_void) {
    // C map: Refloat v1.2.1 `stop` starts at `third_party/refloat/src/main.c:2399`.
    // Upstream stop cleanup in `third_party/refloat/src/main.c:2398-2412` clears IMU/app-data/custom
    // config callbacks, terminates aux+main threads, destroys LEDs, and frees
    // `Data`. This isolated handler only clears app-data/custom config and frees
    // the narrow Rust app-data allocation if that experimental path was installed.
    #[cfg(not(test))]
    {
        let _ = RefloatPackageLifecycle::new(vescpkg_rs::RealBindings).stop();
    }
    #[cfg(all(not(test), target_arch = "arm"))]
    if let Some(ptr) = core::ptr::NonNull::new(_arg.cast::<RefloatPackageState>()) {
        let bindings = vescpkg_rs::RealBindings;
        crate::runtime::request_refloat_runtime_thread_termination(unsafe { ptr.as_ref() });
        let _allocation =
            unsafe { vescpkg_rs::FirmwareAllocation::from_raw_parts(ptr, 1, &bindings) };
    }
}

#[cfg(test)]
mod test_support;
