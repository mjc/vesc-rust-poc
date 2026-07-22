use super::loop_io::{LoopConfig, LoopInput, LoopOutput, LoopState};

use super::{booster::Phase as BoosterPhase, pid::Phase as PidPhase};

#[cfg(test)]
use super::{
    booster::{Profile, Proportional},
    pid::PitchRate,
};

#[cfg(test)]
use crate::domain::{RefloatDarkRideState, RefloatMode, RefloatRealtimeRuntimeSetpoint};

#[cfg(test)]
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, ImuRoll,
    IntegralCurrentGain, MotorCurrent, MotorCurrentLimit, PidScale, RateCurrentGain, SampleRate,
};

impl LoopState {
    /// Advance one upstream Refloat RUNNING balance-current step.
    ///
    /// Source map: upstream calls `pid_update`, `booster_update`, soft-start,
    /// current limiting, darkride inversion, traction freewheel, and motor-current
    /// request in `third_party/refloat/src/main.c:918-956`; the subroutines are
    /// `third_party/refloat/src/pid.c:37-73`,
    /// `third_party/refloat/src/booster.c:32-75`, and
    /// `third_party/refloat/src/imu.c:43-53`.
    #[inline(always)]
    pub(crate) fn advance_balance_loop(self, config: LoopConfig, input: LoopInput) -> LoopOutput {
        let (pid_currents, state) = PidPhase::from_step(config, input).update_state(self);
        let booster_current =
            BoosterPhase::from_step(config, input).filtered_current(state.booster_current);
        let pitch_based = pid_currents.pitch_based_current(
            booster_current,
            state.softstart_pid_limit,
            input.motor_current_max,
            config.hertz,
        );
        let state = state.with_booster_current_and_softstart_limit(
            booster_current,
            pitch_based.softstart_pid_limit,
        );

        let balance_current = pid_currents
            .requested_with_pitch_based(pitch_based)
            .clamped_to(input.current_limit())
            .adjusted_for_darkride(input.darkride)
            .filtered_from(state.balance_current, input.traction_control);
        let state = state.with_balance_current(balance_current);

        LoopOutput {
            requested_current: state.balance_current,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::prelude::{AngleRadians, Rpm};

    fn motor_current(current: Current) -> MotorCurrent {
        MotorCurrent::new(current)
    }

    fn motor_current_limit(current: Current) -> MotorCurrentLimit {
        MotorCurrentLimit::new(current)
    }

    fn electrical_speed(speed: Rpm) -> ElectricalSpeed {
        ElectricalSpeed::new(speed)
    }

    fn setpoint(angle: AngleDegrees) -> RefloatRealtimeRuntimeSetpoint {
        RefloatRealtimeRuntimeSetpoint::new(angle)
    }

    fn roll(angle: AngleRadians) -> ImuRoll {
        ImuRoll::new(angle)
    }

    fn base_config() -> LoopConfig {
        LoopConfig {
            kp: AngleCurrentGain::new(0.0),
            kp2: RateCurrentGain::new(0.0),
            ki: IntegralCurrentGain::new(0.0),
            kp_brake: PidScale::new(1.0),
            kp2_brake: PidScale::new(1.0),
            ki_limit: motor_current_limit(Current::from_amps(0.0)),
            booster_angle: AngleDegrees::from_degrees(0.0),
            booster_ramp: AngleDegrees::from_degrees(0.0),
            booster_current: motor_current(Current::from_amps(0.0)),
            brkbooster_angle: AngleDegrees::from_degrees(0.0),
            brkbooster_ramp: AngleDegrees::from_degrees(0.0),
            brkbooster_current: motor_current(Current::from_amps(0.0)),
            hertz: SampleRate::from_hertz(100.0),
        }
    }

    fn base_input() -> LoopInput {
        LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(0.0)),
            brake_tilt_setpoint: RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
                0.0,
            )),
            balance_pitch: AngleDegrees::from_degrees(0.0),
            raw_pitch: AngleDegrees::from_degrees(0.0),
            roll: roll(AngleRadians::from_radians(0.0)),
            gyro_pitch: AngularVelocity::from_degrees_per_second(0.0),
            gyro_yaw: AngularVelocity::from_degrees_per_second(0.0),
            motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(0.0)),
            motor_current: motor_current(Current::from_amps(1.0)),
            motor_current_max: motor_current_limit(Current::from_amps(100.0)),
            motor_current_min: motor_current_limit(Current::from_amps(100.0)),
            mode: RefloatMode::Normal,
            darkride: RefloatDarkRideState::Upright,
            traction_control: false,
        }
    }

    fn base_state() -> LoopState {
        LoopState {
            balance_current: motor_current(Current::from_amps(0.0)),
            booster_current: motor_current(Current::from_amps(0.0)),
            pid_integral_current: motor_current(Current::from_amps(0.0)),
            pid_kp_brake_scale: PidScale::new(1.0),
            pid_kp2_brake_scale: PidScale::new(1.0),
            pid_kp_accel_scale: PidScale::new(1.0),
            pid_kp2_accel_scale: PidScale::new(1.0),
            softstart_pid_limit: motor_current(Current::from_amps(100.0)),
        }
    }

    fn assert_current(actual: MotorCurrent, expected: MotorCurrent) {
        assert!((actual.current().as_amps() - expected.current().as_amps()).abs() < 0.0001);
    }

    fn assert_scale(actual: PidScale, expected: PidScale) {
        assert!((actual.value() - expected.value()).abs() < 0.0001);
    }

    fn advance_loop(config: LoopConfig, input: LoopInput, state: LoopState) -> LoopOutput {
        state.advance_balance_loop(config, input)
    }

    #[test]
    fn balance_loop_unit_updates_pid_scales_by_erpm_direction_like_refloat_pid() {
        let config = LoopConfig {
            kp_brake: PidScale::new(2.0),
            kp2_brake: PidScale::new(3.0),
            ..base_config()
        };
        let state = base_state();

        let coasting = state.with_updated_pid_scales(
            config,
            electrical_speed(Rpm::from_revolutions_per_minute(0.0)),
        );
        let forward = state.with_updated_pid_scales(
            config,
            electrical_speed(Rpm::from_revolutions_per_minute(1000.0)),
        );
        let reverse = state.with_updated_pid_scales(
            config,
            electrical_speed(Rpm::from_revolutions_per_minute(-1000.0)),
        );

        assert_scale(coasting.pid_kp_brake_scale, PidScale::new(1.0));
        assert_scale(coasting.pid_kp2_brake_scale, PidScale::new(1.0));
        assert_scale(coasting.pid_kp_accel_scale, PidScale::new(1.0));
        assert_scale(coasting.pid_kp2_accel_scale, PidScale::new(1.0));

        assert_scale(forward.pid_kp_brake_scale, PidScale::new(1.01));
        assert_scale(forward.pid_kp2_brake_scale, PidScale::new(1.02));
        assert_scale(forward.pid_kp_accel_scale, PidScale::new(1.0));
        assert_scale(forward.pid_kp2_accel_scale, PidScale::new(1.0));

        assert_scale(reverse.pid_kp_brake_scale, PidScale::new(1.0));
        assert_scale(reverse.pid_kp2_brake_scale, PidScale::new(1.0));
        assert_scale(reverse.pid_kp_accel_scale, PidScale::new(1.01));
        assert_scale(reverse.pid_kp2_accel_scale, PidScale::new(1.02));
    }

    #[test]
    fn balance_loop_unit_persists_pid_integral_across_ticks_like_refloat_pid() {
        let config = LoopConfig {
            ki: IntegralCurrentGain::new(1.0),
            ki_limit: motor_current_limit(Current::from_amps(100.0)),
            ..base_config()
        };
        let input = LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(1.0)),
            ..base_input()
        };

        let first = advance_loop(config, input, base_state());
        let second = advance_loop(config, input, first.state);

        assert_current(
            second.state.pid_integral_current,
            motor_current(Current::from_amps(2.0)),
        );
    }

    #[test]
    fn balance_loop_unit_limits_normal_current_like_refloat_main_loop() {
        let config = LoopConfig {
            kp: AngleCurrentGain::new(10.0),
            ..base_config()
        };
        let cases = [
            (
                motor_current(Current::from_amps(1.0)),
                setpoint(AngleDegrees::from_degrees(10.0)),
                motor_current(Current::from_amps(3.0)),
                motor_current(Current::from_amps(0.6)),
            ),
            (
                motor_current(Current::from_amps(-1.0)),
                setpoint(AngleDegrees::from_degrees(-10.0)),
                motor_current(Current::from_amps(2.0)),
                motor_current(Current::from_amps(-0.4)),
            ),
        ];

        cases.into_iter().for_each(
            |(measured_current, board_setpoint, current_limit, expected_current)| {
                let output = advance_loop(
                    config,
                    LoopInput {
                        setpoint: board_setpoint,
                        motor_current: measured_current,
                        motor_current_max: motor_current_limit(Current::from_amps(3.0)),
                        motor_current_min: motor_current_limit(current_limit.current()),
                        ..base_input()
                    },
                    base_state(),
                );

                // Upstream `pid_update` computes P/I at
                // `third_party/refloat/src/pid.c:40-46`; RUNNING selects max
                // or min current limit at `third_party/refloat/src/main.c:932-942`
                // and smooths at `third_party/refloat/src/main.c:949-954`.
                assert_current(output.state.balance_current, expected_current);
            },
        );
    }

    #[test]
    fn balance_loop_unit_treats_motor_current_min_as_magnitude_like_refloat_main_loop() {
        let output = advance_loop(
            LoopConfig {
                kp: AngleCurrentGain::new(10.0),
                ..base_config()
            },
            LoopInput {
                setpoint: setpoint(AngleDegrees::from_degrees(-10.0)),
                motor_current: motor_current(Current::from_amps(-1.0)),
                motor_current_max: motor_current_limit(Current::from_amps(100.0)),
                motor_current_min: motor_current_limit(Current::from_amps(-2.0)),
                ..base_input()
            },
            base_state(),
        );

        // Upstream treats `current_limit` as a positive scalar before clamping
        // `new_current` at `third_party/refloat/src/main.c:932-942`, even
        // though VESC stores braking current as a negative config value.
        assert_current(
            output.requested_current,
            motor_current(Current::from_amps(-0.4)),
        );
    }

    #[test]
    fn balance_loop_unit_clamps_to_a_zero_firmware_current_limit() {
        let output = advance_loop(
            LoopConfig {
                kp: AngleCurrentGain::new(10.0),
                ..base_config()
            },
            LoopInput {
                setpoint: setpoint(AngleDegrees::from_degrees(10.0)),
                motor_current_max: motor_current_limit(Current::ZERO),
                ..base_input()
            },
            base_state(),
        );

        assert_eq!(output.requested_current.current(), Current::ZERO);
    }

    #[test]
    fn balance_loop_unit_positive_pitch_rate_commands_negative_damping_current() {
        let output = advance_loop(
            LoopConfig {
                kp2: RateCurrentGain::new(2.0),
                ..base_config()
            },
            LoopInput {
                gyro_pitch: AngularVelocity::from_degrees_per_second(10.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
        // `third_party/refloat/src/pid.c:66-72`; RUNNING smooths the requested
        // current at `third_party/refloat/src/main.c:949-954`.
        assert_current(
            output.requested_current,
            motor_current(Current::from_amps(-4.0)),
        );
    }

    #[test]
    fn balance_loop_unit_negative_pitch_rate_commands_positive_damping_current() {
        let output = advance_loop(
            LoopConfig {
                kp2: RateCurrentGain::new(2.0),
                ..base_config()
            },
            LoopInput {
                gyro_pitch: AngularVelocity::from_degrees_per_second(-10.0),
                ..base_input()
            },
            base_state(),
        );

        // Upstream computes `rate_p = -imu->pitch_rate * kp2` at
        // `third_party/refloat/src/pid.c:66-72`; RUNNING smooths the requested
        // current at `third_party/refloat/src/main.c:949-954`.
        assert_current(
            output.requested_current,
            motor_current(Current::from_amps(4.0)),
        );
    }

    #[test]
    fn balance_loop_unit_filters_booster_and_softstart_like_refloat_main_loop() {
        let output = advance_loop(
            LoopConfig {
                booster_angle: AngleDegrees::from_degrees(1.0),
                booster_ramp: AngleDegrees::from_degrees(1.0),
                booster_current: motor_current(Current::from_amps(20.0)),
                brkbooster_angle: AngleDegrees::from_degrees(1.0),
                brkbooster_ramp: AngleDegrees::from_degrees(1.0),
                brkbooster_current: motor_current(Current::from_amps(20.0)),
                ..base_config()
            },
            LoopInput {
                setpoint: setpoint(AngleDegrees::from_degrees(3.0)),
                motor_current: motor_current(Current::from_amps(1.0)),
                motor_current_max: motor_current_limit(Current::from_amps(3.0)),
                motor_current_min: motor_current_limit(Current::from_amps(2.0)),
                ..base_input()
            },
            LoopState {
                softstart_pid_limit: motor_current(Current::from_amps(0.0)),
                ..base_state()
            },
        );

        // Upstream `booster_update` ramps/filter current at
        // `third_party/refloat/src/booster.c:63-75`; RUNNING soft-start clamps
        // pitch-based current and increments the limit at
        // `third_party/refloat/src/main.c:924-930`.
        assert_current(
            output.state.booster_current,
            motor_current(Current::from_amps(0.2)),
        );
        assert_current(
            output.state.balance_current,
            motor_current(Current::from_amps(0.0)),
        );
        assert_current(
            output.requested_current,
            motor_current(Current::from_amps(0.0)),
        );
        assert_current(
            output.state.softstart_pid_limit,
            motor_current(Current::from_amps(1.0)),
        );
    }

    #[test]
    fn balance_loop_unit_booster_proportional_subtracts_brake_tilt_like_refloat_main_loop() {
        let proportional = Proportional::from_input(
            setpoint(AngleDegrees::from_degrees(5.0)),
            setpoint(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(0.0),
        );

        // Upstream subtracts brake tilt from booster proportional before
        // `booster_update` at `third_party/refloat/src/main.c:921-922`.
        assert_eq!(proportional.angle().as_degrees(), 0.0);
    }

    #[test]
    fn balance_loop_unit_booster_subtracts_brake_tilt_like_refloat_main_loop() {
        let output = advance_loop(
            LoopConfig {
                booster_angle: AngleDegrees::from_degrees(0.0),
                booster_ramp: AngleDegrees::from_degrees(1.0),
                booster_current: motor_current(Current::from_amps(20.0)),
                brkbooster_angle: AngleDegrees::from_degrees(0.0),
                brkbooster_ramp: AngleDegrees::from_degrees(1.0),
                brkbooster_current: motor_current(Current::from_amps(20.0)),
                ..base_config()
            },
            LoopInput {
                setpoint: setpoint(AngleDegrees::from_degrees(5.0)),
                brake_tilt_setpoint: RefloatRealtimeRuntimeSetpoint::new(
                    AngleDegrees::from_degrees(5.0),
                ),
                motor_erpm: electrical_speed(Rpm::from_revolutions_per_minute(1000.0)),
                motor_current: motor_current(Current::from_amps(1.0)),
                ..base_input()
            },
            base_state(),
        );

        // Upstream subtracts brake tilt from booster proportional before
        // `booster_update` at `third_party/refloat/src/main.c:921-922`.
        assert_current(
            output.state.booster_current,
            motor_current(Current::from_amps(0.0)),
        );
        assert_current(
            output.requested_current,
            motor_current(Current::from_amps(0.0)),
        );
    }

    #[test]
    fn booster_profile_deadbands_ramps_and_saturates_like_refloat_booster() {
        let profile = Profile {
            current: motor_current(Current::from_amps(20.0)),
            angle: AngleDegrees::from_degrees(1.0),
            ramp: AngleDegrees::from_degrees(2.0),
        };

        assert_current(
            profile.target_current(Proportional::new(AngleDegrees::from_degrees(0.5))),
            motor_current(Current::from_amps(0.0)),
        );
        assert_current(
            profile.target_current(Proportional::new(AngleDegrees::from_degrees(2.0))),
            motor_current(Current::from_amps(10.0)),
        );
        assert_current(
            profile.target_current(Proportional::new(AngleDegrees::from_degrees(-2.0))),
            motor_current(Current::from_amps(-10.0)),
        );
        assert_current(
            profile.target_current(Proportional::new(AngleDegrees::from_degrees(4.0))),
            motor_current(Current::from_amps(20.0)),
        );
    }

    #[test]
    fn balance_loop_unit_pitch_rate_mixes_axes_and_darkride_like_refloat_imu() {
        let upright = PitchRate::from_imu(
            roll(AngleRadians::from_radians(0.0)),
            AngularVelocity::from_degrees_per_second(12.0),
            AngularVelocity::from_degrees_per_second(100.0),
            RefloatDarkRideState::Upright,
        );
        let darkride = PitchRate::from_imu(
            roll(AngleRadians::from_radians(0.0)),
            AngularVelocity::from_degrees_per_second(12.0),
            AngularVelocity::from_degrees_per_second(100.0),
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
        let config = LoopConfig {
            kp: AngleCurrentGain::new(1.0),
            ..base_config()
        };
        let base_input = LoopInput {
            setpoint: setpoint(AngleDegrees::from_degrees(10.0)),
            darkride: RefloatDarkRideState::Active,
            ..base_input()
        };
        let state = LoopState {
            balance_current: motor_current(Current::from_amps(10.0)),
            ..base_state()
        };

        let darkride_output = advance_loop(config, base_input, state);
        let traction_output = advance_loop(
            config,
            LoopInput {
                traction_control: true,
                ..base_input
            },
            state,
        );

        // Upstream RUNNING flips darkride current at
        // `third_party/refloat/src/main.c:944-946`; traction control freewheels
        // at `third_party/refloat/src/main.c:949-954`.
        assert_current(
            darkride_output.state.balance_current,
            motor_current(Current::from_amps(6.0)),
        );
        assert_current(
            traction_output.state.balance_current,
            motor_current(Current::from_amps(0.0)),
        );
    }
}
