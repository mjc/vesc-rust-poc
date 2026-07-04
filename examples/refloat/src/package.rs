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

mod callbacks;
mod custom_config;
mod imu_callback;
mod protocol;
mod startup;
mod state;
mod threads;
mod time;

pub(crate) static REFLOAT_RUNTIME_STATE: vescpkg_rs::PackageStateStore<RefloatPackageState> =
    vescpkg_rs::PackageStateStore::new();

pub use self::custom_config::RefloatCustomConfig;
pub use self::state::RefloatPackageState;

/// Finish Refloat startup after the required state and thread setup succeeds.
///
/// C map: upstream only returns failure for allocation or either thread spawn at
/// `third_party/refloat/src/main.c:2419-2453`; IMU, config, app-data, and
/// extension registration at `third_party/refloat/src/main.c:2455-2459` are
/// best-effort side effects.
#[cfg(any(test, target_arch = "arm"))]
fn finish_startup(required_setup: bool, registrations: impl FnOnce()) -> bool {
    required_setup && {
        registrations();
        true
    }
}

#[cfg(test)]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    let _ = start;
    true
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) fn start(start: &mut vescpkg_rs::PackageStart) -> bool {
    // C map: package init allocates Data, refreshes motor config, installs stop
    // state, spawns main/aux threads, registers callbacks, and adds loader
    // extensions at `third_party/refloat/src/main.c:2419-2461`.
    // VESC calls native init at `base + 4 | 1` without relocating data words at
    // `third_party/vesc/lispBM/lispif_c_lib.c:1087-1100`, so keep this as direct calls;
    // a function-pointer table would contain image-relative addresses.
    let required_setup = startup::install_refloat_package_state(start)
        && threads::start_refloat_runtime_threads(start);
    finish_startup(required_setup, || {
        let _ = imu_callback::register_refloat_imu_callback(start);
        let _ = startup::register_refloat_app_data_callbacks(start);
        let _ = crate::extensions::register_refloat_loader_extensions(start);
    })
}

#[cfg(test)]
mod tests {
    use super::{finish_startup, time::refloat_ticks_elapsed};
    use vescpkg_rs::prelude::TimestampTicks;

    #[test]
    fn refloat_ticks_elapsed_matches_timer_older_strict_boundary() {
        let then = TimestampTicks::from_ticks(10_000);

        assert!(!refloat_ticks_elapsed(
            TimestampTicks::from_ticks(20_000),
            then,
            1,
        ));
        assert!(refloat_ticks_elapsed(
            TimestampTicks::from_ticks(20_001),
            then,
            1,
        ));
    }

    #[test]
    fn startup_ignores_registration_failures_after_required_setup() {
        let registrations = core::cell::Cell::new(0);

        assert!(finish_startup(true, || {
            registrations.set(registrations.get() + 1);
        }));
        assert_eq!(registrations.get(), 1);
    }

    #[test]
    fn startup_stops_before_registration_when_required_setup_fails() {
        assert!(!finish_startup(false, || panic!("registration")));
    }
}

#[cfg(test)]
pub(crate) mod test_support;
