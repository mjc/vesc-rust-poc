use super::FloatOutBoyConfigEditor;
#[cfg(any(test, target_arch = "arm"))]
use super::{
    FloatOutBoyBalanceConfig as B, FloatOutBoyConfigImage as C, FloatOutBoyFaultConfig as F,
    FloatOutBoyMotorControlConfig as M, FloatOutBoyStartupConfig as S,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, RateCurrentGain, Ratio,
};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{Current, MotorCurrent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyFlywheelConfig {
    pub(crate) kp: AngleCurrentGain,
    pub(crate) kp2: RateCurrentGain,
    pub(crate) duty_angle: AngleDegrees,
    pub(crate) duty_threshold: Ratio,
    pub(crate) duty_speed: AngularVelocity,
    pub(crate) relaxed_roll: bool,
}

impl FloatOutBoyConfigEditor<'_> {
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn apply_flywheel_overrides(&mut self, config: FloatOutBoyFlywheelConfig) -> bool {
        self.apply_handtest_safety_overrides()
            && self.set(S::PITCH_TOLERANCE_FIELD, AngleDegrees::from_degrees(0.2))
            && self.set(S::ROLL_TOLERANCE_FIELD, AngleDegrees::from_degrees(25.0))
            && self.set(F::PITCH_FIELD, AngleDegrees::from_degrees(6.0))
            && self.set(
                F::ROLL_FIELD,
                AngleDegrees::from_degrees(if config.relaxed_roll { 90.0 } else { 35.0 }),
            )
            && self.set(B::KP_FIELD, config.kp)
            && self.set(B::KP2_FIELD, config.kp2)
            && self.set(C::DUTY_PUSHBACK_ANGLE_FIELD, config.duty_angle)
            && self.set(C::DUTY_PUSHBACK_THRESHOLD_FIELD, config.duty_threshold)
            && self.set(C::DUTY_PUSHBACK_SPEED_FIELD, config.duty_speed)
            && self.set(C::TILTBACK_RETURN_SPEED_FIELD, config.duty_speed)
            && self.set(M::BRAKE_CURRENT_FIELD, MotorCurrent::new(Current::ZERO))
            && self.set(F::DARKRIDE_FIELD, false)
            && self.set(F::REVERSESTOP_FIELD, false)
            && self.set(C::TILTBACK_VARIABLE_MAX_FIELD, AngleDegrees::ZERO)
    }
}
