//! Float Out Boy LCM support types.
//!
//! These types model Float Out Boy's external LCM mode and hardware configuration
//! surface. Raw config field packing stays at package/config boundaries.

pub use vesc_float_out_boy_leds::{
    FloatOutBoyHardwareLedsConfig, FloatOutBoyInternalLedLayout, FloatOutBoyInternalLedLayoutError,
    FloatOutBoyLedMode, FloatOutBoyLedStripRole,
};
