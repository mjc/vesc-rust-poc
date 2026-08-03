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
    AngleDegrees, AngularVelocity, Current, MotorCurrent, Rpm, SampleRate, SignedRatio,
    TimestampTicks, VescSeconds,
};

#[test]
fn input_tilt_reversal_respects_float_out_boy_ramp_down() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));
    let angle_limit = AngleDegrees::from_degrees(10.0);
    let speed = AngularVelocity::from_degrees_per_second(25.0);
    let sample_rate = SampleRate::from_hertz(500.0);

    let rising = remote_control.update_input_tilt(angle_limit, speed, sample_rate, false);
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(-1.0),
    ));
    let falling = remote_control.update_input_tilt(angle_limit, speed, sample_rate, false);

    assert!(falling < rising);
    assert!((rising - falling) <= AngleDegrees::from_degrees(25.0 / 500.0));
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
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
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
