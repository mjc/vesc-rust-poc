use super::FloatOutBoyConfigEditor;
use super::FloatOutBoyFaultConfig;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, RateCurrentGain, Ratio, WireByte,
};
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
    pub(crate) fn apply_flywheel_overrides(&mut self, config: FloatOutBoyFlywheelConfig) -> bool {
        self.apply_handtest_safety_overrides()
            && self.set_startup_pitch_tolerance(AngleDegrees::from_degrees(0.2))
            && self.set_startup_roll_tolerance(AngleDegrees::from_degrees(25.0))
            && FloatOutBoyFaultConfig::PITCH_FIELD
                .write(self, WireByte::new(6))
                .is_some()
            && FloatOutBoyFaultConfig::ROLL_FIELD
                .write(
                    self,
                    WireByte::new(if config.relaxed_roll { 90 } else { 35 }),
                )
                .is_some()
            && self.set_kp(config.kp)
            && self.set_kp2(config.kp2)
            && self.set_duty_pushback_angle(config.duty_angle)
            && self.set_duty_pushback_threshold(config.duty_threshold)
            && self.set_duty_pushback_speed(config.duty_speed)
            && self.set_tiltback_return_speed(config.duty_speed)
            && self.set_brake_current(MotorCurrent::new(Current::ZERO))
            && self.set_darkride_enabled(false)
            && self.set_reversestop_enabled(false)
            && self.set_tiltback_variable_max(AngleDegrees::ZERO)
    }
}
