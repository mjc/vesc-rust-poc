//! Float Out Boy BMS support.
//!
//! This module owns Float Out Boy-specific BMS extension behavior.

use crate::package::FloatOutBoyPackageState;
#[cfg(test)]
pub(crate) use vescpkg_rs::BmsStartupGrace as FloatOutBoyBmsStartupGrace;
use vescpkg_rs::LispArgs;
use vescpkg_rs::LispValue;
pub(crate) use vescpkg_rs::{
    BmsFaults as FloatOutBoyBmsFaults, BmsSample as FloatOutBoyBmsSample,
    BmsTemperature as FloatOutBoyBmsTemperature, BmsThresholds as FloatOutBoyBmsThresholds,
};

fn float_out_boy_bms_sample_from_lisp_args(args: &LispArgs<'_>) -> Option<FloatOutBoyBmsSample> {
    (args.len() > 5).then_some(())?;
    FloatOutBoyBmsSample::try_from_telemetry(
        args.get(0)?.decode_number_as_f32()?,
        args.get(1)?.decode_number_as_f32()?,
        args.get(2)?.decode_number_as_i32()?,
        args.get(3)?.decode_number_as_i32()?,
        args.get(4)?.decode_number_as_i32()?,
        args.get(5)?.decode_number_as_f32()?,
    )
}

/// Called from Float Out Boy's Lisp loader and BMS polling loop.
///
/// Upstream returns `d->float_conf.bms.enabled` at
/// `third_party/float-out-boy/src/main.c:2319-2331`.
pub(crate) struct ExtBms;

impl vescpkg_rs::StatefulLbmExtension for ExtBms {
    type State = FloatOutBoyPackageState;

    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
        let enabled = state.bms_enabled();
        if enabled && let Some(sample) = float_out_boy_bms_sample_from_lisp_args(&args) {
            state.record_bms_sample(sample);
        }
        LispValue::boolean(enabled)
    }
}

#[cfg(test)]
mod tests;
