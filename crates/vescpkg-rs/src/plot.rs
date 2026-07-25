//! Checked firmware plotting helpers.

use core::ffi::CStr;

/// Failure returned by plotting operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlotError {
    /// The requested plotting slot is not available.
    Unavailable,
    /// A graph index or point value is invalid.
    InvalidValue,
}

/// Optional firmware plotting capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct Plot;

impl Plot {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Initialize a named plot and channel using NUL-terminated strings.
    pub fn init(&self, title: &CStr, channel: &CStr) -> Result<(), PlotError> {
        unsafe { crate::ffi::plot_init(title.as_ptr(), channel.as_ptr()) }
            .then_some(())
            .ok_or(PlotError::Unavailable)
    }

    /// Add a named graph using a NUL-terminated string.
    pub fn add_graph(&self, name: &CStr) -> Result<(), PlotError> {
        unsafe { crate::ffi::plot_add_graph(name.as_ptr()) }
            .then_some(())
            .ok_or(PlotError::Unavailable)
    }

    /// Select a non-negative graph index.
    pub fn set_graph(&self, index: i32) -> Result<(), PlotError> {
        if index < 0 {
            return Err(PlotError::InvalidValue);
        }
        unsafe { crate::ffi::plot_set_graph(index) }
            .then_some(())
            .ok_or(PlotError::Unavailable)
    }

    /// Send one finite point to the selected graph.
    pub fn send_points(&self, x: f32, y: f32) -> Result<(), PlotError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(PlotError::InvalidValue);
        }
        unsafe { crate::ffi::plot_send_points(x, y) }
            .then_some(())
            .ok_or(PlotError::Unavailable)
    }
}

impl crate::Firmware {
    /// Return the optional firmware plotting capability.
    pub fn plot(&self) -> Plot {
        Plot::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional firmware plotting capability.
    pub fn plot(&self) -> Plot {
        Plot::new()
    }
}
