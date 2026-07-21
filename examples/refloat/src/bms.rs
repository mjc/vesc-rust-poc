//! Refloat BMS support.
//!
//! This module owns Refloat-specific BMS extension behavior.

#[cfg(any(test, target_arch = "arm"))]
use crate::package::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispValue;

/// Called from Refloat's Lisp loader and BMS polling loop.
///
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/refloat/src/main.c:2319-2331`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct ExtBms;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::StatefulLbmExtension for ExtBms {
    type State = RefloatPackageState;

    fn call(state: &mut Self::State, _args: vescpkg_rs::LispArgs<'_>) -> LispValue {
        LispValue::boolean(state.bms_enabled())
    }
}

#[cfg(test)]
mod tests {
    use super::ExtBms;
    use crate::config::REFLOAT_DEFAULT_CONFIG;
    use crate::package::test_support::sample_all_data_payloads;
    use crate::package::{RefloatCustomConfig, RefloatPackageState};
    use vescpkg_rs::{
        ConfigBytes, LispArgs, LispValue, StatefulCustomConfigCallback, StatefulLbmExtension,
    };

    #[test]
    fn ext_bms_returns_nil_when_bms_integration_is_disabled() {
        // Refloat returns `d->float_conf.bms.enabled` at
        // `third_party/refloat/src/main.c:2319-2331`.
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let args = LispArgs::empty();
        let nil = LispValue::nil();
        let value = ExtBms::call(&mut state, args);

        assert!(value == nil);
    }

    #[test]
    fn ext_bms_returns_true_when_bms_integration_is_enabled() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut config = REFLOAT_DEFAULT_CONFIG;
        // Generated Refloat v1.2.1 order places `bms.enabled` after the final
        // haptic field and before the BMS thresholds at settings.xml:4076-4082.
        config[265] = 1;
        assert!(RefloatCustomConfig::set_config(&mut state, ConfigBytes::new(&config)).is_ok());

        let value = ExtBms::call(&mut state, LispArgs::empty());

        assert!(value == LispValue::true_value());
    }
}
