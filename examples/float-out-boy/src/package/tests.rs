use super::{FloatOutBoyPackageState, finish_startup, stop, stop_with};
use crate::package::test_support::default_float_out_boy_config_bytes;
use vescpkg_rs::prelude::TimestampTicks;
use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::timer_older_whole_seconds as float_out_boy_ticks_elapsed;

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
    let mut state = FloatOutBoyPackageState::default();
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
    let mut state = FloatOutBoyPackageState::default();
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
