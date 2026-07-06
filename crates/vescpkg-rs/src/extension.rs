//! LispBM extension descriptor validation and registration errors.

use core::ffi::CStr;

use vescpkg_rs_sys::{ExtensionHandler, LbmValue};

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

/// Typed LispBM extension callback arguments.
pub struct LbmExtensionArgs<'a> {
    values: &'a [LbmValue],
}

impl<'a> LbmExtensionArgs<'a> {
    fn from_raw(args: *mut u32, arg_count: u32) -> Option<Self> {
        let len = usize::try_from(arg_count).ok()?;
        if len == 0 {
            return Some(Self { values: &[] });
        }
        crate::lbm_args(args, arg_count).map(|values| Self { values })
    }

    /// Return all raw LispBM values.
    pub const fn values(&self) -> &'a [LbmValue] {
        self.values
    }

    /// Decode one LispBM value as an integer.
    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            crate::LbmApi::new(crate::RealBindings).decode_i32(value)
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        {
            value.0 as i32
        }
    }

    /// Return the firmware true value.
    pub fn true_value(&self) -> LbmValue {
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            crate::LbmApi::new(crate::RealBindings).encode_true()
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        {
            LbmValue(1)
        }
    }

    /// Return the firmware nil value.
    pub fn nil_value(&self) -> LbmValue {
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            crate::LbmApi::new(crate::RealBindings).encode_nil()
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        {
            LbmValue(0)
        }
    }
}

/// Rust implementation for a LispBM extension callback.
pub trait LbmExtension {
    /// Handle one extension call.
    fn call(args: LbmExtensionArgs<'_>) -> LbmValue;
}

/// Firmware ABI trampoline for a typed LispBM extension callback.
///
/// # Safety
///
/// `args` must be null with `arg_count == 0` or point to `arg_count` LispBM values that stay valid for
/// this call.
pub unsafe extern "C" fn lbm_extension_handler<T: LbmExtension>(
    args: *mut u32,
    arg_count: u32,
) -> u32 {
    let Some(args) = LbmExtensionArgs::from_raw(args, arg_count) else {
        return 0;
    };
    T::call(args).0
}
