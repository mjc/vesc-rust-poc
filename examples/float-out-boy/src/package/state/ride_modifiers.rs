use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyWheelSlipState,
};
use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent, Rpm, SampleRate};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct SmoothAngle {
    target: AngleDegrees,
    ramped_step: AngleDegrees,
    setpoint: AngleDegrees,
}

fn loop_step(speed: vescpkg_rs::AngularVelocity, sample_rate: SampleRate) -> AngleDegrees {
    sample_rate
        .sample_period()
        .map_or(AngleDegrees::ZERO, |period| {
            AngleDegrees::from(speed * period)
        })
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
struct TurnTiltState {
    angle: SmoothAngle,
    last_yaw: AngleDegrees,
    last_yaw_change: AngleDegrees,
    yaw_change: AngleDegrees,
    abs_yaw_change: AngleDegrees,
    yaw_aggregate: AngleDegrees,
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

fn atr_step(
    config: crate::config::FloatOutBoyBalanceConfig<'_>,
    target: AngleDegrees,
    forward: bool,
    abs_erpm: f32,
    sample_rate: SampleRate,
    setpoint: AngleDegrees,
) -> AngleDegrees {
    let mut response = 1.0;
    if abs_erpm > 2_500.0 {
        response = config.atr_response_boost().value();
    }
    if abs_erpm > 6_000.0 {
        response *= config.atr_response_boost().value();
    }
    let on = loop_step(config.atr_on_speed(), sample_rate);
    let off = loop_step(config.atr_off_speed(), sample_rate);
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
        || state.yaw_aggregate.abs() < config.turn_tilt_start_angle()
        || state.abs_yaw_change < AngleDegrees::from_degrees(0.04)
    {
        0.0
    } else {
        let mut target = state.abs_yaw_change.as_degrees() * config.turn_tilt_strength().value();
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
            + damper * state.yaw_aggregate.abs().as_degrees()
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

impl TurnTiltState {
    fn aggregate(&mut self, yaw: AngleDegrees) {
        // C map: yaw filtering and aggregation run before the state switch at
        // `third_party/float-out-boy/src/turn_tilt.c:45-72` and
        // `third_party/float-out-boy/src/main.c:800`.
        let mut change = yaw - self.last_yaw;
        let unchanged = change.is_zero() || change.abs() > AngleDegrees::from_degrees(100.0);
        if unchanged {
            change = self.last_yaw_change;
        }
        self.last_yaw_change = change;
        self.last_yaw = yaw;
        let limited = AngleDegrees::from_degrees(change.as_degrees().clamp(-0.10, 0.10));
        self.yaw_change = self.yaw_change * 0.8 + limited * 0.2;
        if !same_source_sign(self.yaw_change, self.yaw_aggregate) {
            self.yaw_aggregate = AngleDegrees::ZERO;
        }
        self.abs_yaw_change = self.yaw_change.abs();
        if self.abs_yaw_change > AngleDegrees::from_degrees(0.04) && !unchanged {
            self.yaw_aggregate = self.yaw_aggregate + self.yaw_change;
        }
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
        *self = Self::default();
    }

    pub(super) fn aggregate_yaw(&mut self, yaw: AngleDegrees) {
        self.turn.aggregate(yaw);
    }

    pub(super) fn advance(
        &mut self,
        config: &FloatOutBoyConfigImage,
        input: RideModifierInput,
    ) -> FloatOutBoyRealtimeRuntimeSetpoints {
        self.advance_modifiers(config, input);
        self.runtime_setpoints(input)
    }

    fn advance_modifiers(&mut self, config: &FloatOutBoyConfigImage, input: RideModifierInput) {
        if input.darkride {
            return;
        }
        if matches!(input.wheelslip, FloatOutBoyWheelSlipState::Detected) {
            self.wind_down_for_wheelslip();
            return;
        }

        let balance = config.balance();
        let sample_rate = config.startup().sample_rate();
        let abs_erpm = input.motor_erpm.abs().as_revolutions_per_minute();
        let erpm_sign = input.motor_erpm.signum();
        let braking = input.motor_current.is_negative();
        self.update_nose(config, input.motor_erpm, sample_rate);
        self.update_turn(balance, input.motor_erpm, sample_rate);
        self.update_torque(
            balance,
            input.filtered_current,
            braking,
            abs_erpm,
            sample_rate,
        );
        self.update_atr(balance, input, braking, abs_erpm, erpm_sign, sample_rate);
        self.update_brake(balance, input, braking, abs_erpm, erpm_sign, sample_rate);
    }

    fn wind_down_for_wheelslip(&mut self) {
        // C map: wheelslip freezes nose angling and winds modifier state down
        // at `third_party/float-out-boy/src/main.c:881-887`.
        self.turn.angle.setpoint = self.turn.angle.setpoint * 0.995;
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
            self.nose + self.turn.angle.setpoint + combined_torque
        };
        let board = input.base_setpoint + input.remote_setpoint + modifier;
        FloatOutBoyRealtimeRuntimeSetpoints::new(
            FloatOutBoyRealtimeRuntimeSetpoint::new(board),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.atr.angle.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.brake.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.torque.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(self.turn.angle.setpoint),
            FloatOutBoyRealtimeRuntimeSetpoint::new(input.remote_setpoint),
        )
    }

    fn update_nose(&mut self, config: &FloatOutBoyConfigImage, erpm: Rpm, sample_rate: SampleRate) {
        // C map: constant/variable nose target and rate limit mirror
        // `third_party/float-out-boy/src/main.c:746-758` and configuration at `:165-173`.
        self.nose = rate_limit(
            self.nose,
            nose_target(config, erpm),
            loop_step(config.nose_angling_speed(), sample_rate),
        );
    }

    fn update_torque(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        current: Current,
        braking: bool,
        abs_erpm: f32,
        sample_rate: SampleRate,
    ) {
        // C map: torque target and on/off ramp selection mirror
        // `third_party/float-out-boy/src/torque_tilt.c:44-82`.
        let target = torque_target(config, current, braking);
        let on = loop_step(config.torque_tilt_on_speed(), sample_rate);
        let off = loop_step(config.torque_tilt_off_speed(), sample_rate);
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
        sample_rate: SampleRate,
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
        let mut forward = input.motor_erpm.is_positive();
        if abs_erpm < 250.0 && abs_torque > 30.0 {
            forward = expected > 0.0;
        }
        let new_diff = expected
            - input
                .acceleration
                .as_revolutions_per_minute()
                .clamp(-5.0, 5.0);
        let accept = if abs_erpm > 2_000.0 {
            0.1
        } else if abs_erpm > 1_000.0 {
            0.05
        } else if abs_erpm > 250.0 {
            0.02
        } else {
            0.0
        };
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
        let step = atr_step(config, target, forward, abs_erpm, sample_rate, setpoint);
        smooth_ramp(&mut self.atr.angle, target, step, 0.05);
    }

    fn update_brake(
        &mut self,
        config: crate::config::FloatOutBoyBalanceConfig<'_>,
        input: RideModifierInput,
        braking: bool,
        abs_erpm: f32,
        erpm_sign: f32,
        sample_rate: SampleRate,
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
        let on = loop_step(config.atr_on_speed(), sample_rate);
        let off = loop_step(config.atr_off_speed(), sample_rate);
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
        sample_rate: SampleRate,
    ) {
        // C map: turn target gates, boosts, direction, and ramp mirror
        // `third_party/float-out-boy/src/turn_tilt.c:74-130`.
        let target = turn_target(&self.turn, config, erpm);
        smooth_ramp(
            &mut self.turn.angle,
            target,
            loop_step(config.turn_tilt_speed(), sample_rate),
            0.04,
        );
    }

    pub(super) const fn atr_accel_diff(self) -> f32 {
        self.atr.accel_diff
    }

    pub(super) const fn atr_speed_boost(self) -> f32 {
        self.atr.speed_boost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::prelude::{ElectricalSpeed, PidScale};

    fn input() -> RideModifierInput {
        RideModifierInput {
            base_setpoint: AngleDegrees::ZERO,
            remote_setpoint: AngleDegrees::ZERO,
            balance_pitch: AngleDegrees::ZERO,
            motor_erpm: Rpm::from_revolutions_per_minute(3_000.0),
            filtered_current: Current::ZERO,
            motor_current: MotorCurrent::new(Current::ZERO),
            acceleration: Rpm::ZERO,
            darkride: false,
            wheelslip: FloatOutBoyWheelSlipState::None,
        }
    }

    #[test]
    fn turn_tilt_uses_filtered_yaw_and_erpm_direction_like_float_out_boy() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_turn_tilt_strength(PidScale::new(5.0)));
        assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(10.0)));
        assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
            Rpm::from_revolutions_per_minute(1_000.0),
        )));

        let mut state = RideModifierState::default();
        for tick in 1..100 {
            let tick = i16::try_from(tick).unwrap_or(i16::MAX);
            state.aggregate_yaw(AngleDegrees::from_degrees(f32::from(tick) * 0.1));
            state.advance(&config, input());
        }
        state.aggregate_yaw(AngleDegrees::from_degrees(10.0));
        let setpoints = state.advance(&config, input());

        assert!(setpoints.turn_tilt().angle().is_positive());
        assert_eq!(setpoints.board().angle(), setpoints.turn_tilt().angle());
    }

    #[test]
    fn disabling_turn_tilt_winds_down_an_existing_setpoint() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_turn_tilt_strength(PidScale::new(5.0)));
        assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(10.0)));
        assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
            Rpm::from_revolutions_per_minute(1_000.0),
        )));

        let mut state = RideModifierState::default();
        for tick in 1..100 {
            let tick = i16::try_from(tick).unwrap_or(i16::MAX);
            state.aggregate_yaw(AngleDegrees::from_degrees(f32::from(tick) * 0.1));
            state.advance(&config, input());
        }
        state.aggregate_yaw(AngleDegrees::from_degrees(10.0));
        let active = state.advance(&config, input()).turn_tilt().angle();
        assert!(active.is_positive());

        assert!(config.editor().set_turn_tilt_strength(PidScale::new(0.0)));
        let disabled = state.advance(&config, input()).turn_tilt().angle();

        assert!(disabled < active);
    }

    #[test]
    fn brake_tilt_uses_balance_offset_while_regenerating_like_float_out_boy() {
        let mut config = FloatOutBoyConfigImage::defaults();
        assert!(config.editor().set_brake_tilt_strength(PidScale::new(10.0)));
        let mut state = RideModifierState::default();
        let setpoints = state.advance(
            &config,
            RideModifierInput {
                balance_pitch: AngleDegrees::from_degrees(5.0),
                motor_current: MotorCurrent::new(Current::from_amps(-5.0)),
                ..input()
            },
        );

        assert!(setpoints.brake_tilt().angle().is_positive());
    }

    #[test]
    fn wheelslip_winds_down_and_aggregates_the_stronger_matching_torque_like_float_out_boy() {
        let mut state = RideModifierState {
            atr: AtrState {
                angle: SmoothAngle {
                    setpoint: AngleDegrees::from_degrees(2.0),
                    ..SmoothAngle::default()
                },
                ..AtrState::default()
            },
            brake: SmoothAngle {
                setpoint: AngleDegrees::from_degrees(1.0),
                ..SmoothAngle::default()
            },
            torque: SmoothAngle {
                setpoint: AngleDegrees::from_degrees(4.0),
                ..SmoothAngle::default()
            },
            ..RideModifierState::default()
        };
        let config = FloatOutBoyConfigImage::defaults();

        let setpoints = state.advance(
            &config,
            RideModifierInput {
                wheelslip: FloatOutBoyWheelSlipState::Detected,
                ..input()
            },
        );

        assert_eq!(
            setpoints.board().angle(),
            AngleDegrees::from_degrees(4.0 * 0.995)
        );
    }

    #[test]
    fn darkride_keeps_remote_tilt_but_suppresses_ride_modifiers_like_float_out_boy() {
        let mut state = RideModifierState {
            nose: AngleDegrees::from_degrees(1.0),
            turn: TurnTiltState {
                angle: SmoothAngle {
                    setpoint: AngleDegrees::from_degrees(2.0),
                    ..SmoothAngle::default()
                },
                ..TurnTiltState::default()
            },
            torque: SmoothAngle {
                setpoint: AngleDegrees::from_degrees(3.0),
                ..SmoothAngle::default()
            },
            ..RideModifierState::default()
        };
        let config = FloatOutBoyConfigImage::defaults();
        let setpoints = state.advance(
            &config,
            RideModifierInput {
                base_setpoint: AngleDegrees::from_degrees(4.0),
                remote_setpoint: AngleDegrees::from_degrees(-1.0),
                darkride: true,
                ..input()
            },
        );

        assert_eq!(setpoints.board().angle(), AngleDegrees::from_degrees(3.0));
        assert_eq!(setpoints.remote().angle(), AngleDegrees::from_degrees(-1.0));
    }

    #[test]
    fn source_sign_treats_both_zero_encodings_as_nonnegative() {
        let positive = AngleDegrees::from_degrees(3.0);
        let positive_zero = AngleDegrees::from_degrees(0.0);
        let negative_zero = AngleDegrees::from_degrees(-0.0);

        assert!(same_source_sign(positive_zero, positive));
        assert!(same_source_sign(negative_zero, positive));
        assert_eq!(combine_torque_offsets(negative_zero, positive), positive,);
    }

    #[test]
    fn nose_angling_covers_source_thresholds_limits_directions_and_winddown() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_tiltback_constant(AngleDegrees::from_degrees(2.0)));
        assert!(editor.set_tiltback_constant_erpm(ElectricalSpeed::new(
            Rpm::from_revolutions_per_minute(500.0),
        )));
        assert!(editor.set_tiltback_variable(PidScale::new(1.0)));
        assert!(editor.set_tiltback_variable_max(AngleDegrees::from_degrees(3.0)));
        assert!(editor.set_tiltback_variable_erpm(ElectricalSpeed::new(
            Rpm::from_revolutions_per_minute(1_000.0),
        )));
        assert!(editor.set_nose_angling_speed(
            vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),
        ));

        for (erpm, expected) in [
            (500.0, 0.0),
            (1_000.0, 2.0),
            (2_000.0, 3.0),
            (10_000.0, 5.0),
            (-2_000.0, -3.0),
            (-10_000.0, -5.0),
        ] {
            assert_eq!(
                nose_target(&config, Rpm::from_revolutions_per_minute(erpm)),
                AngleDegrees::from_degrees(expected),
            );
        }

        let mut state = RideModifierState::default();
        state.update_nose(
            &config,
            Rpm::from_revolutions_per_minute(10_000.0),
            SampleRate::from_hertz(100.0),
        );
        assert!((state.nose.as_degrees() - 1.0).abs() < 0.000_001);
        state.update_nose(&config, Rpm::ZERO, SampleRate::from_hertz(100.0));
        assert_eq!(state.nose, AngleDegrees::ZERO);
    }

    #[test]
    fn torque_tilt_covers_source_threshold_regen_limit_and_return() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_torque_tilt_start_current(MotorCurrent::new(Current::from_amps(
            10.0,
        ))));
        assert!(editor.set_torque_tilt_strength(PidScale::new(0.1)));
        assert!(editor.set_torque_tilt_regen_strength(PidScale::new(0.2)));
        assert!(editor.set_torque_tilt_angle_limit(AngleDegrees::from_degrees(3.0)));
        assert!(editor.set_torque_tilt_on_speed(
            vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),
        ));
        assert!(editor.set_torque_tilt_off_speed(
            vescpkg_rs::AngularVelocity::from_degrees_per_second(50.0),
        ));
        let balance = config.balance();

        for (current, braking, expected) in [
            (5.0, false, 0.0),
            (20.0, false, 1.0),
            (100.0, false, 3.0),
            (-20.0, true, -2.0),
            (-100.0, true, -3.0),
        ] {
            assert_eq!(
                torque_target(balance, Current::from_amps(current), braking),
                AngleDegrees::from_degrees(expected),
            );
        }

        let mut state = RideModifierState::default();
        state.update_torque(
            balance,
            Current::from_amps(100.0),
            false,
            3_000.0,
            SampleRate::from_hertz(100.0),
        );
        let active = state.torque.setpoint;
        assert!(active.is_positive());
        state.update_torque(
            balance,
            Current::ZERO,
            false,
            3_000.0,
            SampleRate::from_hertz(100.0),
        );
        assert!(state.torque.setpoint < active);
    }

    #[test]
    fn atr_covers_acceleration_speed_boost_braking_limit_and_recovery() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_atr_strength_up(PidScale::new(1.0)));
        assert!(editor.set_atr_strength_down(PidScale::new(1.0)));
        assert!(editor.set_atr_threshold_up(AngleDegrees::ZERO));
        assert!(editor.set_atr_threshold_down(AngleDegrees::ZERO));
        assert!(editor.set_atr_speed_boost(PidScale::new(0.5)));
        assert!(editor.set_atr_angle_limit(AngleDegrees::from_degrees(3.0)));
        assert!(editor.set_atr_on_speed(
            vescpkg_rs::AngularVelocity::from_degrees_per_second(100.0),
        ));
        assert!(editor.set_atr_off_speed(
            vescpkg_rs::AngularVelocity::from_degrees_per_second(50.0),
        ));
        assert!(editor.set_atr_amps_accel_ratio(PidScale::new(1.0)));
        assert!(editor.set_atr_amps_decel_ratio(PidScale::new(1.0)));
        let balance = config.balance();
        let mut state = RideModifierState::default();
        let accelerating = RideModifierInput {
            motor_erpm: Rpm::from_revolutions_per_minute(4_000.0),
            filtered_current: Current::from_amps(30.0),
            motor_current: MotorCurrent::new(Current::from_amps(30.0)),
            ..input()
        };

        for _ in 0..200 {
            state.update_atr(
                balance,
                accelerating,
                false,
                4_000.0,
                1.0,
                SampleRate::from_hertz(100.0),
            );
        }
        let accelerating_setpoint = state.atr.angle.setpoint;
        assert!(accelerating_setpoint.is_positive());
        assert!(accelerating_setpoint <= AngleDegrees::from_degrees(3.0));
        assert!((state.atr.speed_boost - 1.0 / 7.0).abs() < 0.000_001);

        let braking = RideModifierInput {
            filtered_current: Current::from_amps(-30.0),
            motor_current: MotorCurrent::new(Current::from_amps(-30.0)),
            ..accelerating
        };
        for _ in 0..400 {
            state.update_atr(
                balance,
                braking,
                true,
                4_000.0,
                1.0,
                SampleRate::from_hertz(100.0),
            );
        }
        assert!(state.atr.angle.setpoint.is_negative());
        assert!(state.atr.angle.setpoint >= AngleDegrees::from_degrees(-3.0));
        assert_eq!(state.atr.speed_boost, 0.0);

        let before_recovery = state.atr.angle.setpoint;
        for _ in 0..1_000 {
            state.update_atr(
                balance,
                RideModifierInput {
                    filtered_current: Current::from_amps(8.0),
                    motor_current: MotorCurrent::new(Current::from_amps(8.0)),
                    ..accelerating
                },
                false,
                4_000.0,
                1.0,
                SampleRate::from_hertz(100.0),
            );
        }
        assert!(state.atr.angle.setpoint > before_recovery);
    }

    #[test]
    fn brake_and_turn_tilt_cover_source_gates_saturation_direction_and_return() {
        let mut config = FloatOutBoyConfigImage::defaults();
        let mut editor = config.editor();
        assert!(editor.set_brake_tilt_strength(PidScale::new(10.0)));
        assert!(editor.set_brake_tilt_lingering(PidScale::new(2.0)));
        assert!(editor.set_turn_tilt_strength(PidScale::new(100.0)));
        assert!(editor.set_turn_tilt_angle_limit(AngleDegrees::from_degrees(3.0)));
        assert!(editor.set_turn_tilt_start_erpm(ElectricalSpeed::new(
            Rpm::from_revolutions_per_minute(1_000.0),
        )));
        let balance = config.balance();
        let mut state = RideModifierState {
            turn: TurnTiltState {
                yaw_aggregate: AngleDegrees::from_degrees(20.0),
                abs_yaw_change: AngleDegrees::from_degrees(0.1),
                ..TurnTiltState::default()
            },
            ..RideModifierState::default()
        };

        assert_eq!(
            turn_target(
                &state.turn,
                balance,
                Rpm::from_revolutions_per_minute(500.0),
            ),
            AngleDegrees::ZERO,
        );
        assert_eq!(
            turn_target(
                &state.turn,
                balance,
                Rpm::from_revolutions_per_minute(3_000.0),
            ),
            AngleDegrees::from_degrees(3.0),
        );
        assert_eq!(
            turn_target(
                &state.turn,
                balance,
                Rpm::from_revolutions_per_minute(-3_000.0),
            ),
            AngleDegrees::from_degrees(-3.0),
        );

        let braking = RideModifierInput {
            balance_pitch: AngleDegrees::from_degrees(5.0),
            motor_current: MotorCurrent::new(Current::from_amps(-5.0)),
            ..input()
        };
        for _ in 0..100 {
            state.update_brake(balance, braking, true, 3_000.0, 1.0, SampleRate::from_hertz(100.0));
        }
        let sustained = state.brake.setpoint;
        assert!(sustained.is_positive());
        for _ in 0..100 {
            state.update_brake(
                balance,
                RideModifierInput {
                    motor_current: MotorCurrent::new(Current::ZERO),
                    ..braking
                },
                false,
                3_000.0,
                1.0,
                SampleRate::from_hertz(100.0),
            );
        }
        assert!(state.brake.setpoint < sustained);

        state.update_turn(
            balance,
            Rpm::from_revolutions_per_minute(3_000.0),
            SampleRate::from_hertz(100.0),
        );
        let active_turn = state.turn.angle.setpoint;
        assert!(active_turn.is_positive());
        state.turn.yaw_aggregate = AngleDegrees::ZERO;
        state.turn.abs_yaw_change = AngleDegrees::ZERO;
        state.update_turn(
            balance,
            Rpm::from_revolutions_per_minute(3_000.0),
            SampleRate::from_hertz(100.0),
        );
        assert!(state.turn.angle.setpoint < active_turn);
    }
}
