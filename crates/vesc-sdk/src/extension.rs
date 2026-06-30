//! LispBM extension descriptor validation and registration errors.

use core::ffi::CStr;

use vesc_ffi::ExtensionHandler;

/// Extension-name validation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionNameError {
    /// The name did not start with the required `ext-` prefix.
    MissingExtPrefix,
}

/// Errors returned when extension registration fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
    /// The extension name failed validation.
    InvalidExtensionName,
    /// Firmware rejected the registration request.
    FirmwareRejected,
}

/// A validated extension registration request.
#[derive(Clone, Copy)]
pub struct ExtensionDescriptor {
    name: &'static CStr,
    handler: ExtensionHandler,
}

impl ExtensionDescriptor {
    /// Build a descriptor from its name and handler.
    pub const fn new(name: &'static CStr, handler: ExtensionHandler) -> Self {
        Self { name, handler }
    }

    /// Return the descriptor name.
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    /// Return the descriptor handler.
    pub const fn handler(self) -> ExtensionHandler {
        self.handler
    }

    /// Validate the descriptor name prefix expected by the firmware.
    pub fn validate(self) -> Result<Self, ExtensionNameError> {
        if self.name.to_bytes().starts_with(b"ext-") {
            Ok(self)
        } else {
            Err(ExtensionNameError::MissingExtPrefix)
        }
    }
}
