//! Float Out Boy app-data protocol wire helpers.
//!
//! C map: app-data packet encoders forward through Float Out Boy buffer helpers in
//! `third_party/float-out-boy/src/conf/buffer.c:33-145`.

use vescpkg_rs::prelude::AngleRadians;

pub(super) fn float_out_boy_degrees(angle: AngleRadians) -> f32 {
    // C map: this converts firmware `radians` telemetry into degrees before encoding
    // payload fields in `third_party/float-out-boy/src/main.c:1267-1310`.
    crate::wire::degrees(angle)
}
