//! Refloat LCM support types.
//!
//! These types model Refloat's external LCM mode and hardware configuration
//! surface. Raw config field packing stays at package/config boundaries.

mod hardware;
mod mode;

pub use self::hardware::{RefloatHardwareConfig, RefloatHardwareLedsConfig};
pub use self::mode::RefloatLedMode;
