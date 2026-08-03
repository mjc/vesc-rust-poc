use super::*;
use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn motor_control_sets_zero_once_while_disabled_like_float_out_boy() {
    let motor = FirmwareTest::new();
    let bindings = motor.motor();
    let mut control = FloatOutBoyMotorControl::new();

    assert!(control.apply(
        bindings,
        FloatOutBoyRunState::Disabled,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(motor.current_command_count(), 1);
    assert_f32_eq!(motor.commanded_current().current().as_amps(), 0.0);

    assert!(!control.apply(
        bindings,
        FloatOutBoyRunState::Disabled,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_eq!(motor.current_command_count(), 1);
}

#[test]
fn motor_control_applies_ready_parking_brake_like_float_out_boy() {
    let motor = FirmwareTest::new();
    let bindings = motor.motor();
    let mut control = FloatOutBoyMotorControl::new();

    assert!(control.apply(
        bindings,
        FloatOutBoyRunState::Ready,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));

    // Upstream `motor_control_apply` resets timeout at
    // `third_party/float-out-boy/src/motor_control.c:92-93`, activates default
    // `PARKING_BRAKE_IDLE` at `third_party/float-out-boy/src/motor_control.c:66-70`,
    // and applies duty zero while stopped at
    // `third_party/float-out-boy/src/motor_control.c:112-114`.
    assert_eq!(motor.keep_alive_count(), 1);
    assert_eq!(motor.duty_command_count(), 1);
    assert_f32_eq!(motor.commanded_duty().ratio().as_ratio(), 0.0);
    assert_eq!(motor.current_command_count(), 0);
    assert_eq!(motor.brake_current_command_count(), 0);
}

#[test]
fn motor_control_seeds_idle_brake_timer_instead_of_reproducing_refloat_uptime_bug() {
    let motor = FirmwareTest::new();
    let bindings = motor.motor();
    let mut control = FloatOutBoyMotorControl::new();

    assert!(control.apply(
        bindings,
        FloatOutBoyRunState::Ready,
        Rpm::ZERO,
        TimestampTicks::from_ticks(20_000),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));

    // Refloat initializes `brake_timer` to zero at
    // `third_party/float-out-boy/src/motor_control.c:29`, so first activation
    // after one second of controller uptime can release immediately. Rust
    // starts the same one-second hold when the parking brake becomes active.
    assert_eq!(motor.keep_alive_count(), 1);
    assert_eq!(motor.duty_command_count(), 1);
    assert_eq!(motor.current_command_count(), 0);
    assert_eq!(motor.brake_current_command_count(), 0);
}

#[test]
fn motor_control_modulates_requested_current_for_vibration_like_float_out_boy() {
    let motor = FirmwareTest::new();
    let mut control = FloatOutBoyMotorControl::new();
    control.play_tone(
        AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(70.0)),
        MotorCurrent::new(Current::from_amps(2.0)),
        SampleRate::from_hertz(832.0),
    );

    for _ in 0..4 {
        control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
        assert!(control.apply(
            motor.motor(),
            FloatOutBoyRunState::Running,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_f32_eq!(motor.commanded_current().current().as_amps(), 3.0);
    }

    control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
    assert!(control.apply(
        motor.motor(),
        FloatOutBoyRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_f32_eq!(motor.commanded_current().current().as_amps(), 7.0);

    control.stop_tone();
    control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
    assert!(control.apply(
        motor.motor(),
        FloatOutBoyRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::from_amps(50.0)),
    ));
    assert_f32_eq!(motor.commanded_current().current().as_amps(), 5.0);
}

#[test]
fn active_tone_saturates_an_empty_counter_instead_of_panicking() {
    let motor = FirmwareTest::new();
    let mut control = FloatOutBoyMotorControl {
        tone_ticks: 1,
        tone_counter: 0,
        ..FloatOutBoyMotorControl::new()
    };

    assert!(control.apply(
        motor.motor(),
        FloatOutBoyRunState::Running,
        Rpm::ZERO,
        TimestampTicks::from_ticks(0),
        FloatOutBoyParkingBrakeMode::Idle,
        MotorCurrent::new(Current::ZERO),
    ));
    assert_eq!(control.tone_counter, 1);
}
