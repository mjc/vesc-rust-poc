//! Float Out Boy LCM support types.
//!
//! These types model Float Out Boy's external LCM mode and hardware configuration
//! surface. Raw config field packing stays at package/config boundaries.

mod hardware;
mod mode;

pub use self::hardware::{
    FloatOutBoyHardwareLedsConfig, FloatOutBoyInternalLedLayout, FloatOutBoyInternalLedLayoutError,
    FloatOutBoyLedStripRole,
};
pub use self::mode::FloatOutBoyLedMode;
