//! Refloat RUNNING balance-current calculation.
//!
//! Source map: upstream executes this path from `refloat_thd` at
//! `third_party/refloat/src/main.c:918-956`, with PID math in
//! `third_party/refloat/src/pid.c:37-73`, booster math in
//! `third_party/refloat/src/booster.c:32-75`, and pitch-rate input from
//! `third_party/refloat/src/imu.c:43-53`.

use crate::domain::{RefloatDarkRideState, RefloatMode};

/// Config inputs consumed by one Refloat RUNNING balance-current step.
///
/// Source map: upstream reads these values from `RefloatConfig` in
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/main.c:924-942`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopConfig {
    pub(crate) kp: f32,
    pub(crate) kp2: f32,
    pub(crate) ki: f32,
    pub(crate) kp_brake: f32,
    pub(crate) kp2_brake: f32,
    pub(crate) ki_limit: f32,
    pub(crate) booster_angle: f32,
    pub(crate) booster_ramp: f32,
    pub(crate) booster_current: f32,
    pub(crate) brkbooster_angle: f32,
    pub(crate) brkbooster_ramp: f32,
    pub(crate) brkbooster_current: f32,
    pub(crate) hertz: u16,
}

/// Runtime inputs consumed by one Refloat RUNNING balance-current step.
///
/// Source map: upstream combines setpoint, IMU, motor, mode, darkride, and
/// traction state in `third_party/refloat/src/main.c:918-956`; pitch-rate input
/// is prepared by `third_party/refloat/src/imu.c:43-53`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopInput {
    pub(crate) setpoint_degrees: f32,
    pub(crate) balance_pitch_degrees: f32,
    pub(crate) raw_pitch_degrees: f32,
    pub(crate) roll_radians: f32,
    pub(crate) gyro_pitch_degrees_per_second: f32,
    pub(crate) gyro_yaw_degrees_per_second: f32,
    pub(crate) motor_erpm: f32,
    pub(crate) motor_current_amps: f32,
    pub(crate) motor_current_max_amps: f32,
    pub(crate) motor_current_min_amps: f32,
    pub(crate) mode: RefloatMode,
    pub(crate) darkride: RefloatDarkRideState,
    pub(crate) traction_control: bool,
}

/// Mutable PID/booster/current state for one Refloat balance-current step.
///
/// Source map: upstream stores these fields in `Data.pid`, `Data.booster`,
/// `Data.softstart_pid_limit`, and `Data.balance_current` while running
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/main.c:924-954`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopState {
    pub(crate) balance_current_amps: f32,
    pub(crate) booster_current_amps: f32,
    pub(crate) pid_integral_current: f32,
    pub(crate) pid_kp_brake_scale: f32,
    pub(crate) pid_kp2_brake_scale: f32,
    pub(crate) pid_kp_accel_scale: f32,
    pub(crate) pid_kp2_accel_scale: f32,
    pub(crate) softstart_pid_limit: f32,
}

/// Result of one Refloat RUNNING balance-current step.
///
/// Source map: upstream stores `d->balance_current` and immediately requests it
/// from motor control at `third_party/refloat/src/main.c:949-956`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopOutput {
    pub(crate) state: RefloatBalanceLoopState,
    pub(crate) requested_current_amps: f32,
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
    let setpoint_error = input.setpoint_degrees - input.balance_pitch_degrees;
    state.pid_integral_current += setpoint_error * config.ki;
    if config.ki_limit > 0.0 && state.pid_integral_current.abs() > config.ki_limit {
        state.pid_integral_current = config.ki_limit * state.pid_integral_current.signum();
    }

    if input.motor_erpm.abs() < 500.0 {
        state.pid_kp_brake_scale = 0.01 + 0.99 * state.pid_kp_brake_scale;
        state.pid_kp2_brake_scale = 0.01 + 0.99 * state.pid_kp2_brake_scale;
        state.pid_kp_accel_scale = 0.01 + 0.99 * state.pid_kp_accel_scale;
        state.pid_kp2_accel_scale = 0.01 + 0.99 * state.pid_kp2_accel_scale;
    } else if input.motor_erpm > 0.0 {
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

    let angle_p_current = setpoint_error
        * config.kp
        * if setpoint_error > 0.0 {
            state.pid_kp_accel_scale
        } else {
            state.pid_kp_brake_scale
        };

    let sin_roll = libm::sinf(input.roll_radians);
    let cos_roll = libm::cosf(input.roll_radians);
    let mut pitch_rate = cos_roll * cos_roll * input.gyro_pitch_degrees_per_second
        + sin_roll * cos_roll * input.gyro_yaw_degrees_per_second;
    if matches!(input.darkride, RefloatDarkRideState::Active) {
        pitch_rate = -pitch_rate;
    }
    let rate_p_current = -pitch_rate
        * config.kp2
        * if -pitch_rate > 0.0 {
            state.pid_kp2_accel_scale
        } else {
            state.pid_kp2_brake_scale
        };

    let booster_proportional = input.setpoint_degrees - input.raw_pitch_degrees;
    let braking = input.motor_current_amps < 0.0;
    let mut booster_target_current = if braking {
        config.brkbooster_current
    } else {
        config.booster_current
    };
    let mut booster_start_angle = if braking {
        config.brkbooster_angle
    } else {
        config.booster_angle
    };
    let booster_ramp = if braking {
        config.brkbooster_ramp
    } else {
        config.booster_ramp
    };
    if input.motor_erpm.abs() > 3000.0 {
        let speed_stiffness = ((input.motor_erpm.abs() - 3000.0) / 10000.0).min(1.0);
        if braking {
            booster_target_current += booster_target_current * speed_stiffness;
        } else {
            booster_start_angle /= 1.0 + speed_stiffness;
        }
    }
    let abs_booster_proportional = booster_proportional.abs();
    if abs_booster_proportional > booster_start_angle {
        let past_start_angle = abs_booster_proportional - booster_start_angle;
        if past_start_angle < booster_ramp {
            booster_target_current *=
                booster_proportional.signum() * past_start_angle / booster_ramp;
        } else {
            booster_target_current *= booster_proportional.signum();
        }
    } else {
        booster_target_current = 0.0;
    }
    state.booster_current_amps = booster_target_current * 0.01 + state.booster_current_amps * 0.99;

    let mut pitch_based_current = rate_p_current + state.booster_current_amps;
    if state.softstart_pid_limit < input.motor_current_max_amps {
        pitch_based_current =
            pitch_based_current.signum() * pitch_based_current.abs().min(state.softstart_pid_limit);
        state.softstart_pid_limit += 100.0 / f32::from(config.hertz.max(1));
    }
    let current_limit = match input.mode {
        RefloatMode::HandTest => 7.0,
        RefloatMode::Flywheel => 40.0,
        RefloatMode::Normal if braking => input.motor_current_min_amps,
        RefloatMode::Normal => input.motor_current_max_amps,
    };
    let mut new_current = angle_p_current + state.pid_integral_current + pitch_based_current;
    if new_current.abs() > current_limit {
        new_current = new_current.signum() * current_limit;
    }
    if matches!(input.darkride, RefloatDarkRideState::Active) {
        new_current = -new_current;
    }

    state.balance_current_amps = if input.traction_control {
        0.0
    } else {
        state.balance_current_amps * 0.8 + new_current * 0.2
    };

    RefloatBalanceLoopOutput {
        requested_current_amps: state.balance_current_amps,
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_loop_unit_limits_normal_current_like_refloat_main_loop() {
        let base_state = RefloatBalanceLoopState {
            balance_current_amps: 0.0,
            booster_current_amps: 0.0,
            pid_integral_current: 0.0,
            pid_kp_brake_scale: 1.0,
            pid_kp2_brake_scale: 1.0,
            pid_kp_accel_scale: 1.0,
            pid_kp2_accel_scale: 1.0,
            softstart_pid_limit: 100.0,
        };
        let config = RefloatBalanceLoopConfig {
            kp: 10.0,
            kp2: 0.0,
            ki: 0.0,
            kp_brake: 1.0,
            kp2_brake: 1.0,
            ki_limit: 0.0,
            booster_angle: 0.0,
            booster_ramp: 0.0,
            booster_current: 0.0,
            brkbooster_angle: 0.0,
            brkbooster_ramp: 0.0,
            brkbooster_current: 0.0,
            hertz: 100,
        };

        let cases = [
            (1.0_f32, 10.0_f32, 3.0_f32, 0.6_f32),
            (-1.0_f32, -10.0_f32, 2.0_f32, -0.4_f32),
        ];

        cases.into_iter().for_each(
            |(motor_current_amps, setpoint_degrees, limit_amps, expected_amps)| {
                let output = refloat_balance_loop_step(
                    config,
                    RefloatBalanceLoopInput {
                        setpoint_degrees,
                        balance_pitch_degrees: 0.0,
                        raw_pitch_degrees: 0.0,
                        roll_radians: 0.0,
                        gyro_pitch_degrees_per_second: 0.0,
                        gyro_yaw_degrees_per_second: 0.0,
                        motor_erpm: 0.0,
                        motor_current_amps,
                        motor_current_max_amps: 3.0,
                        motor_current_min_amps: limit_amps,
                        mode: RefloatMode::Normal,
                        darkride: RefloatDarkRideState::Upright,
                        traction_control: false,
                    },
                    base_state,
                );

                // Upstream `pid_update` computes P/I at
                // `third_party/refloat/src/pid.c:40-46`; RUNNING selects max
                // or min current limit at `third_party/refloat/src/main.c:932-942`
                // and smooths at `third_party/refloat/src/main.c:949-954`.
                assert!((output.state.balance_current_amps - expected_amps).abs() < 0.0001);
            },
        );
    }

    #[test]
    fn balance_loop_unit_filters_booster_and_softstart_like_refloat_main_loop() {
        let output = refloat_balance_loop_step(
            RefloatBalanceLoopConfig {
                kp: 0.0,
                kp2: 0.0,
                ki: 0.0,
                kp_brake: 1.0,
                kp2_brake: 1.0,
                ki_limit: 0.0,
                booster_angle: 1.0,
                booster_ramp: 1.0,
                booster_current: 20.0,
                brkbooster_angle: 1.0,
                brkbooster_ramp: 1.0,
                brkbooster_current: 20.0,
                hertz: 100,
            },
            RefloatBalanceLoopInput {
                setpoint_degrees: 3.0,
                balance_pitch_degrees: 0.0,
                raw_pitch_degrees: 0.0,
                roll_radians: 0.0,
                gyro_pitch_degrees_per_second: 0.0,
                gyro_yaw_degrees_per_second: 0.0,
                motor_erpm: 0.0,
                motor_current_amps: 1.0,
                motor_current_max_amps: 3.0,
                motor_current_min_amps: 2.0,
                mode: RefloatMode::Normal,
                darkride: RefloatDarkRideState::Upright,
                traction_control: false,
            },
            RefloatBalanceLoopState {
                balance_current_amps: 0.0,
                booster_current_amps: 0.0,
                pid_integral_current: 0.0,
                pid_kp_brake_scale: 1.0,
                pid_kp2_brake_scale: 1.0,
                pid_kp_accel_scale: 1.0,
                pid_kp2_accel_scale: 1.0,
                softstart_pid_limit: 0.0,
            },
        );

        // Upstream `booster_update` ramps/filter current at
        // `third_party/refloat/src/booster.c:63-75`; RUNNING soft-start clamps
        // pitch-based current and increments the limit at
        // `third_party/refloat/src/main.c:924-930`.
        assert!((output.state.booster_current_amps - 0.2).abs() < 0.0001);
        assert_eq!(output.state.balance_current_amps, 0.0);
        assert_eq!(output.requested_current_amps, 0.0);
        assert_eq!(output.state.softstart_pid_limit, 1.0);
    }

    #[test]
    fn balance_loop_unit_darkride_and_traction_control_match_refloat_main_loop() {
        let config = RefloatBalanceLoopConfig {
            kp: 1.0,
            kp2: 0.0,
            ki: 0.0,
            kp_brake: 1.0,
            kp2_brake: 1.0,
            ki_limit: 0.0,
            booster_angle: 0.0,
            booster_ramp: 0.0,
            booster_current: 0.0,
            brkbooster_angle: 0.0,
            brkbooster_ramp: 0.0,
            brkbooster_current: 0.0,
            hertz: 100,
        };
        let base_input = RefloatBalanceLoopInput {
            setpoint_degrees: 10.0,
            balance_pitch_degrees: 0.0,
            raw_pitch_degrees: 0.0,
            roll_radians: 0.0,
            gyro_pitch_degrees_per_second: 0.0,
            gyro_yaw_degrees_per_second: 0.0,
            motor_erpm: 0.0,
            motor_current_amps: 1.0,
            motor_current_max_amps: 100.0,
            motor_current_min_amps: 100.0,
            mode: RefloatMode::Normal,
            darkride: RefloatDarkRideState::Active,
            traction_control: false,
        };
        let state = RefloatBalanceLoopState {
            balance_current_amps: 10.0,
            booster_current_amps: 0.0,
            pid_integral_current: 0.0,
            pid_kp_brake_scale: 1.0,
            pid_kp2_brake_scale: 1.0,
            pid_kp_accel_scale: 1.0,
            pid_kp2_accel_scale: 1.0,
            softstart_pid_limit: 100.0,
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
        assert_eq!(darkride_output.state.balance_current_amps, 6.0);
        assert_eq!(traction_output.state.balance_current_amps, 0.0);
    }
}
