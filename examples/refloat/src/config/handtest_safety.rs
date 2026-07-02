use super::{
    ATR_STRENGTH_DOWN_FIELD, ATR_STRENGTH_UP_FIELD, RefloatBalanceConfig, RefloatConfigEditor,
    RefloatFaultConfig, TILTBACK_CONSTANT_FIELD, TILTBACK_VARIABLE_FIELD,
    TORQUE_TILT_REGEN_STRENGTH_FIELD, TORQUE_TILT_STRENGTH_FIELD, TURN_TILT_STRENGTH_FIELD,
};
use vescpkg_rs::prelude::{AngleDegrees, IntegralCurrentGain, PidScale, VescSeconds};

#[cfg_attr(test, allow(dead_code))]
impl RefloatConfigEditor<'_> {
    pub(crate) fn apply_handtest_safety_overrides(&mut self) -> bool {
        // C map: HANDTEST temporarily clears these tune fields at
        // `third_party/refloat/src/main.c:1431-1444`; the serialized offsets
        // follow `third_party/refloat/src/conf/settings.xml:3943-3981`.
        RefloatBalanceConfig::KI_FIELD
            .write(self, IntegralCurrentGain::new(0.0))
            .is_some()
            && RefloatBalanceConfig::KP_BRAKE_FIELD
                .write(self, PidScale::new(1.0))
                .is_some()
            && RefloatBalanceConfig::KP2_BRAKE_FIELD
                .write(self, PidScale::new(1.0))
                .is_some()
            && RefloatBalanceConfig::BOOSTER_ANGLE_FIELD
                .write(self, AngleDegrees::from_degrees(100.0))
                .is_some()
            && RefloatBalanceConfig::BRAKE_BOOSTER_ANGLE_FIELD
                .write(self, AngleDegrees::from_degrees(100.0))
                .is_some()
            && self.clear_handtest_tune_fields()
            && RefloatFaultConfig::DELAY_PITCH_FIELD
                .write(self, VescSeconds::from_seconds(0.05))
                .is_some()
            && RefloatFaultConfig::DELAY_ROLL_FIELD
                .write(self, VescSeconds::from_seconds(0.05))
                .is_some()
    }

    fn clear_handtest_tune_fields(&mut self) -> bool {
        [
            TORQUE_TILT_STRENGTH_FIELD,
            TORQUE_TILT_REGEN_STRENGTH_FIELD,
            ATR_STRENGTH_UP_FIELD,
            ATR_STRENGTH_DOWN_FIELD,
            TURN_TILT_STRENGTH_FIELD,
            TILTBACK_CONSTANT_FIELD,
            TILTBACK_VARIABLE_FIELD,
        ]
        .into_iter()
        .all(|field| field.clear(self).is_some())
    }
}
