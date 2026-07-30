use super::smooth_setpoint::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};
use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyRealtimeAtrAccelerationDiff, FloatOutBoyRealtimeAtrSpeedBoost,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, Current, Frequency, MotorCurrent, Rpm, SampleRate, VescSeconds,
};

const LOOP_HERTZ_COMPAT: f32 = 720.0;
const TURN_TILT_YAW_CUTOFF: Frequency = Frequency::from_hertz(25.0);
const TURN_TILT_YAW_RATE_LIMIT: AngularVelocity = AngularVelocity::from_degrees_per_second(72.0);
const TURN_TILT_YAW_RATE_THRESHOLD: AngularVelocity =
    AngularVelocity::from_degrees_per_second(30.0);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct SmoothAngle {
    target: AngleDegrees,
    ramped_step: AngleDegrees,
    setpoint: AngleDegrees,
}

fn loop_step(speed: vescpkg_rs::AngularVelocity, elapsed: VescSeconds) -> AngleDegrees {
    AngleDegrees::from(speed * elapsed)
}

fn smooth_ramp(state: &mut SmoothAngle, target: AngleDegrees, step: AngleDegrees, smoothing: f32) {
    // C map: all four modifier modules use `smooth_rampf` from
    // `third_party/float-out-boy/src/utils.c:36-64` with a 1.5 degree center window.
    state.target = target;
    let diff = target - state.setpoint;
    if diff.abs() < AngleDegrees::from_degrees(1.5) {
        state.ramped_step =
            step * (smoothing * diff.as_degrees() / 2.0) + state.ramped_step * (1.0 - smoothing);
        let centering = state
            .ramped_step
            .abs()
            .min(step * (diff.as_degrees().abs() / 2.0))
            * diff.signum();
        state.setpoint = if diff.abs() < centering.abs() {
            target
        } else {
            state.setpoint + centering
        };
    } else {
        state.ramped_step =
            step * (smoothing * diff.signum()) + state.ramped_step * (1.0 - smoothing);
        state.setpoint = state.setpoint + state.ramped_step;
    }
}

fn rate_limit(value: AngleDegrees, target: AngleDegrees, step: AngleDegrees) -> AngleDegrees {
    let diff = target - value;
    if diff.abs() < step {
        target
    } else {
        value + step * diff.signum()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct AtrState {
    angle: SmoothAngle,
    accel_diff: f32,
    speed_boost: f32,
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

fn atr_step(
    config: crate::config::FloatOutBoyBalanceConfig<'_>,
    target: AngleDegrees,
    forward: bool,
    abs_erpm: f32,
    elapsed: VescSeconds,
    setpoint: AngleDegrees,
) -> AngleDegrees {
    let mut response = 1.0;
    if abs_erpm > 2_500.0 {
        response = config.atr_response_boost().value();
    }
    if abs_erpm > 6_000.0 {
        response *= config.atr_response_boost().value();
    }
    let on = loop_step(config.atr_on_speed(), elapsed);
    let off = loop_step(config.atr_off_speed(), elapsed);
    let mut step = if forward {
        if setpoint.is_negative() {
            if setpoint < target {
                if target.is_positive()
                    && (target - setpoint) > AngleDegrees::from_degrees(2.0)
                    && abs_erpm > 2_000.0
                {
                    off * config.atr_transition_boost().value()
                } else {
                    off
                }
            } else {
                on * response
            }
        } else if target > AngleDegrees::from_degrees(-3.0) && setpoint > target {
            off
        } else {
            on * response
        }
    } else if setpoint.is_positive() {
        if setpoint > target {
            if target.is_negative()
                && (setpoint - target) > AngleDegrees::from_degrees(2.0)
                && abs_erpm > 2_000.0
            {
                off * config.atr_transition_boost().value()
            } else {
                off
            }
        } else {
            on * response
        }
    } else if target < AngleDegrees::from_degrees(3.0) && setpoint < target {
        off
    } else {
        on * response
    };
    if abs_erpm < 500.0 {
        step = step / 2.0;
    }
    step
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
    current: Current,
    braking: bool,
) -> AngleDegrees {
    let strength = if braking {
        config.torque_tilt_regen_strength().value()
    } else {
        config.torque_tilt_strength().value()
    };
    AngleDegrees::from_degrees(
        ((current.as_amps().abs() - config.torque_tilt_start_current().current().as_amps())
            .max(0.0)
            * strength)
            .min(config.torque_tilt_angle_limit().as_degrees())
            * current.signum(),
    )
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
        let mut target = state.yaw.rate.abs().as_degrees_per_second() / LOOP_HERTZ_COMPAT
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
            self.rate.as_degrees_per_second() * (1.0 - alpha) + limited * alpha,
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
    torque: SmoothAngle,
    atr: AtrState,
    brake: SmoothAngle,
    turn: TurnTiltState,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RideModifierInput {
    pub(super) base_setpoint: AngleDegrees,
    pub(super) remote_setpoint: AngleDegrees,
    pub(super) balance_pitch: AngleDegrees,
    pub(super) motor_erpm: Rpm,
    pub(super) filtered_current: Current,
    pub(super) motor_current: MotorCurrent,
    pub(super) acceleration: Rpm,
    pub(super) darkride: bool,
    pub(super) wheelslip: FloatOutBoyWheelSlipState,
}

impl RideModifierState {
    pub(super) fn reset(&mut self) {
        self.nose = AngleDegrees::ZERO;
        self.torque = SmoothAngle::default();
        self.atr = AtrState::default();
        self.brake = SmoothAngle::default();
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
        if matches!(input.wheelslip, FloatOutBoyWheelSlipState::Detected) {
            self.configure_turn_setpoint(elapsed);
            self.wind_down_for_wheelslip();
            return;
        }

        let balance = config.balance();
        let abs_erpm = input.motor_erpm.abs().as_revolutions_per_minute();
        let erpm_sign = input.motor_erpm.signum();
        let braking = input.motor_current.is_negative();
        self.update_nose(config, input.motor_erpm, elapsed);
        self.update_turn(balance, input.motor_erpm, elapsed);
        self.update_torque(balance, input.filtered_current, braking, abs_erpm, elapsed);
        self.update_atr(balance, input, braking, abs_erpm, erpm_sign, elapsed);
        self.update_brake(balance, input, braking, abs_erpm, erpm_sign, elapsed);
    }

    fn wind_down_for_wheelslip(&mut self) {
        // C map: wheelslip freezes nose angling and winds modifier state down
        // at `third_party/float-out-boy/src/main.c:881-887`.
        self.turn.angle.wind_down();
        self.torque.setpoint = self.torque.setpoint * 0.995;
        self.atr.angle.setpoint = self.atr.angle.setpoint * 0.995;
        self.atr.angle.target = self.atr.angle.target * 0.99;
        self.brake.setpoint = self.brake.setpoint * 0.995;
        self.brake.target = self.brake.target * 0.99;
    }

    fn runtime_setpoints(&self, input: RideModifierInput) -> FloatOutBoyRealtimeRuntimeSetpoints {
        let ab = self.atr.angle.setpoint + self.brake.setpoint;
        let combined_torque = combine_torque_offsets(ab, self.torque.setpoint);
        let modifier = if input.darkride {
            AngleDegrees::ZERO
        } else {
            self.nose + self.turn.angle.value() + combined_torque
        };
        let board = input.base_setpoint + input.remote_setpoint + modifier;
        FloatOutBoyRealtimeRuntimeSetpoints::new(
            FloatOutBoyRealtimeRuntimeSetpoint::new(board),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.atr.angle.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.brake.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.torque.setpoint),
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
        current: Current,
        braking: bool,
        abs_erpm: f32,
        elapsed: VescSeconds,
    ) {
        // C map: torque target and on/off ramp selection mirror
        // `third_party/float-out-boy/src/torque_tilt.c:44-82`.
        let target = torque_target(config, current, braking);
        let on = loop_step(config.torque_tilt_on_speed(), elapsed);
        let off = loop_step(config.torque_tilt_off_speed(), elapsed);
        let mut step = if self.torque.setpoint.as_degrees() * target.as_degrees() < 0.0 {
            on.max(off)
        } else if self.torque.setpoint.abs() > target.abs() {
            off
        } else {
            on
        };
        if abs_erpm < 500.0 {
            step = step / 2.0;
        }
        smooth_ramp(&mut self.torque, target, step, 0.04);
    }

    fn update_atr(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        input: RideModifierInput,
        braking: bool,
        abs_erpm: f32,
        erpm_sign: f32,
        elapsed: VescSeconds,
    ) {
        // C map: expected/measured acceleration, speed boost, target filtering,
        // and ramp selection mirror `third_party/float-out-boy/src/atr.c:52-171`.
        let current = input.filtered_current.as_amps();
        let abs_torque = current.abs();
        let ratio = if braking {
            config.atr_amps_decel_ratio().value()
        } else {
            config.atr_amps_accel_ratio().value()
        };
        let expected = if abs_torque < 25.0 {
            (current - erpm_sign * 8.0) / ratio
        } else {
            let sign = current.signum();
            (sign * 25.0 - erpm_sign * 8.0) / ratio + sign * (abs_torque - 25.0) / (ratio * 1.3)
        };
        let forward = if abs_erpm > 250.0 || current < 30.0 {
            !input.motor_erpm.is_negative()
        } else {
            current >= 0.0
        };
        let measured =
            (input.acceleration.as_revolutions_per_minute() / LOOP_HERTZ_COMPAT).clamp(-5.0, 5.0);
        let new_diff = expected - measured;
        let cutoff_hertz = if abs_erpm > 2_000.0 {
            1.0
        } else if abs_erpm > 1_000.0 {
            6.0
        } else if abs_erpm > 250.0 {
            10.0
        } else {
            0.0
        };
        let update_rate = SampleRate::from_hertz(1.0 / elapsed.as_seconds());
        let accept = super::motor_kinematics::refloat_ema_alpha(
            Frequency::from_hertz(cutoff_hertz),
            update_rate,
        );
        self.atr.accel_diff = if accept == 0.0 {
            0.0
        } else {
            self.atr.accel_diff * (1.0 - accept) + new_diff * accept
        };
        let mut strength = if forward == (self.atr.accel_diff > 0.0) {
            config.atr_strength_up().value()
        } else {
            config.atr_strength_down().value()
        };
        if abs_erpm > 3_000.0 && !braking {
            let configured = config.atr_speed_boost().value();
            let divisor = if configured.abs() > 0.4 {
                (configured.abs() - 0.4) * 5_000.0 + 3_000.0
            } else {
                3_000.0
            };
            self.atr.speed_boost = ((abs_erpm - 3_000.0) / divisor).min(1.0) * configured;
            strength += strength * self.atr.speed_boost;
        } else {
            self.atr.speed_boost = 0.0;
        }
        let threshold = if braking {
            config.atr_threshold_down().as_degrees()
        } else {
            config.atr_threshold_up().as_degrees()
        };
        let mut target = strength * self.atr.accel_diff;
        target = if target.abs() < threshold {
            0.0
        } else {
            target - target.signum() * threshold
        };
        let filtered = (self.atr.angle.target.as_degrees() * 0.95 + target * 0.05).clamp(
            -config.atr_angle_limit().as_degrees(),
            config.atr_angle_limit().as_degrees(),
        );
        let target = AngleDegrees::from_degrees(filtered);
        let setpoint = self.atr.angle.setpoint;
        let step = atr_step(config, target, forward, abs_erpm, elapsed, setpoint);
        smooth_ramp(&mut self.atr.angle, target, step, 0.05);
    }

    fn update_brake(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        input: RideModifierInput,
        braking: bool,
        abs_erpm: f32,
        erpm_sign: f32,
        elapsed: VescSeconds,
    ) {
        // C map: braking target, downhill damping, and lingering ramp mirror
        // `third_party/float-out-boy/src/brake_tilt.c:42-91`.
        let strength = config.brake_tilt_strength().value();
        let factor = if strength == 0.0 {
            0.0
        } else {
            -(0.5 + (20.0 - strength) / 5.0)
        };
        let balance_offset = input.base_setpoint + input.remote_setpoint - input.balance_pitch;
        let mut target = AngleDegrees::ZERO;
        if factor < 0.0
            && braking
            && abs_erpm > 2_000.0
            && balance_offset.as_degrees().is_sign_negative() != erpm_sign.is_sign_negative()
        {
            let mut downhill = 1.0;
            if (input.motor_erpm.as_revolutions_per_minute() > 1_000.0
                && self.atr.accel_diff < -1.0)
                || (input.motor_erpm.as_revolutions_per_minute() < -1_000.0
                    && self.atr.accel_diff > 1.0)
            {
                downhill += self.atr.accel_diff.abs() / 2.0;
            }
            if downhill <= 2.0 {
                target = balance_offset / (factor * downhill);
            }
        }
        let on = loop_step(config.atr_on_speed(), elapsed);
        let off = loop_step(config.atr_off_speed(), elapsed);
        let mut step = off / config.brake_tilt_lingering().value().max(1.0);
        if target.abs() > self.brake.setpoint.abs() {
            step = on * 1.5;
        } else if abs_erpm < 800.0 {
            step = on;
        }
        if abs_erpm < 500.0 {
            step = step / 2.0;
        }
        smooth_ramp(&mut self.brake, target, step, 0.05);
    }

    fn update_turn(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        erpm: Rpm,
        elapsed: VescSeconds,
    ) {
        if config.turn_tilt_strength().value() == 0.0 {
            return;
        }
        self.configure_turn_setpoint(elapsed);
        // C map: turn target gates, boosts, direction, and ramp mirror
        // `src/turn_tilt.c` at the pinned Refloat cutoff.
        let target = turn_target(&self.turn, config, erpm);
        self.turn.angle.update(
            target,
            SmoothSetpointDirection::from_erpm(erpm),
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
    }

    fn configure_turn_setpoint(&mut self, elapsed: VescSeconds) {
        let seconds = elapsed.as_seconds();
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
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
            SampleRate::from_hertz(1.0 / seconds),
        );
    }

    pub(super) const fn atr_accel_diff(self) -> FloatOutBoyRealtimeAtrAccelerationDiff {
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(self.atr.accel_diff)
    }

    pub(super) const fn atr_speed_boost(self) -> FloatOutBoyRealtimeAtrSpeedBoost {
        FloatOutBoyRealtimeAtrSpeedBoost::from_units(self.atr.speed_boost)
    }
}

#[cfg(test)]
mod tests;
