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

mod callbacks;
mod custom_config;
mod imu_callback;
mod protocol;
mod startup;
mod state;
mod threads;
mod time;

pub use self::custom_config::FloatOutBoyCustomConfig;
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
mod tests {
    use super::{
        FloatOutBoyPackageState, finish_startup, stop, stop_with, time::float_out_boy_ticks_elapsed,
    };
    use crate::{
        domain::FloatOutBoyAllDataPayloads,
        package::test_support::default_float_out_boy_config_bytes,
    };
    use vescpkg_rs::prelude::TimestampTicks;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn float_out_boy_ticks_elapsed_matches_timer_older_strict_boundary() {
        let then = TimestampTicks::from_ticks(10_000);

        assert!(!float_out_boy_ticks_elapsed(
            TimestampTicks::from_ticks(20_000),
            then,
            1,
        ));
        assert!(float_out_boy_ticks_elapsed(
            TimestampTicks::from_ticks(20_001),
            then,
            1,
        ));
    }

    #[test]
    fn float_out_boy_ticks_elapsed_matches_timer_older_across_tick_wrap() {
        let then = TimestampTicks::from_ticks(u32::MAX - 5_000);

        assert!(!float_out_boy_ticks_elapsed(
            TimestampTicks::from_ticks(4_999),
            then,
            1,
        ));
        assert!(float_out_boy_ticks_elapsed(
            TimestampTicks::from_ticks(5_000),
            then,
            1,
        ));
    }

    #[test]
    fn startup_ignores_registration_failures_after_required_setup() {
        let registrations = core::cell::Cell::new(0);

        assert!(
            finish_startup(Ok(()), || {
                registrations.set(registrations.get() + 1);
            })
            .is_ok()
        );
        assert_eq!(registrations.get(), 1);
    }

    #[test]
    fn startup_stops_before_registration_when_required_setup_fails() {
        let registrations = core::cell::Cell::new(0);

        assert!(
            finish_startup(
                Err(vescpkg_rs::PackageStartError::LoaderUnavailable),
                || registrations.set(registrations.get() + 1),
            )
            .is_err()
        );
        assert_eq!(registrations.get(), 0);
    }

    #[test]
    fn stop_tears_down_internal_led_runtime() {
        let _firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = default_float_out_boy_config_bytes();
        config[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
        assert!(state.store_serialized_config(&config));
        state.apply_pending_internal_led_refresh();
        assert!(state.internal_leds_operational());

        assert_eq!(stop(&mut state), vescpkg_rs::PackageStopDisposition::Drop,);

        assert!(!state.internal_leds_operational());
    }

    #[test]
    fn stop_retains_state_when_internal_led_dma_cannot_quiesce() {
        let _firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = default_float_out_boy_config_bytes();
        config[227] = crate::lcm::FloatOutBoyLedMode::Internal.id();
        assert!(state.store_serialized_config(&config));
        state.apply_pending_internal_led_refresh();

        let disposition = stop_with(&mut state, |state| {
            state.destroy_internal_leds_with(|_| false)
        });

        assert_eq!(disposition, vescpkg_rs::PackageStopDisposition::Retain,);
        assert!(!state.internal_leds_operational());
    }
}

#[cfg(test)]
pub(crate) mod test_support;
