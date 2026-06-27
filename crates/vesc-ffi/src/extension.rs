//! LispBM extension descriptor validation and registration errors.

use core::ffi::CStr;

use crate::loader::ExtensionHandler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionNameError {
    MissingExtPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
    InvalidExtensionName,
    FirmwareRejected,
}

#[derive(Clone, Copy)]
pub struct ExtensionDescriptor {
    name: &'static CStr,
    handler: ExtensionHandler,
}

impl ExtensionDescriptor {
    pub const fn new(name: &'static CStr, handler: ExtensionHandler) -> Self {
        Self { name, handler }
    }

    pub const fn name(self) -> &'static CStr {
        self.name
    }

    pub const fn handler(self) -> ExtensionHandler {
        self.handler
    }

    pub fn validate(self) -> Result<Self, ExtensionNameError> {
        if self.name.to_bytes().starts_with(b"ext-") {
            Ok(self)
        } else {
            Err(ExtensionNameError::MissingExtPrefix)
        }
    }
}
