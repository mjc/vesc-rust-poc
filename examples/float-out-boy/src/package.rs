//! Float Out Boy package app-data boundary.
//!
//! Float Out Boy `v1.2.1` (`0ef6e99d8701`) anchors:
//! - `third_party/float-out-boy/src/main.c:2143-2295` handles incoming app-data commands.
//! - `third_party/float-out-boy/src/main.c:2334-2403` owns custom config get/set/XML and stop cleanup.
//! - `third_party/float-out-boy/src/main.c:2456-2457` registers custom config and app-data handlers.
//!
//! The Rust state here is still a narrow `FloatOutBoyPackageState`, not upstream's
//! full `Data`; upstream shares `Data *` through `ARG` for app-data, custom
//! config, BMS, threads, and stop cleanup.

#[cfg(any(test, target_arch = "arm"))]
mod callbacks;
mod custom_config;
#[cfg(any(test, target_arch = "arm"))]
mod imu_callback;
mod protocol;
#[cfg(any(test, target_arch = "arm"))]
mod startup;
mod state;
#[cfg(any(test, target_arch = "arm"))]
mod threads;

pub use self::custom_config::FloatOutBoyCustomConfig;
#[cfg(test)]
pub(crate) use self::custom_config::set_float_out_boy_custom_config_for_test;
pub use self::state::FloatOutBoyPackageState;

/// Finish Float Out Boy startup after the required state and thread setup succeeds.
///
/// C map: upstream only returns failure for allocation or either thread spawn at
/// `third_party/float-out-boy/src/main.c:2419-2453`; IMU, config, app-data, and
/// extension registration at `third_party/float-out-boy/src/main.c:2455-2459` are
/// best-effort side effects.
#[cfg(any(test, target_arch = "arm"))]
fn finish_startup(
    required_setup: Result<(), vescpkg_rs::PackageStartError>,
    registrations: impl FnOnce(),
) -> Result<(), vescpkg_rs::PackageStartError> {
    required_setup?;
    registrations();
    Ok(())
}

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn stop(state: &mut FloatOutBoyPackageState) -> vescpkg_rs::PackageStopDisposition {
    stop_with(state, FloatOutBoyPackageState::destroy_internal_leds)
}

#[cfg(any(test, target_arch = "arm"))]
fn stop_with(
    state: &mut FloatOutBoyPackageState,
    destroy_internal_leds: impl FnOnce(&mut FloatOutBoyPackageState) -> bool,
) -> vescpkg_rs::PackageStopDisposition {
    state.stop_data_recorder();
    if destroy_internal_leds(state) {
        vescpkg_rs::PackageStopDisposition::Drop
    } else {
        vescpkg_rs::PackageStopDisposition::Retain
    }
}

#[cfg(all(not(test), not(target_arch = "arm")))]
pub(crate) fn stop(_state: &mut FloatOutBoyPackageState) -> vescpkg_rs::PackageStopDisposition {
    vescpkg_rs::PackageStopDisposition::Drop
}

#[cfg(test)]
pub(crate) fn start(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    startup::install_float_out_boy_package_state(start)
}

#[cfg(all(not(test), target_arch = "arm"))]
#[inline(never)]
pub(crate) fn start(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    // C map: package init allocates Data, refreshes motor config, installs stop
    // state, spawns main/aux threads, registers callbacks, and adds loader
    // extensions at `third_party/float-out-boy/src/main.c:2419-2461`.
    // VESC calls native init at `base + 4 | 1` without relocating data words at
    // `third_party/vesc/lispBM/lispif_c_lib.c:1087-1100`, so keep this as direct calls;
    // a function-pointer table would contain image-relative addresses.
    startup::install_float_out_boy_package_state(start)?;
    threads::start_float_out_boy_runtime_threads(start)?;
    finish_startup(Ok(()), || {
        let _ = imu_callback::register_float_out_boy_imu_callback(start);
        let _ = startup::register_float_out_boy_app_data_callbacks(start);
        let _ = crate::extensions::register_float_out_boy_loader_extensions(start);
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;
