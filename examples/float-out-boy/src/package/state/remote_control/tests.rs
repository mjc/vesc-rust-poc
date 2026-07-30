use super::{RemoteControlState, RemoteCurrentTarget, RemoteMove, handle_packet};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
    FloatOutBoyRealtimeRemoteInput, FloatOutBoyRunState,
};
use crate::package::state::FloatOutBoyPackageState;
use crate::package::test_support::{
    FloatOutBoyConfigTestBytes, editable_config_from_bytes,
    sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::prelude::{
    AngleDegrees, Current, MotorCurrent, Rpm, SampleRate, SignedRatio, TimestampTicks, VescSeconds,
};

impl RemoteCurrentTarget {
    const fn deciamps(self) -> i16 {
        self.0
    }
}

impl RemoteControlState {
    const fn target_deciamps_for_test(self) -> i16 {
        self.target.deciamps()
    }

    const fn remaining_steps_for_test(self) -> u16 {
        self.steps
    }
}

#[test]
fn input_tilt_first_update_matches_refloat_smooth_setpoint() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));

    let setpoint = remote_control.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        SampleRate::from_hertz(500.0),
        false,
    );

    assert!((setpoint.as_degrees() - 0.000_178_73).abs() < 0.000_000_1);
}

#[test]
fn input_tilt_reversal_eventually_crosses_zero() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));
    let angle_limit = AngleDegrees::from_degrees(10.0);
    let sample_rate = SampleRate::from_hertz(500.0);
    for _ in 0..100 {
        remote_control.update_input_tilt(angle_limit, sample_rate, false);
    }
    let rising = remote_control.input_tilt.value();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(-1.0),
    ));
    for _ in 0..500 {
        remote_control.update_input_tilt(angle_limit, sample_rate, false);
    }
    let reversed = remote_control.input_tilt.value();

    assert!(rising.is_positive());
    assert!(reversed.is_negative());
}

#[test]
fn input_tilt_stays_within_five_percent_over_equal_time_at_different_cadences() {
    let angle_limit = AngleDegrees::from_degrees(10.0);
    let input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(1.0));
    let mut nominal = RemoteControlState::default();
    let mut delayed = RemoteControlState::default();
    nominal.set_input(input);
    delayed.set_input(input);

    for _ in 0..50 {
        nominal.update_input_tilt_elapsed(angle_limit, VescSeconds::from_seconds(0.002), false);
    }
    for _ in 0..25 {
        delayed.update_input_tilt_elapsed(angle_limit, VescSeconds::from_seconds(0.004), false);
    }

    let difference = (delayed.input_tilt.value() - nominal.input_tilt.value()).abs();
    assert!(
        difference.as_degrees() / nominal.input_tilt.value().as_degrees() < 0.05,
        "nominal={:?} delayed={:?}",
        nominal.input_tilt.value(),
        delayed.input_tilt.value(),
    );
}

#[test]
fn darkride_mirrors_the_remote_tilt_setpoint() {
    let mut upright = RemoteControlState::default();
    let mut darkride = RemoteControlState::default();
    let input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.5));
    upright.set_input(input);
    darkride.set_input(input);
    let elapsed = VescSeconds::from_seconds(0.002);
    for _ in 0..100 {
        upright.update_input_tilt_elapsed(AngleDegrees::from_degrees(10.0), elapsed, false);
        darkride.update_input_tilt_elapsed(AngleDegrees::from_degrees(10.0), elapsed, true);
    }

    assert_eq!(darkride.input_tilt.value(), -upright.input_tilt.value());
}

#[test]
fn runtime_reset_clears_remote_tilt_motion() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));
    remote_control.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        SampleRate::from_hertz(500.0),
        false,
    );

    remote_control.reset_runtime_vars();

    assert_eq!(remote_control.input_tilt.value(), AngleDegrees::ZERO);
}

#[test]
fn remote_throttle_requests_idle_current_like_float_out_boy_do_rc_move() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(0.5),
    ));
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    ));
    let mut config = *state.serialized_config();
    config.edit_float_out_boy_config(|config| {
        assert!(
            config.set_remote_throttle_current_max(MotorCurrent::new(Current::from_amps(10.0,)))
        );
    });
    config.edit_float_out_boy_config(|config| {
        assert!(config.set_remote_throttle_grace_period(VescSeconds::ZERO));
    });
    let config = editable_config_from_bytes(&config);
    let remote_throttle = config.remote_throttle();

    let requested_current = remote_control
        .request_remote_throttle_current(
            remote_throttle,
            TimestampTicks::from_ticks(1),
            TimestampTicks::from_ticks(0),
        )
        .expect("remote throttle should request current");

    // Upstream `do_rc_move(d)` uses default inverted throttle and filters
    // `rc_current = old * 0.95 + target * 0.05` before requesting current
    // at `third_party/float-out-boy/src/main.c:291-298`; 10A max with 50% input requests -0.25A.
    assert_f32_eq!(requested_current.current().as_amps(), -0.25);
}

#[test]
fn rc_move_command_checksum_failure_becomes_zero_current_step_like_float_out_boy() {
    // C map: `cmd_rc_move` compares `sum != time + current` as ints, then
    // sets `current = 0` at `third_party/float-out-boy/src/main.c:1735-1741`.
    assert_eq!(
        RemoteMove::from_float_out_boy_command(1, 1, 255, 0),
        RemoteMove {
            target: RemoteCurrentTarget::ZERO,
            duration_steps: 1
        }
    );
}

#[test]
fn rc_move_command_steps_idle_current_like_float_out_boy_do_rc_move() {
    let mut remote_control = RemoteControlState::default();
    remote_control.queue_move(RemoteMove::from_float_out_boy_command(1, 40, 2, 42));

    let requested_current = remote_control
        .request_active_move_current(Rpm::ZERO)
        .expect("active RC move should request current");

    // Upstream `cmd_rc_move` sets `rc_steps = time * 100` and target
    // current/10 at `third_party/float-out-boy/src/main.c:1747-1756`; `do_rc_move` filters the first
    // READY tick by 5% at `third_party/float-out-boy/src/main.c:276-286`.
    assert!((requested_current.current().as_amps() - 0.2).abs() < 0.0001);
}

#[test]
fn active_move_saturates_its_tick_counter_instead_of_panicking() {
    let mut remote_control = RemoteControlState {
        steps: 1,
        counter: u16::MAX,
        ..RemoteControlState::default()
    };

    assert!(
        remote_control
            .request_active_move_current(Rpm::ZERO)
            .is_some()
    );
    assert_eq!(remote_control.counter, u16::MAX);
}

#[test]
fn rc_move_rejects_a_trailing_payload_byte_without_queueing_current() {
    let mut remote_control = RemoteControlState::default();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
    );
    let packet = [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        FloatOutBoyAppDataCommand::RcMove.id(),
        1,
        40,
        2,
        42,
        0,
    ];

    assert!(!handle_packet(payloads, &mut remote_control, &packet));
    assert_eq!(remote_control.request_active_move_current(Rpm::ZERO), None);
}

#[test]
fn rc_move_halves_large_target_after_500_steps_like_float_out_boy_do_rc_move() {
    let mut remote_control = RemoteControlState::default();
    remote_control.queue_move(RemoteMove::from_float_out_boy_command(1, 60, 6, 66));

    for _ in 0..500 {
        assert!(
            remote_control
                .request_active_move_current(Rpm::ZERO)
                .is_some()
        );
    }

    // Upstream `do_rc_move(d)` halves targets above 2A when `rc_counter`
    // reaches 500 at `third_party/float-out-boy/src/main.c:281-284`, after decrementing steps.
    assert_eq!(remote_control.target_deciamps_for_test(), 30);
    assert_eq!(remote_control.remaining_steps_for_test(), 100);
}
