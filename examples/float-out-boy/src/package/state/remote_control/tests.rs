use super::{PhysicalRemoteInput, RemoteControlState};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand,
    FloatOutBoyRealtimeRemoteInput,
};
use crate::motor_torque::REFLOAT_COMPAT_TORQUE_CONSTANT;
use crate::package::state::FloatOutBoyPackageState;
use vescpkg_rs::prelude::{
    AngleDegrees, Ratio, SampleRate, SignedRatio, Speed, TimestampTicks, VescSeconds,
};

fn remote_packet(value: i8) -> [u8; 3] {
    [
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::Remote.id(),
        value.to_ne_bytes()[0],
    ]
}

fn physical_input(value: f32) -> SignedRatio {
    SignedRatio::clamped(value)
}

#[test]
fn input_tilt_first_update_matches_refloat_smooth_setpoint() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));

    let setpoint = remote_control.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        VescSeconds::from_seconds(0.2),
        SampleRate::from_hertz(500.0),
        false,
    );

    assert!((setpoint.as_degrees() - 0.000_178_73).abs() < 0.000_000_1);
}

#[test]
fn input_tilt_uses_serialized_filter_time_constant() {
    let input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(1.0));
    let mut faster = RemoteControlState::default();
    let mut slower = RemoteControlState::default();
    faster.set_input(input);
    slower.set_input(input);

    let faster_setpoint = faster.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        VescSeconds::from_seconds(0.1),
        SampleRate::from_hertz(500.0),
        false,
    );
    let slower_setpoint = slower.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        VescSeconds::from_seconds(0.4),
        SampleRate::from_hertz(500.0),
        false,
    );

    assert!(faster_setpoint > slower_setpoint);
    assert!(slower_setpoint > AngleDegrees::ZERO);
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
        remote_control.update_input_tilt(
            angle_limit,
            VescSeconds::from_seconds(0.2),
            sample_rate,
            false,
        );
    }
    let rising = remote_control.tilt_setpoint.value();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(-1.0),
    ));
    for _ in 0..500 {
        remote_control.update_input_tilt(
            angle_limit,
            VescSeconds::from_seconds(0.2),
            sample_rate,
            false,
        );
    }
    let reversed = remote_control.tilt_setpoint.value();

    assert!(rising.is_positive());
    assert!(reversed.is_negative());
}

#[test]
fn input_tilt_stays_within_five_percent_over_equal_time_at_different_cadences() {
    let angle_limit = AngleDegrees::from_degrees(10.0);
    let mut nominal = RemoteControlState::default();
    let mut delayed = RemoteControlState::default();
    let input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(1.0));
    nominal.set_input(input);
    delayed.set_input(input);

    for _ in 0..50 {
        nominal.update_input_tilt_elapsed(
            angle_limit,
            VescSeconds::from_seconds(0.2),
            VescSeconds::from_seconds(0.002),
            false,
        );
    }
    for _ in 0..25 {
        delayed.update_input_tilt_elapsed(
            angle_limit,
            VescSeconds::from_seconds(0.2),
            VescSeconds::from_seconds(0.004),
            false,
        );
    }

    let difference = (delayed.tilt_setpoint.value() - nominal.tilt_setpoint.value()).abs();
    assert!(
        difference.as_degrees() / nominal.tilt_setpoint.value().as_degrees() < 0.05,
        "nominal={:?} delayed={:?}",
        nominal.tilt_setpoint.value(),
        delayed.tilt_setpoint.value(),
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
        upright.update_input_tilt_elapsed(
            AngleDegrees::from_degrees(10.0),
            VescSeconds::from_seconds(0.2),
            elapsed,
            false,
        );
        darkride.update_input_tilt_elapsed(
            AngleDegrees::from_degrees(10.0),
            VescSeconds::from_seconds(0.2),
            elapsed,
            true,
        );
    }

    assert_eq!(
        darkride.tilt_setpoint.value(),
        -upright.tilt_setpoint.value()
    );
}

#[test]
fn runtime_reset_clears_remote_tilt_motion() {
    let mut remote_control = RemoteControlState::default();
    remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
        SignedRatio::from_ratio_const(1.0),
    ));
    remote_control.update_input_tilt(
        AngleDegrees::from_degrees(10.0),
        VescSeconds::from_seconds(0.2),
        SampleRate::from_hertz(500.0),
        false,
    );

    remote_control.reset_runtime_vars();

    assert_eq!(remote_control.tilt_setpoint.value(), AngleDegrees::ZERO);
}

#[test]
fn cutoff_remote_config_decodes_zero_move_limit_and_ten_second_grace() {
    let state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

    assert_eq!(
        state.serialized_config.remote().max_move_speed(),
        Speed::ZERO
    );
    assert_eq!(
        state.serialized_config.remote().grace_period(),
        VescSeconds::from_seconds(10.0)
    );
}

#[test]
fn unified_remote_command_requires_a_value_but_ignores_trailing_bytes() {
    let mut remote_control = RemoteControlState::default();
    let now = TimestampTicks::from_ticks(30_001);

    assert!(!remote_control.handle_packet(
        now,
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Remote.id()
        ],
    ));
    assert!(remote_control.handle_packet(
        now,
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::Remote.id(),
            127,
            99
        ],
    ));
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(1.0)
    );
}

#[test]
fn unified_remote_command_normalizes_signed_endpoints_and_ignores_minus_128() {
    let now = TimestampTicks::from_ticks(30_001);
    let mut remote_control = RemoteControlState::default();

    assert!(remote_control.handle_packet(
        now,
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &remote_packet(-127),
    ));
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(-1.0)
    );
    assert!(remote_control.handle_packet(
        TimestampTicks::from_ticks(35_000),
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &remote_packet(i8::MIN),
    ));
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(-1.0)
    );
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(1.0)),
        now: TimestampTicks::from_ticks(35_002),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::ZERO,
        move_grace: VescSeconds::from_seconds(0.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(1.0)
    );
}

#[test]
fn command_move_requires_strict_two_second_disengage_grace() {
    let mut remote_control = RemoteControlState::default();
    let disengage = TimestampTicks::from_ticks(1_000);

    assert!(remote_control.handle_packet(
        TimestampTicks::from_ticks(21_000),
        disengage,
        Speed::from_kilometers_per_hour(8.0),
        &remote_packet(127),
    ));
    assert_eq!(remote_control.move_target, None);
    assert!(remote_control.handle_packet(
        TimestampTicks::from_ticks(21_001),
        disengage,
        Speed::from_kilometers_per_hour(8.0),
        &remote_packet(127),
    ));
    assert_eq!(
        remote_control.move_target,
        Some(Speed::from_kilometers_per_hour(8.0))
    );
}

#[test]
fn command_move_uses_five_kph_default_when_configured_limit_is_zero() {
    let mut remote_control = RemoteControlState::default();
    assert!(remote_control.handle_packet(
        TimestampTicks::from_ticks(20_001),
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &remote_packet(127),
    ));
    assert_eq!(
        remote_control.move_target,
        Some(Speed::from_kilometers_per_hour(5.0))
    );
}

#[test]
fn command_input_owns_remote_through_the_exact_half_second_boundary() {
    let mut remote_control = RemoteControlState::default();
    let command_time = TimestampTicks::from_ticks(30_001);
    assert!(remote_control.handle_packet(
        command_time,
        TimestampTicks::from_ticks(0),
        Speed::ZERO,
        &remote_packet(127),
    ));

    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(-1.0)),
        now: TimestampTicks::from_ticks(35_001),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::ZERO,
        move_grace: VescSeconds::from_seconds(0.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(1.0)
    );
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(-1.0)),
        now: TimestampTicks::from_ticks(35_002),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::ZERO,
        move_grace: VescSeconds::from_seconds(0.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(-1.0)
    );
}

#[test]
fn physical_remote_applies_deadband_and_tilt_inversion_but_not_move_inversion() {
    let mut remote_control = RemoteControlState::default();
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(0.6)),
        now: TimestampTicks::from_ticks(30_001),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.2),
        inverted: true,
        maximum_move_speed: Speed::from_kilometers_per_hour(8.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });

    assert!((remote_control.input().ratio().as_ratio() + 0.5).abs() < 0.000_001);
    assert!(
        (remote_control
            .move_target
            .expect("physical move target")
            .as_kilometers_per_hour()
            - 4.0)
            .abs()
            < 0.000_001
    );
}

#[test]
fn physical_move_uses_the_configured_strict_disengage_grace_boundary() {
    let mut remote_control = RemoteControlState::default();
    let disengage = TimestampTicks::from_ticks(1_000);
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(1.0)),
        now: TimestampTicks::from_ticks(101_000),
        disengage_epoch: disengage,
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(7.0),
        move_grace: VescSeconds::from_seconds(10.0),
    });
    assert_eq!(remote_control.move_target, None);

    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(1.0)),
        now: TimestampTicks::from_ticks(101_001),
        disengage_epoch: disengage,
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(7.0),
        move_grace: VescSeconds::from_seconds(10.0),
    });
    assert_eq!(
        remote_control.move_target,
        Some(Speed::from_kilometers_per_hour(7.0))
    );
}

#[test]
fn command_priority_timeout_uses_wrapping_system_ticks() {
    let mut remote_control = RemoteControlState::default();
    let command_time = TimestampTicks::from_ticks(u32::MAX - 1_000);
    assert!(remote_control.handle_packet(
        command_time,
        TimestampTicks::from_ticks(u32::MAX.wrapping_sub(30_000)),
        Speed::ZERO,
        &remote_packet(127),
    ));
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(-1.0)),
        now: TimestampTicks::from_ticks(3_999),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::ZERO,
        move_grace: VescSeconds::from_seconds(0.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(1.0)
    );
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(-1.0)),
        now: TimestampTicks::from_ticks(4_000),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::ZERO,
        move_grace: VescSeconds::from_seconds(0.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(-1.0)
    );
}

#[test]
fn physical_remote_neutral_holds_zero_speed_through_one_second_then_releases() {
    let mut remote_control = RemoteControlState::default();
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(1.0)),
        now: TimestampTicks::from_ticks(30_001),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(5.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(0.0)),
        now: TimestampTicks::from_ticks(40_001),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(5.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });
    assert_eq!(remote_control.move_target, Some(Speed::ZERO));
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(0.0)),
        now: TimestampTicks::from_ticks(40_002),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(5.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });
    assert_eq!(remote_control.move_target, None);
}

#[test]
fn stale_physical_remote_clears_tilt_and_move_immediately() {
    let mut remote_control = RemoteControlState::default();
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: Some(physical_input(1.0)),
        now: TimestampTicks::from_ticks(30_001),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(5.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });
    remote_control.refresh_physical_input(PhysicalRemoteInput {
        raw: None,
        now: TimestampTicks::from_ticks(30_002),
        disengage_epoch: TimestampTicks::from_ticks(0),
        deadband: Ratio::from_ratio_const(0.0),
        inverted: false,
        maximum_move_speed: Speed::from_kilometers_per_hour(5.0),
        move_grace: VescSeconds::from_seconds(2.0),
    });
    assert_eq!(
        remote_control.input().ratio(),
        SignedRatio::from_ratio_const(0.0)
    );
    assert_eq!(remote_control.move_target, None);
}

#[test]
fn ready_move_pi_uses_elapsed_time_and_clamps_to_ten_newton_metres() {
    let mut remote_control = RemoteControlState {
        move_target: Some(Speed::from_kilometers_per_hour(5.0)),
        ..RemoteControlState::default()
    };
    let current = remote_control
        .request_ready_current(
            Speed::ZERO,
            VescSeconds::from_seconds(0.1),
            REFLOAT_COMPAT_TORQUE_CONSTANT,
        )
        .expect("active move target");
    assert!(
        (current.current().as_amps() - 6.5 / 0.6075).abs() < 0.0001,
        "{current:?}"
    );

    let clamped = remote_control
        .request_ready_current(
            Speed::from_kilometers_per_hour(-100.0),
            VescSeconds::from_seconds(1.0),
            REFLOAT_COMPAT_TORQUE_CONSTANT,
        )
        .expect("active move target");
    assert!((clamped.current().as_amps() - 10.0 / 0.6075).abs() < 0.0001);
}

#[test]
fn inactive_move_resets_integral_and_requests_no_current() {
    let mut remote_control = RemoteControlState {
        move_integral: REFLOAT_COMPAT_TORQUE_CONSTANT
            .torque_from_current(vescpkg_rs::Current::from_amps(1.0)),
        ..RemoteControlState::default()
    };
    assert_eq!(
        remote_control.request_ready_current(
            Speed::ZERO,
            VescSeconds::from_seconds(0.1),
            REFLOAT_COMPAT_TORQUE_CONSTANT,
        ),
        None
    );
    assert_eq!(remote_control.move_integral, vescpkg_rs::MotorTorque::ZERO);
}
