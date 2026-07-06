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

#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{AppDataBindings, ImuApi, ImuBindings, MotorTelemetryApi, MotorTelemetryBindings};

mod custom_config;
mod imu_callback;
mod lifecycle;
mod protocol;
mod startup;
mod state;
mod state_transition;
mod threads;

pub use self::custom_config::register_refloat_custom_config;
pub use self::lifecycle::RefloatPackageLifecycle;
pub use self::state::RefloatPackageState;

#[cfg(test)]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    vescpkg_rs::start_package(start, &[])
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    vescpkg_rs::start_package(
        start,
        &[
            startup::install_refloat_package_state,
            threads::start_refloat_runtime_threads,
            imu_callback::register_refloat_imu_callback,
            startup::register_refloat_app_data_callbacks,
            crate::extensions::register_refloat_loader_extensions,
        ],
    )
}

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
fn handle_refloat_app_data_packet<B: AppDataBindings, M: MotorTelemetryBindings, I: ImuBindings>(
    state: &mut RefloatPackageState,
    lifecycle: &RefloatPackageLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
    packet: vescpkg_rs::ffi::AppDataPacket<'_>,
) -> bool {
    state.handle_packet_with_runtime(lifecycle, telemetry, imu, packet.0)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn loaded_image_base() -> u32 {
    vescpkg_rs::firmware_loaded_function_offset!(refloat_app_data_callback)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn refloat_app_data_handler() -> vescpkg_rs::ffi::AppDataHandler {
    vescpkg_rs::firmware_rebased_thumb_handler!(
        refloat_app_data_callback,
        vescpkg_rs::ffi::AppDataHandler
    )
}

#[cfg(all(not(test), target_arch = "arm"))]
fn refloat_state_from_arg() -> Option<&'static mut RefloatPackageState> {
    // C map: closest visible state compatibility edge is `state_compat` at
    // Refloat v1.2.1 `third_party/refloat/src/state.c:50`; loader ARG storage happens at
    // `third_party/refloat/src/main.c:2432`.
    vescpkg_rs::RealBindings.typed_app_data_arg(loaded_image_base())
}

/// Device app-data behavior invoked by the SDK firmware callback trampoline.
///
/// C map: upstream `on_command_received` starts at `third_party/refloat/src/main.c:2143` and is
/// registered in `third_party/refloat/src/main.c:2457`.
#[cfg(all(not(test), target_arch = "arm"))]
struct RefloatAppData;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::AppDataCallback for RefloatAppData {
    fn handle(packet: vescpkg_rs::ffi::AppDataPacket<'static>) {
        let Some(state) = refloat_state_from_arg() else {
            return;
        };
        let lifecycle = RefloatPackageLifecycle::new(vescpkg_rs::RealBindings);
        let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
        let imu = ImuApi::new(vescpkg_rs::RealImuBindings);
        let _ = handle_refloat_app_data_packet(state, &lifecycle, &telemetry, &imu, packet);
    }
}

vescpkg_rs::firmware_app_data_callback!(refloat_app_data_callback, RefloatAppData);

struct RefloatStop;

impl vescpkg_rs::StopCallback for RefloatStop {
    type State = RefloatPackageState;

    fn stop(context: vescpkg_rs::StopContext<Self::State>) {
        // C map: Refloat v1.2.1 `stop` starts at `third_party/refloat/src/main.c:2399`.
        // Upstream stop cleanup in `third_party/refloat/src/main.c:2398-2412` clears IMU/app-data/custom
        // config callbacks, terminates aux+main threads, destroys LEDs, and frees
        // `Data`. This isolated handler only clears app-data/custom config and frees
        // the narrow Rust app-data allocation if that experimental path was installed.
        #[cfg(not(test))]
        {
            let _ = RefloatPackageLifecycle::new(vescpkg_rs::RealBindings).stop();
        }
        #[cfg(not(all(not(test), target_arch = "arm")))]
        let _ = &context;
        #[cfg(all(not(test), target_arch = "arm"))]
        if let Some(state) = context.state() {
            let bindings = vescpkg_rs::RealBindings;
            threads::request_refloat_runtime_thread_termination(state);
            let _allocation = context.reclaim_allocation(1, &bindings);
        }
    }
}

fn refloat_stop_handler() -> vescpkg_rs::ffi::StopHandler {
    vescpkg_rs::stop_callback::<RefloatStop>
}

#[cfg(test)]
mod test_support;
