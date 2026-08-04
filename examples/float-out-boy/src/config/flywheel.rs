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
            && S::set_startup_pitch_tolerance(self, AngleDegrees::from_degrees(0.2))
            && S::set_startup_roll_tolerance(self, AngleDegrees::from_degrees(25.0))
            && self.set(F::PITCH_FIELD, AngleDegrees::from_degrees(6.0))
            && self.set(
                F::ROLL_FIELD,
                AngleDegrees::from_degrees(if config.relaxed_roll { 90.0 } else { 35.0 }),
            )
            && B::set_kp(self, config.kp)
            && B::set_kp2(self, config.kp2)
            && C::set_duty_pushback_angle(self, config.duty_angle)
            && C::set_duty_pushback_threshold(self, config.duty_threshold)
            && C::set_duty_pushback_speed(self, config.duty_speed)
            && C::set_tiltback_return_speed(self, config.duty_speed)
            && M::set_brake_current(self, MotorCurrent::new(Current::ZERO))
            && F::set_darkride_enabled(self, false)
            && F::set_reversestop_enabled(self, false)
            && C::set_tiltback_variable_max(self, AngleDegrees::ZERO)
    }
}
