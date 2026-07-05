use super::types::{
    RefloatBalanceLoopConfig, RefloatBalanceLoopInput, RefloatBalanceLoopOutput,
    RefloatBalanceLoopState,
};
use crate::domain::{
    RefloatDarkRideState, RefloatMode, RefloatRealtimeBalancePitch, RefloatRealtimeRuntimeSetpoint,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, AngularVelocity, Current, ElectricalSpeed, ImuPitch, ImuRoll,
    MotorCurrent, Rpm, SampleRate,
};

/// Board setpoint error used by Refloat PID P/I terms.
///
/// Source map: upstream computes `setpoint - imu->balance_pitch` at
/// `third_party/refloat/src/pid.c:40`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct RefloatSetpointError(AngleDegrees);

impl RefloatSetpointError {
    #[inline(always)]
    const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    #[inline(always)]
    const fn angle(self) -> AngleDegrees {
        self.0
    }
}

/// Refloat pitch-rate value after roll/yaw mixing and darkride sign handling.
///
/// Source map: upstream computes `imu->pitch_rate` at
/// `third_party/refloat/src/imu.c:46-53`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct RefloatPitchRate(AngularVelocity);

impl RefloatPitchRate {
    #[inline(always)]
    const fn new(rate: AngularVelocity) -> Self {
        Self(rate)
    }

    #[inline(always)]
    const fn rate(self) -> AngularVelocity {
        self.0
    }
}

/// Positive magnitude used to clamp Refloat RUNNING balance current.
///
/// Source map: upstream chooses a scalar `current_limit` at
/// `third_party/refloat/src/main.c:932-940`, then clamps with
/// `fabsf(new_current) > current_limit` at `third_party/refloat/src/main.c:941-942`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
struct RefloatCurrentLimitMagnitude(Current);

impl RefloatCurrentLimitMagnitude {
    #[inline(always)]
    fn from_motor_current(limit: MotorCurrent) -> Self {
        Self(Current::from_amps(limit.current().as_amps().abs()))
    }

    #[inline(always)]
    fn as_amps(self) -> f32 {
        self.0.as_amps()
    }
}

/// Booster proportional angle supplied to upstream `booster_update`.
///
/// Source map: upstream computes this as
/// `d->setpoint - d->brake_tilt.setpoint - d->imu.pitch` at
/// `third_party/refloat/src/main.c:921`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct RefloatBoosterProportional(AngleDegrees);

impl RefloatBoosterProportional {
    #[inline(always)]
    const fn new(angle: AngleDegrees) -> Self {
        Self(angle)
    }

    #[inline(always)]
    const fn angle(self) -> AngleDegrees {
        self.0
    }
}

#[inline(always)]
fn refloat_motor_current(amps: f32) -> MotorCurrent {
    MotorCurrent::new(Current::from_amps(amps))
}

#[inline(always)]
fn refloat_angle_degrees_from_radians(angle: AngleRadians) -> AngleDegrees {
    AngleDegrees::from_degrees(angle.as_radians() * 180.0 / core::f32::consts::PI)
}

#[inline(always)]
fn refloat_raw_pitch_degrees(pitch: ImuPitch) -> AngleDegrees {
    refloat_angle_degrees_from_radians(pitch.angle())
}

#[inline(always)]
fn refloat_balance_pitch_degrees(balance_pitch: RefloatRealtimeBalancePitch) -> AngleDegrees {
    refloat_angle_degrees_from_radians(balance_pitch.angle())
}

#[inline(always)]
fn refloat_electrical_rpm(speed: ElectricalSpeed) -> Rpm {
    speed.rpm()
}

#[inline(always)]
fn refloat_motor_braking(current: MotorCurrent) -> bool {
    current.current().as_amps() < 0.0
}

/// Source map: upstream computes PID error at
/// `third_party/refloat/src/pid.c:40`.
#[inline(always)]
pub(crate) fn refloat_setpoint_error(
    setpoint: RefloatRealtimeRuntimeSetpoint,
    balance_pitch: RefloatRealtimeBalancePitch,
) -> RefloatSetpointError {
    RefloatSetpointError::new(setpoint.angle() - refloat_balance_pitch_degrees(balance_pitch))
}

/// Source map: upstream computes roll-corrected pitch rate at
/// `third_party/refloat/src/imu.c:46-53`.
#[inline(always)]
pub(crate) fn refloat_pitch_rate(
    roll: ImuRoll,
    gyro_pitch: AngularVelocity,
    gyro_yaw: AngularVelocity,
    darkride: RefloatDarkRideState,
) -> RefloatPitchRate {
    let roll_radians = roll.angle().as_radians();
    let sin_roll = libm::sinf(roll_radians);
    let cos_roll = libm::cosf(roll_radians);
    let pitch_rate = gyro_pitch * (cos_roll * cos_roll) + gyro_yaw * (sin_roll * cos_roll);

    RefloatPitchRate::new(if matches!(darkride, RefloatDarkRideState::Active) {
        -pitch_rate
    } else {
        pitch_rate
    })
}

/// Source map: upstream subtracts brake tilt before `booster_update` at
/// `third_party/refloat/src/main.c:921-922`.
#[inline(always)]
pub(crate) fn refloat_booster_proportional(
    setpoint: RefloatRealtimeRuntimeSetpoint,
    brake_tilt: RefloatRealtimeRuntimeSetpoint,
    raw_pitch: ImuPitch,
) -> RefloatBoosterProportional {
    RefloatBoosterProportional::new(
        setpoint.angle() - brake_tilt.angle() - refloat_raw_pitch_degrees(raw_pitch),
    )
}

/// Source map: upstream clamps PID I at
/// `third_party/refloat/src/pid.c:41-46`.
#[inline(always)]
fn refloat_integral_current(
    integral: MotorCurrent,
    error: RefloatSetpointError,
    ki: f32,
    limit: MotorCurrent,
) -> MotorCurrent {
    let next = integral + refloat_motor_current(error.angle().as_degrees() * ki);
    if limit.current().as_amps() > 0.0 && next.current().as_amps().abs() > limit.current().as_amps()
    {
        refloat_motor_current(limit.current().as_amps() * next.current().as_amps().signum())
    } else {
        next
    }
}

/// Source map: upstream smooths PID brake/accel scales at
/// `third_party/refloat/src/pid.c:48-67`.
#[inline(always)]
fn refloat_update_pid_scales(
    config: RefloatBalanceLoopConfig,
    motor_erpm: ElectricalSpeed,
    mut state: RefloatBalanceLoopState,
) -> RefloatBalanceLoopState {
    let erpm = refloat_electrical_rpm(motor_erpm).as_revolutions_per_minute();
    if erpm.abs() < 500.0 {
        state.pid_kp_brake_scale = 0.01 + 0.99 * state.pid_kp_brake_scale;
        state.pid_kp2_brake_scale = 0.01 + 0.99 * state.pid_kp2_brake_scale;
        state.pid_kp_accel_scale = 0.01 + 0.99 * state.pid_kp_accel_scale;
        state.pid_kp2_accel_scale = 0.01 + 0.99 * state.pid_kp2_accel_scale;
    } else if erpm > 0.0 {
        state.pid_kp_brake_scale = 0.01 * config.kp_brake + 0.99 * state.pid_kp_brake_scale;
        state.pid_kp2_brake_scale = 0.01 * config.kp2_brake + 0.99 * state.pid_kp2_brake_scale;
        state.pid_kp_accel_scale = 0.01 + 0.99 * state.pid_kp_accel_scale;
        state.pid_kp2_accel_scale = 0.01 + 0.99 * state.pid_kp2_accel_scale;
    } else {
        state.pid_kp_brake_scale = 0.01 + 0.99 * state.pid_kp_brake_scale;
        state.pid_kp2_brake_scale = 0.01 + 0.99 * state.pid_kp2_brake_scale;
        state.pid_kp_accel_scale = 0.01 * config.kp_brake + 0.99 * state.pid_kp_accel_scale;
        state.pid_kp2_accel_scale = 0.01 * config.kp2_brake + 0.99 * state.pid_kp2_accel_scale;
    }
    state
}

/// Source map: upstream computes angle P at
/// `third_party/refloat/src/pid.c:69`.
#[inline(always)]
fn refloat_angle_p_current(
    error: RefloatSetpointError,
    kp: f32,
    accel_scale: f32,
    brake_scale: f32,
) -> MotorCurrent {
    let scale = if error.angle().as_degrees() > 0.0 {
        accel_scale
    } else {
        brake_scale
    };
    refloat_motor_current(error.angle().as_degrees() * kp * scale)
}

/// Source map: upstream computes rate P at
/// `third_party/refloat/src/pid.c:71-72`.
#[inline(always)]
fn refloat_rate_p_current(
    pitch_rate: RefloatPitchRate,
    kp2: f32,
    accel_scale: f32,
    brake_scale: f32,
) -> MotorCurrent {
    let rate_p = refloat_motor_current(-pitch_rate.rate().as_degrees_per_second() * kp2);
    rate_p
        * if rate_p.current().as_amps() > 0.0 {
            accel_scale
        } else {
            brake_scale
        }
}

/// Source map: upstream computes booster target current at
/// `third_party/refloat/src/booster.c:32-73`.
#[inline(always)]
fn refloat_booster_target_current(
    config: RefloatBalanceLoopConfig,
    motor_erpm: ElectricalSpeed,
    motor_current: MotorCurrent,
    proportional: RefloatBoosterProportional,
) -> MotorCurrent {
    let braking = refloat_motor_braking(motor_current);
    let mut current = if braking {
        config.brkbooster_current
    } else {
        config.booster_current
    };
    let mut angle = if braking {
        config.brkbooster_angle
    } else {
        config.booster_angle
    };
    let ramp = if braking {
        config.brkbooster_ramp
    } else {
        config.booster_ramp
    };

    let abs_erpm = refloat_electrical_rpm(motor_erpm)
        .as_revolutions_per_minute()
        .abs();
    if abs_erpm > 3000.0 {
        let speed_stiffness = ((abs_erpm - 3000.0) / 10000.0).min(1.0);
        if braking {
            current = current + current * speed_stiffness;
        } else {
            angle = angle / (1.0 + speed_stiffness);
        }
    }

    let proportional_degrees = proportional.angle().as_degrees();
    let abs_proportional_degrees = proportional_degrees.abs();
    if abs_proportional_degrees > angle.as_degrees() {
        let past_start_angle_degrees = abs_proportional_degrees - angle.as_degrees();
        if past_start_angle_degrees < ramp.as_degrees() {
            current * (proportional_degrees.signum() * past_start_angle_degrees / ramp.as_degrees())
        } else {
            current * proportional_degrees.signum()
        }
    } else {
        refloat_motor_current(0.0)
    }
}

/// Source map: upstream filters booster current at
/// `third_party/refloat/src/booster.c:74-75`.
#[inline(always)]
fn refloat_filter_booster_current(target: MotorCurrent, previous: MotorCurrent) -> MotorCurrent {
    target * 0.01 + previous * 0.99
}

/// Source map: upstream soft-start clamps pitch-based current at
/// `third_party/refloat/src/main.c:924-930`.
#[inline(always)]
fn refloat_pitch_based_current(
    rate_p: MotorCurrent,
    booster: MotorCurrent,
    softstart_pid_limit: MotorCurrent,
    motor_current_max: MotorCurrent,
    hertz: SampleRate,
) -> (MotorCurrent, MotorCurrent) {
    let pitch_based = rate_p + booster;
    if softstart_pid_limit < motor_current_max {
        let pitch_based_amps = pitch_based.current().as_amps();
        (
            refloat_motor_current(
                pitch_based_amps.signum()
                    * pitch_based_amps
                        .abs()
                        .min(softstart_pid_limit.current().as_amps()),
            ),
            softstart_pid_limit + refloat_motor_current(100.0 / hertz.as_hertz().max(1.0)),
        )
    } else {
        (pitch_based, softstart_pid_limit)
    }
}

/// Source map: upstream selects RUNNING current limit at
/// `third_party/refloat/src/main.c:932-942`.
#[inline(always)]
fn refloat_current_limit(
    mode: RefloatMode,
    braking: bool,
    motor_current_max: MotorCurrent,
    motor_current_min: MotorCurrent,
) -> RefloatCurrentLimitMagnitude {
    RefloatCurrentLimitMagnitude::from_motor_current(match mode {
        RefloatMode::HandTest => refloat_motor_current(7.0),
        RefloatMode::Flywheel => refloat_motor_current(40.0),
        RefloatMode::Normal if braking => motor_current_min,
        RefloatMode::Normal => motor_current_max,
    })
}

/// Source map: upstream clamps RUNNING current at
/// `third_party/refloat/src/main.c:941-942`.
#[inline(always)]
fn refloat_clamp_current(
    current: MotorCurrent,
    limit: RefloatCurrentLimitMagnitude,
) -> MotorCurrent {
    if current.current().as_amps().abs() > limit.as_amps() {
        refloat_motor_current(limit.as_amps() * current.current().as_amps().signum())
    } else {
        current
    }
}

/// Calculate one upstream Refloat RUNNING balance-current step.
///
/// Source map: upstream calls `pid_update`, `booster_update`, soft-start,
/// current limiting, darkride inversion, traction freewheel, and motor-current
/// request in `third_party/refloat/src/main.c:918-956`; the subroutines are
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/imu.c:43-53`.
#[inline(always)]
pub(crate) fn refloat_balance_loop_step(
    config: RefloatBalanceLoopConfig,
    input: RefloatBalanceLoopInput,
    mut state: RefloatBalanceLoopState,
) -> RefloatBalanceLoopOutput {
    let setpoint_error = refloat_setpoint_error(input.setpoint, input.balance_pitch);
    state.pid_integral_current = refloat_integral_current(
        state.pid_integral_current,
        setpoint_error,
        config.ki,
        config.ki_limit,
    );
    state = refloat_update_pid_scales(config, input.motor_erpm, state);

    let angle_p_current = refloat_angle_p_current(
        setpoint_error,
        config.kp,
        state.pid_kp_accel_scale,
        state.pid_kp_brake_scale,
    );
    let pitch_rate =
        refloat_pitch_rate(input.roll, input.gyro_pitch, input.gyro_yaw, input.darkride);
    let rate_p_current = refloat_rate_p_current(
        pitch_rate,
        config.kp2,
        state.pid_kp2_accel_scale,
        state.pid_kp2_brake_scale,
    );

    let booster_proportional =
        refloat_booster_proportional(input.setpoint, input.brake_tilt_setpoint, input.raw_pitch);
    let booster_target_current = refloat_booster_target_current(
        config,
        input.motor_erpm,
        input.motor_current,
        booster_proportional,
    );
    state.booster_current =
        refloat_filter_booster_current(booster_target_current, state.booster_current);

    let (pitch_based_current, softstart_pid_limit) = refloat_pitch_based_current(
        rate_p_current,
        state.booster_current,
        state.softstart_pid_limit,
        input.motor_current_max,
        config.hertz,
    );
    state.softstart_pid_limit = softstart_pid_limit;

    let new_current = angle_p_current + state.pid_integral_current + pitch_based_current;
    let current_limit = refloat_current_limit(
        input.mode,
        refloat_motor_braking(input.motor_current),
        input.motor_current_max,
        input.motor_current_min,
    );
    let new_current = refloat_clamp_current(new_current, current_limit);
    let new_current = if matches!(input.darkride, RefloatDarkRideState::Active) {
        -new_current
    } else {
        new_current
    };

    state.balance_current = if input.traction_control {
        refloat_motor_current(0.0)
    } else {
        state.balance_current * 0.8 + new_current * 0.2
    };

    RefloatBalanceLoopOutput {
        requested_current: state.balance_current,
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn angle(degrees: f32) -> AngleDegrees {
        AngleDegrees::from_degrees(degrees)
    }

    fn current(amps: f32) -> MotorCurrent {
        MotorCurrent::new(Current::from_amps(amps))
    }

    fn erpm(revolutions_per_minute: f32) -> ElectricalSpeed {
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(revolutions_per_minute))
    }

    fn hertz(value: f32) -> SampleRate {
        SampleRate::from_hertz(value)
    }

    fn setpoint(degrees: f32) -> RefloatRealtimeRuntimeSetpoint {
        RefloatRealtimeRuntimeSetpoint::new(angle(degrees))
    }

    fn balance_pitch(degrees: f32) -> RefloatRealtimeBalancePitch {
        RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(
            degrees * core::f32::consts::PI / 180.0,
        ))
    }

    fn raw_pitch(degrees: f32) -> ImuPitch {
        ImuPitch::new(AngleRadians::from_radians(
            degrees * core::f32::consts::PI / 180.0,
        ))
    }

    fn roll(radians: f32) -> ImuRoll {
        ImuRoll::new(AngleRadians::from_radians(radians))
    }

    fn gyro(degrees_per_second: f32) -> AngularVelocity {
        AngularVelocity::from_degrees_per_second(degrees_per_second)
    }

    fn base_config() -> RefloatBalanceLoopConfig {
        RefloatBalanceLoopConfig {
            kp: 0.0,
            kp2: 0.0,
            ki: 0.0,
            kp_brake: 1.0,
            kp2_brake: 1.0,
            ki_limit: current(0.0),
            booster_angle: angle(0.0),
            booster_ramp: angle(0.0),
            booster_current: current(0.0),
            brkbooster_angle: angle(0.0),
            brkbooster_ramp: angle(0.0),
            brkbooster_current: current(0.0),
            hertz: hertz(100.0),
        }
    }

    fn base_input() -> RefloatBalanceLoopInput {
        RefloatBalanceLoopInput {
            setpoint: setpoint(0.0),
            brake_tilt_setpoint: setpoint(0.0),
            balance_pitch: balance_pitch(0.0),
            raw_pitch: raw_pitch(0.0),
            roll: roll(0.0),
            gyro_pitch: gyro(0.0),
            gyro_yaw: gyro(0.0),
            motor_erpm: erpm(0.0),
            motor_current: current(1.0),
            motor_current_max: current(100.0),
            motor_current_min: current(100.0),
            mode: RefloatMode::Normal,
            darkride: RefloatDarkRideState::Upright,
            traction_control: false,
        }
    }

    fn base_state() -> RefloatBalanceLoopState {
        RefloatBalanceLoopState {
            balance_current: current(0.0),
            booster_current: current(0.0),
            pid_integral_current: current(0.0),
            pid_kp_brake_scale: 1.0,
            pid_kp2_brake_scale: 1.0,
            pid_kp_accel_scale: 1.0,
            pid_kp2_accel_scale: 1.0,
            softstart_pid_limit: current(100.0),
        }
    }

    fn assert_current_amps(actual: MotorCurrent, expected: f32) {
        assert!((actual.current().as_amps() - expected).abs() < 0.0001);
    }

    #[test]
    fn balance_loop_unit_limits_normal_current_like_refloat_main_loop() {
        let config = RefloatBalanceLoopConfig {
            kp: 10.0,
            ..base_config()
        };
        let cases = [
            (current(1.0), setpoint(10.0), current(3.0), 0.6_f32),
            (current(-1.0), setpoint(-10.0), current(2.0), -0.4_f32),
        ];

        cases.into_iter().for_each(
            |(motor_current, board_setpoint, current_limit, expected_amps)| {
                let output = refloat_balance_loop_step(
                    config,
                    RefloatBalanceLoopInput {
                        setpoint: board_setpoint,
                        motor_current,
                        motor_current_max: current(3.0),
                        motor_current_min: current_limit,
                        ..base_input()
                    },
                    base_state(),
                );

                // Upstream `pid_update` computes P/I at
                // `third_party/refloat/src/pid.c:40-46`; RUNNING selects max
                // or min current limit at `third_party/refloat/src/main.c:932-942`
                // and smooths at `third_party/refloat/src/main.c:949-954`.
                assert_current_amps(output.state.balance_current, expected_amps);
            },
        );
    }

    #[test]
    fn balance_loop_unit_treats_motor_current_min_as_magnitude_like_refloat_main_loop() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                kp: 10.0,
                ..base_config()
            },
            RefloatBalanceLoopInput {
                setpoint: setpoint(-10.0),
                motor_current: current(-1.0),
                motor_current_max: current(100.0),
                motor_current_min: current(-2.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream treats `current_limit` as a positive scalar before clamping
        // `new_current` at `third_party/refloat/src/main.c:932-942`, even
        // though VESC stores braking current as a negative config value.
        assert_current_amps(output.requested_current, -0.4);
    }

    #[test]
    fn balance_loop_unit_positive_pitch_rate_commands_negative_damping_current() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                kp2: 2.0,
                ..base_config()
            },
            RefloatBalanceLoopInput {
                gyro_pitch: gyro(10.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
        // `third_party/refloat/src/pid.c:66-72`; RUNNING smooths the requested
        // current at `third_party/refloat/src/main.c:949-954`.
        assert_current_amps(output.requested_current, -4.0);
    }

    #[test]
    fn balance_loop_unit_negative_pitch_rate_commands_positive_damping_current() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                kp2: 2.0,
                ..base_config()
            },
            RefloatBalanceLoopInput {
                gyro_pitch: gyro(-10.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
        // `third_party/refloat/src/pid.c:66-72`; RUNNING smooths the requested
        // current at `third_party/refloat/src/main.c:949-954`.
        assert_current_amps(output.requested_current, 4.0);
    }

    #[test]
    fn balance_loop_unit_filters_booster_and_softstart_like_refloat_main_loop() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                booster_angle: angle(1.0),
                booster_ramp: angle(1.0),
                booster_current: current(20.0),
                brkbooster_angle: angle(1.0),
                brkbooster_ramp: angle(1.0),
                brkbooster_current: current(20.0),
                ..base_config()
            },
            RefloatBalanceLoopInput {
                setpoint: setpoint(3.0),
                motor_current: current(1.0),
                motor_current_max: current(3.0),
                motor_current_min: current(2.0),
                ..base_input()
            },
            RefloatBalanceLoopState {
                softstart_pid_limit: current(0.0),
                ..base_state()
            },
        );

        // Upstream `booster_update` ramps/filter current at
        // `third_party/refloat/src/booster.c:63-75`; RUNNING soft-start clamps
        // pitch-based current and increments the limit at
        // `third_party/refloat/src/main.c:924-930`.
        assert_current_amps(output.state.booster_current, 0.2);
        assert_current_amps(output.state.balance_current, 0.0);
        assert_current_amps(output.requested_current, 0.0);
        assert_current_amps(output.state.softstart_pid_limit, 1.0);
    }

    #[test]
    fn balance_loop_unit_booster_proportional_subtracts_brake_tilt_like_refloat_main_loop() {
        let proportional =
            refloat_booster_proportional(setpoint(5.0), setpoint(5.0), raw_pitch(0.0));

        // Upstream subtracts brake tilt from booster proportional before
        // `booster_update` at `third_party/refloat/src/main.c:921-922`.
        assert_eq!(proportional.angle().as_degrees(), 0.0);
    }

    #[test]
    fn balance_loop_unit_booster_subtracts_brake_tilt_like_refloat_main_loop() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                booster_angle: angle(0.0),
                booster_ramp: angle(1.0),
                booster_current: current(20.0),
                brkbooster_angle: angle(0.0),
                brkbooster_ramp: angle(1.0),
                brkbooster_current: current(20.0),
                ..base_config()
            },
            RefloatBalanceLoopInput {
                setpoint: setpoint(5.0),
                brake_tilt_setpoint: setpoint(5.0),
                motor_erpm: erpm(1000.0),
                motor_current: current(1.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream subtracts brake tilt from booster proportional before
        // `booster_update` at `third_party/refloat/src/main.c:921-922`.
        assert_current_amps(output.state.booster_current, 0.0);
        assert_current_amps(output.requested_current, 0.0);
    }

    #[test]
    fn balance_loop_unit_pitch_rate_mixes_axes_and_darkride_like_refloat_imu() {
        let upright = refloat_pitch_rate(
            roll(0.0),
            gyro(12.0),
            gyro(100.0),
            RefloatDarkRideState::Upright,
        );
        let darkride = refloat_pitch_rate(
            roll(0.0),
            gyro(12.0),
            gyro(100.0),
            RefloatDarkRideState::Active,
        );

        // Upstream mixes roll, gyro Y, and gyro Z at
        // `third_party/refloat/src/imu.c:46-51`, then flips darkride at
        // `third_party/refloat/src/imu.c:52-53`.
        assert_eq!(upright.rate().as_degrees_per_second(), 12.0);
        assert_eq!(darkride.rate().as_degrees_per_second(), -12.0);
    }

    #[test]
    fn balance_loop_unit_darkride_and_traction_control_match_refloat_main_loop() {
        let config = RefloatBalanceLoopConfig {
            kp: 1.0,
            ..base_config()
        };
        let base_input = RefloatBalanceLoopInput {
            setpoint: setpoint(10.0),
            darkride: RefloatDarkRideState::Active,
            ..base_input()
        };
        let state = RefloatBalanceLoopState {
            balance_current: current(10.0),
            ..base_state()
        };

        let darkride_output = refloat_balance_loop_step(config, base_input, state);
        let traction_output = refloat_balance_loop_step(
            config,
            RefloatBalanceLoopInput {
                traction_control: true,
                ..base_input
            },
            state,
        );

        // Upstream RUNNING flips darkride current at
        // `third_party/refloat/src/main.c:944-946`; traction control freewheels
        // at `third_party/refloat/src/main.c:949-954`.
        assert_current_amps(darkride_output.state.balance_current, 6.0);
        assert_current_amps(traction_output.state.balance_current, 0.0);
    }
}
