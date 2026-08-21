use super::*;

impl FloatOutBoyConfigImage {
    pub(crate) fn set_haptic_current_threshold(&mut self, threshold: Ratio) -> bool {
        let mut editor = self.editor();
        FloatOutBoyHapticConfig::CURRENT_THRESHOLD_FIELD
            .write(&mut editor, threshold)
            .is_some()
    }
}

impl FloatOutBoyConfigEditor<'_> {
    pub(crate) fn set_meta_is_default(&mut self, is_default: bool) -> bool {
        self.set_flag(FloatOutBoyMetadataConfig::IS_DEFAULT_FIELD, is_default)
    }

    generated_config_setters! { set_hertz(sample_rate: SampleRate) => FloatOutBoyStartupConfig::HERTZ_FIELD; }

    pub(crate) fn set_moving_faults_disabled(&mut self, disabled: bool) -> bool {
        self.set_flag(
            FloatOutBoyFaultConfig::MOVING_FAULT_DISABLED_FIELD,
            disabled,
        )
    }

    generated_config_setters! {
        set_fault_adc_half_erpm(speed: ElectricalSpeed) => FloatOutBoyFaultConfig::ADC_HALF_ERPM_FIELD;
        set_switch_half_delay(delay: VescSeconds) => FloatOutBoyFaultConfig::DELAY_SWITCH_HALF_FIELD;
        set_switch_full_delay(delay: VescSeconds) => FloatOutBoyFaultConfig::DELAY_SWITCH_FULL_FIELD;
    }

    generated_config_setters! { set_remote_throttle_current_max(current: MotorCurrent) => FloatOutBoyRemoteThrottleConfig::CURRENT_MAX_FIELD; }

    generated_config_setters! { set_remote_throttle_grace_period(duration: VescSeconds) => FloatOutBoyRemoteThrottleConfig::GRACE_PERIOD_FIELD; }

    generated_config_setters! { set_speed_pushback_threshold(speed: vescpkg_rs::WireByte) => FloatOutBoyConfigImage::SPEED_PUSHBACK_THRESHOLD_FIELD; }

    generated_config_setters! { set_input_tilt_inverted(inverted: bool) => FloatOutBoyConfigImage::INPUT_TILT_INVERT_FIELD; }

    generated_config_setters! { set_input_tilt_deadband(deadband: Ratio) => FloatOutBoyConfigImage::INPUT_TILT_DEADBAND_FIELD; }
}

impl FloatOutBoyMetadataConfig<'_> {
    pub(crate) fn is_default(self) -> bool {
        generated_field(Self::IS_DEFAULT_FIELD.read(self.0))
    }
}
