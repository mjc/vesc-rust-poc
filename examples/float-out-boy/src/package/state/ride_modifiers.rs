use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyWheelSlipState,
};
use crate::motor_torque::{MotorTorque, REFLOAT_COMPAT_TORQUE_CONSTANT};
use vescpkg_rs::WrappedAngleMotion;
#[cfg(test)]
use vescpkg_rs::prelude::Current;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, Frequency, MotorCurrent, PidScale, Rpm, SampleRate, VescSeconds,
};
use vescpkg_rs::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};

const LOOP_HERTZ_COMPAT: f32 = 720.0;
const TURN_TILT_YAW_CUTOFF: Frequency = Frequency::from_hertz(25.0);
const TURN_TILT_YAW_RATE_LIMIT: AngularVelocity = AngularVelocity::from_degrees_per_second(72.0);
const TURN_TILT_YAW_RATE_THRESHOLD: AngularVelocity =
    AngularVelocity::from_degrees_per_second(30.0);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct AtrState {
    angle: SmoothSetpoint,
    accel_diff: f32,
    speed_boost: f32,
    transition_target: AngleDegrees,
    transition_boost: SmoothSetpointMultiplier,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct TurnTiltState {
    angle: SmoothSetpoint,
    yaw: WrappedAngleMotion,
}

fn same_source_sign(lhs: AngleDegrees, rhs: AngleDegrees) -> bool {
    // Refloat's `sign` macro returns -1 only for values below zero; both
    // positive and negative IEEE-754 zero therefore belong to the positive
    // branch. Using the unit type keeps that C compatibility rule explicit.
    lhs.is_negative() == rhs.is_negative()
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
    SmoothSetpointMultiplier::from_factor(factor).unwrap_or(SmoothSetpointMultiplier::ONE)
}

fn motor_direction(erpm: Rpm, torque: MotorTorque) -> SmoothSetpointDirection {
    if erpm.abs().as_revolutions_per_minute() > 250.0
        || torque < MotorTorque::from_newton_meters(18.0)
    {
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
    let strength = configured_strength / REFLOAT_COMPAT_TORQUE_CONSTANT.as_newton_meters_per_amp();
    let start_torque = REFLOAT_COMPAT_TORQUE_CONSTANT
        .torque_from_current(config.torque_tilt_start_current().current());
    AngleDegrees::from_degrees(
        ((torque.abs().as_newton_meters() - start_torque.as_newton_meters()).max(0.0) * strength)
            .min(config.torque_tilt_angle_limit().as_degrees())
            * torque.signum(),
    )
}

fn atr_expected_acceleration(torque: MotorTorque, erpm: Rpm, configured_ratio: PidScale) -> f32 {
    let torque = torque.as_newton_meters();
    let abs_torque = torque.abs();
    let compatibility_constant = REFLOAT_COMPAT_TORQUE_CONSTANT.as_newton_meters_per_amp();
    let torque_offset = 8.0 * compatibility_constant;
    let factor = configured_ratio.value() * compatibility_constant;
    if abs_torque < 15.0 {
        (torque - erpm.signum() * torque_offset) / factor
    } else {
        let sign = torque.signum();
        (sign * 15.0 - erpm.signum() * torque_offset) / factor
            + sign * (abs_torque - 15.0) / (factor * 1.3)
    }
}

fn turn_target(
    state: &TurnTiltState,
    config: crate::config::FloatOutBoyBalanceConfig<'_>,
    erpm: Rpm,
) -> AngleDegrees {
    let abs_erpm = erpm.abs().as_revolutions_per_minute();
    let target = if abs_erpm
        < config
            .turn_tilt_start_erpm()
            .rpm()
            .as_revolutions_per_minute()
        || config.turn_tilt_strength().value() == 0.0
        || state.yaw.aggregate().abs() < config.turn_tilt_start_angle()
        || state.yaw.rate().abs() < TURN_TILT_YAW_RATE_THRESHOLD
    {
        0.0
    } else {
        let mut target = state.yaw.rate().abs().as_degrees_per_second() / LOOP_HERTZ_COMPAT
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
            + damper * state.yaw.aggregate().abs().as_degrees()
                / config.turn_tilt_yaw_aggregate().as_degrees())
        .min(2.0);
        target.clamp(
            -config.turn_tilt_angle_limit().as_degrees(),
            config.turn_tilt_angle_limit().as_degrees(),
        ) * erpm.signum()
    };
    AngleDegrees::from_degrees(target)
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

    fn abs_erpm(self) -> f32 {
        self.erpm.abs().as_revolutions_per_minute()
    }
}

impl RideModifierState {
    pub(super) const fn atr_transition_boost(
        &self,
    ) -> crate::domain::FloatOutBoyRealtimeAtrTransitionBoost {
        crate::domain::FloatOutBoyRealtimeAtrTransitionBoost::from_factor(
            self.atr.transition_boost.factor(),
        )
    }

    pub(super) fn reset(&mut self) {
        self.nose = AngleDegrees::ZERO;
        self.torque.reset();
        self.atr.angle.reset();
        self.atr.accel_diff = 0.0;
        self.atr.speed_boost = 0.0;
        self.atr.transition_target = AngleDegrees::ZERO;
        self.atr.transition_boost = SmoothSetpointMultiplier::ONE;
        self.brake.reset();
        self.turn.angle.reset();
        self.turn.yaw = WrappedAngleMotion::default();
    }

    pub(super) fn aggregate_yaw(
        &mut self,
        yaw: AngleDegrees,
        elapsed: VescSeconds,
        filter_rate: SampleRate,
    ) {
        // C map: yaw filtering and aggregation run before the state switch at
        // `third_party/float-out-boy/src/turn_tilt.c:45-72` and
        // `third_party/float-out-boy/src/main.c:800`.
        self.turn.yaw.observe(
            yaw,
            elapsed,
            TURN_TILT_YAW_RATE_LIMIT,
            vescpkg_rs::Ratio::clamped(vescpkg_rs::ema_alpha(TURN_TILT_YAW_CUTOFF, filter_rate)),
            TURN_TILT_YAW_RATE_THRESHOLD,
        );
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
        let Some(frequency) = smooth_setpoint_frequency(elapsed) else {
            if input.wheelslip == FloatOutBoyWheelSlipState::Detected {
                self.wind_down_for_wheelslip();
            }
            return;
        };
        let balance = config.balance();
        self.configure_smooth_setpoints(balance, frequency);
        if input.wheelslip == FloatOutBoyWheelSlipState::Detected {
            self.wind_down_for_wheelslip();
            return;
        }

        let motor = ModifierMotorState::from_input(input);
        self.update_nose(config, input.motor_erpm, elapsed);
        self.update_turn(balance, motor, elapsed);
        self.update_torque(balance, input.filtered_torque, motor, elapsed);
        self.update_atr(balance, input, motor, elapsed, frequency);
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
        self.nose = vescpkg_rs::slew_toward(
            self.nose,
            nose_target(config, erpm),
            vescpkg_rs::angle_step(config.nose_angling_speed(), elapsed),
        );
    }

    fn update_torque(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        torque: MotorTorque,
        motor: ModifierMotorState,
        elapsed: VescSeconds,
    ) {
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
        update_rate: SampleRate,
    ) {
        let ratio = if motor.braking {
            config.atr_amps_decel_ratio()
        } else {
            config.atr_amps_accel_ratio()
        };
        let expected = atr_expected_acceleration(input.filtered_torque, motor.erpm, ratio);
        let forward = motor.direction.is_forward();
        let measured =
            (input.acceleration.as_revolutions_per_minute() / LOOP_HERTZ_COMPAT).clamp(-5.0, 5.0);
        let new_diff = expected - measured;
        let abs_erpm = motor.abs_erpm();
        let cutoff_hertz = if abs_erpm > 2_000.0 {
            1.0
        } else if abs_erpm > 1_000.0 {
            6.0
        } else if abs_erpm > 250.0 {
            10.0
        } else {
            0.0
        };
        let accept = vescpkg_rs::ema_alpha(Frequency::from_hertz(cutoff_hertz), update_rate);
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
        if abs_erpm > 3_000.0 && !motor.braking {
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
        let threshold = if motor.braking {
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
        let target = AngleDegrees::from_degrees(target.clamp(
            -config.atr_angle_limit().as_degrees(),
            config.atr_angle_limit().as_degrees(),
        ));
        let transition_alpha = vescpkg_rs::ema_alpha(Frequency::from_hertz(6.0), update_rate);
        self.atr.transition_target =
            self.atr.transition_target + (target - self.atr.transition_target) * transition_alpha;
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
            && motor.abs_erpm() > 2_000.0
            && balance_offset.as_degrees().is_sign_negative()
                != motor.erpm.signum().is_sign_negative()
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

    fn configure_smooth_setpoints(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        frequency: SampleRate,
    ) {
        let winddown_time_constant = VescSeconds::from_seconds(0.2);
        let time_constant = config.turn_tilt_filter_time_constant();
        let speed_time_constant = VescSeconds::from_seconds(time_constant.as_seconds() * 0.5);
        let turn_speed = AngularVelocity::from_degrees_per_second(20.0);
        self.turn.angle.configure(
            SmoothSetpointConfig::symmetric(
                time_constant,
                speed_time_constant,
                speed_time_constant,
                winddown_time_constant,
                turn_speed,
                turn_speed,
            ),
            frequency,
        );
        self.torque
            .configure(config.torque_tilt_filter(winddown_time_constant), frequency);
        let filter = config.atr_filter(winddown_time_constant);
        self.atr.angle.configure(filter, frequency);
        let off_speed = AngularVelocity::from_degrees_per_second(
            filter.on_speed_up.as_degrees_per_second()
                / config.brake_tilt_lingering().value().max(1.0),
        );
        self.brake
            .configure(filter.with_off_speed(off_speed), frequency);
    }

    pub(super) const fn atr_accel_diff(self) -> f32 {
        self.atr.accel_diff
    }

    pub(super) const fn atr_speed_boost(self) -> f32 {
        self.atr.speed_boost
    }
}

fn smooth_setpoint_frequency(elapsed: VescSeconds) -> Option<SampleRate> {
    let seconds = elapsed.as_seconds();
    (seconds.is_finite() && seconds > 0.0).then(|| SampleRate::from_hertz(1.0 / seconds))
}

#[cfg(test)]
mod tests;
