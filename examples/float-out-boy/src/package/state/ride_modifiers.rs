use super::smooth_setpoint::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};
use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyRealtimeAtrAccelerationDiff, FloatOutBoyRealtimeAtrSpeedBoost,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyWheelSlipState,
};
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
use core::ops::{Mul, Sub};
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, Current, Frequency, MotorCurrent, PidScale, Rpm, SampleRate,
    VescSeconds,
};

const LOOP_RATE_COMPAT: SampleRate = SampleRate::from_hertz(720.0);
const MOTOR_DIRECTION_ERPM_THRESHOLD: Rpm = Rpm::from_revolutions_per_minute(250.0);
const MOTOR_DIRECTION_TORQUE_THRESHOLD: MotorTorque = MotorTorque::from_newton_meters(18.0);
const ATR_TORQUE_LINEAR_THRESHOLD: MotorTorque = MotorTorque::from_newton_meters(15.0);
const ATR_TORQUE_OFFSET_CURRENT: Current = Current::from_amps(8.0);
const ATR_FILTER_TEN_HZ_MIN_ERPM: Rpm = Rpm::from_revolutions_per_minute(250.0);
const ATR_FILTER_SIX_HZ_MIN_ERPM: Rpm = Rpm::from_revolutions_per_minute(1_000.0);
const ATR_FILTER_ONE_HZ_MIN_ERPM: Rpm = Rpm::from_revolutions_per_minute(2_000.0);
const ATR_SPEED_BOOST_START_ERPM: Rpm = Rpm::from_revolutions_per_minute(3_000.0);
const ATR_SPEED_BOOST_BASE_RANGE: Rpm = Rpm::from_revolutions_per_minute(3_000.0);
const ATR_SPEED_BOOST_EXTRA_RANGE: Rpm = Rpm::from_revolutions_per_minute(5_000.0);
const BRAKE_TILT_MIN_ERPM: Rpm = Rpm::from_revolutions_per_minute(2_000.0);
const BRAKE_TILT_DOWNHILL_ERPM: Rpm = Rpm::from_revolutions_per_minute(1_000.0);
const BRAKE_TILT_DOWNHILL_REVERSE_ERPM: Rpm = Rpm::from_revolutions_per_minute(-1_000.0);
const TURN_TILT_YAW_CUTOFF: Frequency = Frequency::from_hertz(25.0);
const TURN_TILT_YAW_RATE_LIMIT: AngularVelocity = AngularVelocity::from_degrees_per_second(72.0);
const TURN_TILT_YAW_RATE_THRESHOLD: AngularVelocity =
    AngularVelocity::from_degrees_per_second(30.0);

fn loop_step(speed: vescpkg_rs::AngularVelocity, elapsed: VescSeconds) -> AngleDegrees {
    AngleDegrees::from(speed * elapsed)
}

fn rate_limit(value: AngleDegrees, target: AngleDegrees, step: AngleDegrees) -> AngleDegrees {
    let diff = target - value;
    if diff.abs() < step {
        target
    } else {
        value + step * diff.signum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AtrState {
    angle: SmoothSetpoint,
    accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff,
    speed_boost: FloatOutBoyRealtimeAtrSpeedBoost,
    transition_target: AngleDegrees,
    transition_boost: SmoothSetpointMultiplier,
}

impl Default for AtrState {
    fn default() -> Self {
        Self {
            angle: SmoothSetpoint::default(),
            accel_diff: FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
            speed_boost: FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0),
            transition_target: AngleDegrees::ZERO,
            transition_boost: SmoothSetpointMultiplier::ONE,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct YawMotion {
    last: AngleDegrees,
    rate: AngularVelocity,
    aggregate: AngleDegrees,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct TurnTiltState {
    angle: SmoothSetpoint,
    yaw: YawMotion,
}

fn same_source_sign(lhs: AngleDegrees, rhs: AngleDegrees) -> bool {
    // Refloat's `sign` macro returns -1 only for values below zero; both
    // positive and negative IEEE-754 zero therefore belong to the positive
    // branch. Using the unit type keeps that C compatibility rule explicit.
    lhs.is_negative() == rhs.is_negative()
}

fn wrapped_yaw_delta(yaw: AngleDegrees, previous: AngleDegrees) -> AngleDegrees {
    let change = yaw - previous;
    if change < AngleDegrees::from_degrees(-180.0) {
        change + AngleDegrees::from_degrees(360.0)
    } else if change > AngleDegrees::from_degrees(180.0) {
        change - AngleDegrees::from_degrees(360.0)
    } else {
        change
    }
}

fn combine_torque_offsets(ab: AngleDegrees, torque: AngleDegrees) -> AngleDegrees {
    if same_source_sign(ab, torque) {
        AngleDegrees::from_degrees(
            ab.signum() * ab.as_degrees().abs().max(torque.as_degrees().abs()),
        )
    } else {
        ab + torque
    }
}

fn atr_transition_multiplier(
    setpoint: AngleDegrees,
    transition_target: AngleDegrees,
    configured: PidScale,
) -> SmoothSetpointMultiplier {
    let degrees_diff = (setpoint - transition_target).abs().as_degrees() - 1.0;
    let factor =
        if setpoint.as_degrees() * transition_target.as_degrees() < 0.0 && degrees_diff > 0.0 {
            1.0 + degrees_diff.min(1.0) * (configured.value() - 1.0)
        } else {
            1.0
        };
    SmoothSetpointMultiplier::from_factor(factor)
}

fn motor_direction(erpm: Rpm, torque: MotorTorque) -> SmoothSetpointDirection {
    if erpm.abs() > MOTOR_DIRECTION_ERPM_THRESHOLD || torque < MOTOR_DIRECTION_TORQUE_THRESHOLD {
        SmoothSetpointDirection::from_erpm(erpm)
    } else {
        SmoothSetpointDirection::from_forward(!torque.is_negative())
    }
}

fn nose_target(config: &FloatOutBoyConfigImage, erpm: Rpm) -> AngleDegrees {
    let abs_erpm = erpm.abs().as_revolutions_per_minute();
    let variable_rate =
        config.tiltback_variable().value() / 1_000.0 * config.tiltback_variable_max().signum();
    let variable_max_erpm = if variable_rate == 0.0 {
        0.0
    } else {
        (config.tiltback_variable_max().as_degrees() / variable_rate).abs()
    };
    let variable_erpm = (abs_erpm
        - config
            .tiltback_variable_erpm()
            .rpm()
            .as_revolutions_per_minute())
    .clamp(0.0, variable_max_erpm);
    let mut target = variable_rate * variable_erpm * erpm.signum();
    if abs_erpm
        > config
            .tiltback_constant_erpm()
            .rpm()
            .as_revolutions_per_minute()
    {
        target += config.tiltback_constant().as_degrees() * erpm.signum();
    }
    AngleDegrees::from_degrees(target)
}

fn torque_target(
    config: crate::config::FloatOutBoyBalanceConfig<'_>,
    torque: MotorTorque,
    braking: bool,
) -> AngleDegrees {
    let configured_strength = if braking {
        config.torque_tilt_regen_strength().value()
    } else {
        config.torque_tilt_strength().value()
    };
    let start_torque = MotorTorqueConstant::REFLOAT_COMPAT
        .torque_from_current(config.torque_tilt_start_current().current());
    let excess_torque = torque.abs().sub(start_torque).max(MotorTorque::ZERO);
    let equivalent_current = MotorTorqueConstant::REFLOAT_COMPAT.current_from_torque(excess_torque);
    AngleDegrees::from_degrees(
        (equivalent_current.as_amps() * configured_strength)
            .min(config.torque_tilt_angle_limit().as_degrees())
            * torque.signum().as_ratio(),
    )
}

fn atr_expected_acceleration(
    torque: MotorTorque,
    erpm: Rpm,
    configured_ratio: PidScale,
) -> FloatOutBoyRealtimeAtrAccelerationDiff {
    let compatibility_constant = MotorTorqueConstant::REFLOAT_COMPAT;
    let torque_offset = compatibility_constant.torque_from_current(ATR_TORQUE_OFFSET_CURRENT);
    let expected = if torque.abs() < ATR_TORQUE_LINEAR_THRESHOLD {
        let adjusted_torque = torque.sub(torque_offset.mul(erpm.signum()));
        compatibility_constant
            .current_from_torque(adjusted_torque)
            .as_amps()
            / configured_ratio.value()
    } else {
        let sign = torque.signum().as_ratio();
        let linear_torque = ATR_TORQUE_LINEAR_THRESHOLD
            .mul(sign)
            .sub(torque_offset.mul(erpm.signum()));
        let nonlinear_torque = torque.abs().sub(ATR_TORQUE_LINEAR_THRESHOLD).mul(sign);
        compatibility_constant
            .current_from_torque(linear_torque)
            .as_amps()
            / configured_ratio.value()
            + compatibility_constant
                .current_from_torque(nonlinear_torque)
                .as_amps()
                / (configured_ratio.value() * 1.3)
    };
    FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(expected)
}

fn turn_target(
    state: &TurnTiltState,
    config: crate::config::FloatOutBoyBalanceConfig<'_>,
    erpm: Rpm,
) -> AngleDegrees {
    let abs_erpm = erpm.abs().as_revolutions_per_minute();
    let mut target = if config.turn_tilt_strength().value() == 0.0
        || state.yaw.aggregate.abs() < config.turn_tilt_start_angle()
        || state.yaw.rate.abs() < TURN_TILT_YAW_RATE_THRESHOLD
    {
        0.0
    } else {
        let mut target = state.yaw.rate.abs().as_degrees_per_second() / LOOP_RATE_COMPAT.as_hertz()
            * config.turn_tilt_strength().value();
        let boost = if abs_erpm
            < config
                .turn_tilt_erpm_boost_end()
                .as_revolutions_per_minute()
        {
            1.0 + abs_erpm * f32::from(config.turn_tilt_erpm_boost())
                / 100.0
                / config
                    .turn_tilt_erpm_boost_end()
                    .as_revolutions_per_minute()
        } else {
            1.0 + f32::from(config.turn_tilt_erpm_boost()) / 100.0
        };
        target *= boost;
        let damper = if abs_erpm < 2_000.0 { 0.5 } else { 1.0 };
        target *= (1.0
            + damper * state.yaw.aggregate.abs().as_degrees()
                / config.turn_tilt_yaw_aggregate().as_degrees())
        .min(2.0);
        target.clamp(
            -config.turn_tilt_angle_limit().as_degrees(),
            config.turn_tilt_angle_limit().as_degrees(),
        )
    };
    if abs_erpm
        < config
            .turn_tilt_start_erpm()
            .rpm()
            .as_revolutions_per_minute()
    {
        target = 0.0;
    } else {
        target *= erpm.signum();
    }
    AngleDegrees::from_degrees(target)
}

impl YawMotion {
    fn observe(&mut self, yaw: AngleDegrees, elapsed: VescSeconds, filter_rate: SampleRate) {
        // C map: yaw filtering and aggregation run before the state switch at
        // `third_party/float-out-boy/src/turn_tilt.c:45-72` and
        // `third_party/float-out-boy/src/main.c:800`.
        let change = wrapped_yaw_delta(yaw, self.last);
        self.last = yaw;
        let seconds = elapsed.as_seconds();
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        let limit = TURN_TILT_YAW_RATE_LIMIT.as_degrees_per_second();
        let limited = (change.as_degrees() / seconds).clamp(-limit, limit);
        let alpha = super::motor_kinematics::refloat_ema_alpha(TURN_TILT_YAW_CUTOFF, filter_rate);
        self.rate = AngularVelocity::from_degrees_per_second(
            self.rate.as_degrees_per_second() * (1.0 - alpha.as_ratio())
                + limited * alpha.as_ratio(),
        );
        if self.rate.is_negative() != self.aggregate.is_negative() {
            self.aggregate = AngleDegrees::ZERO;
        }
        if self.rate.abs() > TURN_TILT_YAW_RATE_THRESHOLD {
            self.aggregate = self.aggregate + change;
        }
    }
}

impl TurnTiltState {
    fn aggregate(&mut self, yaw: AngleDegrees, elapsed: VescSeconds, filter_rate: SampleRate) {
        self.yaw.observe(yaw, elapsed, filter_rate);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct RideModifierState {
    nose: AngleDegrees,
    torque: SmoothSetpoint,
    atr: AtrState,
    brake: SmoothSetpoint,
    turn: TurnTiltState,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RideModifierInput {
    pub(super) base_setpoint: AngleDegrees,
    pub(super) remote_setpoint: AngleDegrees,
    pub(super) balance_pitch: AngleDegrees,
    pub(super) motor_erpm: Rpm,
    pub(super) filtered_torque: MotorTorque,
    pub(super) motor_current: MotorCurrent,
    pub(super) acceleration: Rpm,
    pub(super) darkride: bool,
    pub(super) wheelslip: FloatOutBoyWheelSlipState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModifierMotorState {
    erpm: Rpm,
    direction: SmoothSetpointDirection,
    braking: bool,
}

impl ModifierMotorState {
    fn from_input(input: RideModifierInput) -> Self {
        Self {
            erpm: input.motor_erpm,
            direction: motor_direction(input.motor_erpm, input.filtered_torque),
            braking: input.motor_current.is_negative(),
        }
    }

    fn abs_erpm(self) -> Rpm {
        self.erpm.abs()
    }
}

impl RideModifierState {
    pub(super) fn reset(&mut self) {
        self.nose = AngleDegrees::ZERO;
        self.torque.reset();
        self.atr.angle.reset();
        self.atr.accel_diff = FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0);
        self.atr.speed_boost = FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0);
        self.atr.transition_target = AngleDegrees::ZERO;
        self.atr.transition_boost = SmoothSetpointMultiplier::ONE;
        self.brake.reset();
        self.turn.angle.reset();
        self.turn.yaw = YawMotion::default();
    }

    pub(super) fn aggregate_yaw(
        &mut self,
        yaw: AngleDegrees,
        elapsed: VescSeconds,
        filter_rate: SampleRate,
    ) {
        self.turn.aggregate(yaw, elapsed, filter_rate);
    }

    #[cfg(test)]
    pub(super) fn advance(
        &mut self,
        config: &FloatOutBoyConfigImage,
        input: RideModifierInput,
    ) -> FloatOutBoyRealtimeRuntimeSetpoints {
        let elapsed = config
            .startup()
            .sample_rate()
            .sample_period()
            .unwrap_or(VescSeconds::ZERO);
        self.advance_elapsed(config, input, elapsed)
    }

    pub(super) fn advance_elapsed(
        &mut self,
        config: &FloatOutBoyConfigImage,
        input: RideModifierInput,
        elapsed: VescSeconds,
    ) -> FloatOutBoyRealtimeRuntimeSetpoints {
        self.advance_modifiers(config, input, elapsed);
        self.runtime_setpoints(input)
    }

    fn advance_modifiers(
        &mut self,
        config: &FloatOutBoyConfigImage,
        input: RideModifierInput,
        elapsed: VescSeconds,
    ) {
        if input.darkride {
            return;
        }
        if input.wheelslip == FloatOutBoyWheelSlipState::Detected {
            self.configure_smooth_setpoints(config.balance(), elapsed);
            self.wind_down_for_wheelslip();
            return;
        }

        let balance = config.balance();
        let motor = ModifierMotorState::from_input(input);
        self.update_nose(config, input.motor_erpm, elapsed);
        self.update_turn(balance, motor, elapsed);
        self.update_torque(balance, input.filtered_torque, motor, elapsed);
        self.update_atr(balance, input, motor, elapsed);
        self.update_brake(balance, input, motor, elapsed);
    }

    fn wind_down_for_wheelslip(&mut self) {
        // C map: each cutoff modifier delegates wheelslip decay to SmoothSetpoint.
        self.turn.angle.wind_down();
        self.torque.wind_down();
        self.atr.angle.wind_down();
        self.atr.transition_target = self.atr.angle.value();
        self.atr.transition_boost = SmoothSetpointMultiplier::ONE;
        self.brake.wind_down();
    }

    fn runtime_setpoints(&self, input: RideModifierInput) -> FloatOutBoyRealtimeRuntimeSetpoints {
        let ab = self.atr.angle.value() + self.brake.value();
        let combined_torque = combine_torque_offsets(ab, self.torque.value());
        let modifier = if input.darkride {
            AngleDegrees::ZERO
        } else {
            self.nose + self.turn.angle.value() + combined_torque
        };
        let board = input.base_setpoint + input.remote_setpoint + modifier;
        FloatOutBoyRealtimeRuntimeSetpoints::new(
            FloatOutBoyRealtimeRuntimeSetpoint::new(board),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.atr.angle.value()),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.brake.value()),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.torque.value()),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.turn.angle.value()),
            FloatOutBoyRealtimeRuntimeSetpoint::new(input.remote_setpoint),
        )
    }

    fn update_nose(&mut self, config: &FloatOutBoyConfigImage, erpm: Rpm, elapsed: VescSeconds) {
        // C map: constant/variable nose target and rate limit mirror
        // `third_party/float-out-boy/src/main.c:746-758` and configuration at `:165-173`.
        self.nose = rate_limit(
            self.nose,
            nose_target(config, erpm),
            loop_step(config.nose_angling_speed(), elapsed),
        );
    }

    fn update_torque(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        torque: MotorTorque,
        motor: ModifierMotorState,
        elapsed: VescSeconds,
    ) {
        self.configure_torque_setpoint(config, elapsed);
        self.torque.update(
            torque_target(config, torque, motor.braking),
            motor.direction,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
    }

    fn update_atr(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        input: RideModifierInput,
        motor: ModifierMotorState,
        elapsed: VescSeconds,
    ) {
        let Some(update_rate) = smooth_setpoint_frequency(elapsed) else {
            return;
        };
        self.configure_atr_setpoint(config, elapsed);
        let ratio = if motor.braking {
            config.atr_amps_decel_ratio()
        } else {
            config.atr_amps_accel_ratio()
        };
        let expected =
            atr_expected_acceleration(input.filtered_torque, motor.erpm, ratio).as_erpm_delta();
        let forward = motor.direction.is_forward();
        let measured = (input.acceleration.as_revolutions_per_minute()
            / LOOP_RATE_COMPAT.as_hertz())
        .clamp(-5.0, 5.0);
        let new_diff = expected - measured;
        let abs_erpm = motor.abs_erpm();
        let cutoff_hertz = if abs_erpm > ATR_FILTER_ONE_HZ_MIN_ERPM {
            1.0
        } else if abs_erpm > ATR_FILTER_SIX_HZ_MIN_ERPM {
            6.0
        } else if abs_erpm > ATR_FILTER_TEN_HZ_MIN_ERPM {
            10.0
        } else {
            0.0
        };
        let accept = super::motor_kinematics::refloat_ema_alpha(
            Frequency::from_hertz(cutoff_hertz),
            update_rate,
        );
        let accel_diff = if accept.is_zero() {
            0.0
        } else {
            self.atr.accel_diff.as_erpm_delta() * (1.0 - accept.as_ratio())
                + new_diff * accept.as_ratio()
        };
        self.atr.accel_diff = FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(accel_diff);
        let mut strength = if forward == (accel_diff > 0.0) {
            config.atr_strength_up().value()
        } else {
            config.atr_strength_down().value()
        };
        if abs_erpm > ATR_SPEED_BOOST_START_ERPM && !motor.braking {
            let configured = config.atr_speed_boost().value();
            let divisor = if configured.abs() > 0.4 {
                ATR_SPEED_BOOST_EXTRA_RANGE * (configured.abs() - 0.4) + ATR_SPEED_BOOST_BASE_RANGE
            } else {
                ATR_SPEED_BOOST_BASE_RANGE
            };
            let speed_boost =
                ((abs_erpm - ATR_SPEED_BOOST_START_ERPM) / divisor).min(1.0) * configured;
            self.atr.speed_boost = FloatOutBoyRealtimeAtrSpeedBoost::from_units(speed_boost);
            strength += strength * speed_boost;
        } else {
            self.atr.speed_boost = FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0);
        }
        let threshold = if motor.braking {
            config.atr_threshold_down().as_degrees()
        } else {
            config.atr_threshold_up().as_degrees()
        };
        let mut target = strength * accel_diff;
        target = if target.abs() < threshold {
            0.0
        } else {
            target - target.signum() * threshold
        };
        let target = AngleDegrees::from_degrees(target.clamp(
            -config.atr_angle_limit().as_degrees(),
            config.atr_angle_limit().as_degrees(),
        ));
        let transition_alpha =
            super::motor_kinematics::refloat_ema_alpha(Frequency::from_hertz(6.0), update_rate);
        self.atr.transition_target = self.atr.transition_target
            + (target - self.atr.transition_target) * transition_alpha.as_ratio();
        self.atr.transition_boost = atr_transition_multiplier(
            self.atr.angle.value(),
            self.atr.transition_target,
            config.atr_transition_boost(),
        );
        self.atr.angle.update(
            target,
            SmoothSetpointDirection::from_forward(forward),
            self.atr.transition_boost,
            elapsed,
        );
    }

    fn update_brake(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        input: RideModifierInput,
        motor: ModifierMotorState,
        elapsed: VescSeconds,
    ) {
        let accel_diff = self.atr.accel_diff.as_erpm_delta();
        self.configure_brake_setpoint(config, elapsed);
        let strength = config.brake_tilt_strength().value();
        let factor = if strength == 0.0 {
            0.0
        } else {
            -(0.5 + (20.0 - strength) / 5.0)
        };
        let balance_offset = input.base_setpoint + input.remote_setpoint - input.balance_pitch;
        let mut target = AngleDegrees::ZERO;
        if factor < 0.0
            && motor.braking
            && motor.abs_erpm() > BRAKE_TILT_MIN_ERPM
            && balance_offset.as_degrees().is_sign_negative()
                != motor.erpm.signum().is_sign_negative()
        {
            let mut downhill = 1.0;
            if (input.motor_erpm > BRAKE_TILT_DOWNHILL_ERPM && accel_diff < -1.0)
                || (input.motor_erpm < BRAKE_TILT_DOWNHILL_REVERSE_ERPM && accel_diff > 1.0)
            {
                downhill += accel_diff.abs() / 2.0;
            }
            if downhill <= 2.0 {
                target = balance_offset / (factor * downhill);
            }
        }
        self.brake.update(
            target,
            motor.direction,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
    }

    fn update_turn(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        motor: ModifierMotorState,
        elapsed: VescSeconds,
    ) {
        if config.turn_tilt_strength().value() == 0.0 {
            return;
        }
        self.configure_turn_setpoint(elapsed);
        // C map: turn target gates, boosts, direction, and ramp mirror
        // `src/turn_tilt.c` at the pinned Refloat cutoff.
        let target = turn_target(&self.turn, config, motor.erpm);
        self.turn.angle.update(
            target,
            motor.direction,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
    }

    fn configure_turn_setpoint(&mut self, elapsed: VescSeconds) {
        let Some(frequency) = smooth_setpoint_frequency(elapsed) else {
            return;
        };
        // Current Refloat removed the legacy serialized turn-tilt speed and
        // configures its SmoothSetpoint at a fixed 20 degrees/second.
        self.turn.angle.configure(
            SmoothSetpointConfig {
                time_constant: VescSeconds::from_seconds(0.2),
                on_speed_time_constant: VescSeconds::from_seconds(0.1),
                off_speed_time_constant: VescSeconds::from_seconds(0.1),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: AngularVelocity::from_degrees_per_second(20.0),
                off_speed_up: AngularVelocity::from_degrees_per_second(20.0),
                on_speed_down: AngularVelocity::from_degrees_per_second(20.0),
                off_speed_down: AngularVelocity::from_degrees_per_second(20.0),
            },
            frequency,
        );
    }

    fn configure_torque_setpoint(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        elapsed: VescSeconds,
    ) {
        let Some(frequency) = smooth_setpoint_frequency(elapsed) else {
            return;
        };
        self.torque.configure(
            SmoothSetpointConfig {
                time_constant: VescSeconds::from_seconds(0.2),
                on_speed_time_constant: VescSeconds::from_seconds(0.08),
                off_speed_time_constant: VescSeconds::from_seconds(0.16),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: config.torque_tilt_on_speed(),
                off_speed_up: config.torque_tilt_off_speed(),
                on_speed_down: config.torque_tilt_on_speed(),
                off_speed_down: config.torque_tilt_off_speed(),
            },
            frequency,
        );
    }

    fn configure_atr_setpoint(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        elapsed: VescSeconds,
    ) {
        let Some(frequency) = smooth_setpoint_frequency(elapsed) else {
            return;
        };
        self.atr.angle.configure(
            SmoothSetpointConfig {
                time_constant: VescSeconds::from_seconds(0.3),
                on_speed_time_constant: VescSeconds::from_seconds(0.1),
                off_speed_time_constant: VescSeconds::from_seconds(0.01),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: config.atr_on_speed(),
                off_speed_up: config.atr_off_speed(),
                on_speed_down: config.atr_on_speed(),
                off_speed_down: config.atr_off_speed(),
            },
            frequency,
        );
    }

    fn configure_brake_setpoint(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        elapsed: VescSeconds,
    ) {
        let Some(frequency) = smooth_setpoint_frequency(elapsed) else {
            return;
        };
        let off_speed = config.atr_off_speed() / config.brake_tilt_lingering().value().max(1.0);
        self.brake.configure(
            SmoothSetpointConfig {
                time_constant: VescSeconds::from_seconds(0.3),
                on_speed_time_constant: VescSeconds::from_seconds(0.1),
                off_speed_time_constant: VescSeconds::from_seconds(0.01),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: config.atr_on_speed(),
                off_speed_up: off_speed,
                on_speed_down: config.atr_on_speed(),
                off_speed_down: off_speed,
            },
            frequency,
        );
    }

    fn configure_smooth_setpoints(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        elapsed: VescSeconds,
    ) {
        self.configure_turn_setpoint(elapsed);
        self.configure_torque_setpoint(config, elapsed);
        self.configure_atr_setpoint(config, elapsed);
        self.configure_brake_setpoint(config, elapsed);
    }

    pub(super) const fn atr_accel_diff(self) -> FloatOutBoyRealtimeAtrAccelerationDiff {
        self.atr.accel_diff
    }

    pub(super) const fn atr_speed_boost(self) -> FloatOutBoyRealtimeAtrSpeedBoost {
        self.atr.speed_boost
    }
}

fn smooth_setpoint_frequency(elapsed: VescSeconds) -> Option<SampleRate> {
    let seconds = elapsed.as_seconds();
    (seconds.is_finite() && seconds > 0.0).then(|| SampleRate::from_hertz(1.0 / seconds))
}

#[cfg(test)]
mod tests;
