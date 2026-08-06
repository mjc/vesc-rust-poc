//! Float Out Boy hardware LED configuration.
//!
//! C map: defaults mirror the hardware LED settings in
//! `third_party/float-out-boy/src/conf/settings.xml:3560-3863`. Float Out Boy treats the
//! mode as flags when enabling internal LEDs and the external LCM at
//! `third_party/float-out-boy/src/leds.c:795-830` and
//! `third_party/float-out-boy/src/lcm.c:27-28`.

use crate::leds::{
    FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
};

use super::mode::FloatOutBoyLedMode;

/// Logical role of one configured internal LED strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyLedStripRole {
    /// Status/sensor strip.
    Status,
    /// Front strip.
    Front,
    /// Rear strip.
    Rear,
}

impl FloatOutBoyLedStripRole {
    const fn index(self) -> usize {
        match self {
            Self::Status => 0,
            Self::Front => 1,
            Self::Rear => 2,
        }
    }
}

/// A validated source-compatible ordering of the internal strips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyInternalLedLayout {
    roles: [FloatOutBoyLedStripRole; 3],
    role_count: usize,
    offsets: [Option<usize>; 3],
    pixel_count: usize,
}

impl FloatOutBoyInternalLedLayout {
    /// Return configured nonempty roles in physical order.
    #[must_use]
    pub fn roles(&self) -> &[FloatOutBoyLedStripRole] {
        self.roles.get(..self.role_count).unwrap_or_default()
    }

    /// Return one role's offset in the shared physical pixel sequence.
    #[must_use]
    pub fn offset(self, role: FloatOutBoyLedStripRole) -> Option<usize> {
        self.offsets.get(role.index()).copied().flatten()
    }

    const_field_getters! {
        /// Return the complete physical pixel count.
        pub fn pixel_count -> usize = pixel_count;
    }
}

/// Internal LED layout validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatOutBoyInternalLedLayoutError {
    /// Selected front and rear strips exceed Refloat's 60-pixel renderer map.
    FrontAndRearCountExceedsMaximum,
}

impl core::fmt::Display for FloatOutBoyInternalLedLayoutError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("front and rear LED count exceeds 60")
    }
}

impl core::error::Error for FloatOutBoyInternalLedLayoutError {}

/// Float Out Boy hardware LED configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOutBoyHardwareLedsConfig {
    pub(crate) mode: FloatOutBoyLedMode,
    pub(crate) pin: FloatOutBoyLedPin,
    pub(crate) pin_config: FloatOutBoyLedPinConfig,
    pub(crate) status: FloatOutBoyLedStripConfig,
    pub(crate) front: FloatOutBoyLedStripConfig,
    pub(crate) rear: FloatOutBoyLedStripConfig,
}

impl FloatOutBoyHardwareLedsConfig {
    /// Build Refloat's nonempty status/front/rear physical strip ordering.
    ///
    /// # Errors
    ///
    /// Returns an error when selected front and rear strips exceed Refloat's
    /// 60-pixel animation-map limit.
    pub fn internal_layout(
        self,
    ) -> Result<FloatOutBoyInternalLedLayout, FloatOutBoyInternalLedLayoutError> {
        let mut layout = FloatOutBoyInternalLedLayout {
            roles: [FloatOutBoyLedStripRole::Status; 3],
            role_count: 0,
            offsets: [None; 3],
            pixel_count: 0,
        };
        let strips = [
            (FloatOutBoyLedStripRole::Status, self.status),
            (FloatOutBoyLedStripRole::Front, self.front),
            (FloatOutBoyLedStripRole::Rear, self.rear),
        ];
        let mut front_rear_count = 0_usize;

        for order in [
            FloatOutBoyLedStripOrder::First,
            FloatOutBoyLedStripOrder::Second,
            FloatOutBoyLedStripOrder::Third,
        ] {
            let Some((role, strip)) = strips
                .into_iter()
                .find(|(_, strip)| strip.order == order && strip.count > 0)
            else {
                continue;
            };
            let Some(slot) = layout.roles.get_mut(layout.role_count) else {
                continue;
            };
            *slot = role;
            let Some(offset) = layout.offsets.get_mut(role.index()) else {
                continue;
            };
            *offset = Some(layout.pixel_count);
            let count = usize::from(strip.count);
            if !matches!(role, FloatOutBoyLedStripRole::Status) {
                front_rear_count = front_rear_count.saturating_add(count);
            }
            layout.role_count = layout.role_count.saturating_add(1);
            layout.pixel_count = layout.pixel_count.saturating_add(count);
        }

        if front_rear_count > 60 {
            return Err(FloatOutBoyInternalLedLayoutError::FrontAndRearCountExceedsMaximum);
        }

        Ok(layout)
    }
}

#[cfg(test)]
#[path = "hardware/tests/api.rs"]
mod test_api;

#[cfg(test)]
mod tests;
