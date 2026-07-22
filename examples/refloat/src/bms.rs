//! Refloat BMS support.
//!
//! This module owns Refloat-specific BMS extension behavior. Typed package
//! BMS config/state wiring is tracked separately by VESCR-261.

#[cfg(any(test, target_arch = "arm"))]
use crate::package::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::LispValue;

/// Called from Refloat's Lisp loader and BMS polling loop.
///
/// Returns nil for now, matching a startup config with BMS integration disabled.
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/refloat/src/main.c:2319-2331`; VESCR-261 tracks wiring this callback
/// to typed package state so enabled BMS configs can match upstream too.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct ExtBms;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::StatefulLbmExtension for ExtBms {
    type State = RefloatPackageState;

    fn call(_state: &mut Self::State, _args: vescpkg_rs::LispArgs<'_>) -> LispValue {
        LispValue::nil()
    }
}

#[cfg(test)]
mod tests {
    use super::ExtBms;
    use crate::package::RefloatPackageState;
    use crate::package::test_support::sample_all_data_payloads;
    use vescpkg_rs::{LispArgs, LispValue, StatefulLbmExtension};

    #[test]
    fn ext_bms_returns_nil_while_typed_bms_config_is_unwired() {
        // Refloat returns `d->float_conf.bms.enabled` at
        // `third_party/refloat/src/main.c:2319-2331`; the Rust port currently
        // matches startup defaults where BMS integration is disabled.
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let args = LispArgs::empty();
        let nil = LispValue::nil();
        let value = ExtBms::call(&mut state, args);

        assert!(value == nil);
    }
}
