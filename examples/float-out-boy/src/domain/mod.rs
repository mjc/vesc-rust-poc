//! Float Out Boy-specific ride-domain types.
//!
//! These types compose the reusable `vescpkg-rs` package-author units and
//! semantic wrappers into Float Out Boy concepts. Raw firmware/app-data primitives
//! should stay at explicit boundary conversions.
//!
//! Source anchors for the compatibility surface below are Float Out Boy `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/float-out-boy/src/main.c:1313-1399` defines `COMMAND_GET_ALLDATA` response layout.
//! - `third_party/float-out-boy/src/main.c:1876-1901` defines realtime-data ID-list packet layout.
//! - `third_party/float-out-boy/src/main.c:1190-1205` defines startup `Data` initialization order.

pub use vesc_float_out_boy_protocol::*;
