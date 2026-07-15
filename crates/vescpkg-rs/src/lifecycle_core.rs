//! Package lifecycle helpers built on binding traits.

use crate::bindings::LbmBindings;
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use crate::extension::{ExtensionDescriptor, RegisterError};
#[cfg(not(test))]
use vescpkg_rs_sys::LbmValue;
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use vescpkg_rs_sys::{ExtensionHandler, NativeImage};

/// Thin wrapper around the LispBM binding set used by package code.
pub(crate) struct LbmApi<B> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    /// Construct a new LispBM API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Register one LispBM extension name and handler.
    #[cfg(test)]
    pub fn register_extension(
        &self,
        name: crate::ExtensionName,
        handler: ExtensionHandler,
    ) -> Result<(), RegisterError> {
        unsafe {
            self.bindings
                .add_extension(name.as_cstr().as_ptr(), handler)
        }
        .then_some(())
        .ok_or(RegisterError::FirmwareRejected)
    }

    /// Decode a LispBM integer value into an `i32`.
    #[cfg(not(test))]
    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    /// Return the LispBM true value.
    #[cfg(not(test))]
    pub fn encode_true(&self) -> LbmValue {
        self.bindings.encode_true()
    }

    /// Return the LispBM nil value.
    #[cfg(not(test))]
    pub fn encode_nil(&self) -> LbmValue {
        self.bindings.encode_nil()
    }
}

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
/// Package lifecycle controller that owns the shared LispBM API wrapper.
pub(crate) struct PackageLifecycle<B> {
    api: LbmApi<B>,
}

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
impl<B: LbmBindings> PackageLifecycle<B> {
    /// Construct a package lifecycle controller.
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    /// Register a validated extension descriptor with firmware.
    #[cfg(test)]
    pub fn register_extension(&self, descriptor: ExtensionDescriptor) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;

        self.api
            .register_extension(descriptor.name(), descriptor.handler())
    }

    /// Register an extension whose handler address is relative to a loaded native image.
    ///
    /// The descriptor name is a Rust `CStr` reference produced by package code, so on target it is
    /// already a runtime PC-relative pointer. PIC may materialize the handler as either an image
    /// offset or an already loaded address, so loader metadata resolves both forms.
    ///
    /// # Safety
    ///
    /// `image` must describe the loaded native package image that owns
    /// `descriptor`. The resolved handler address must be a valid firmware
    /// LispBM extension function and remain valid for as long as firmware may
    /// call the registered extension.
    pub(crate) unsafe fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;
        let name = descriptor.name().as_cstr().as_ptr();
        let handler_address = descriptor.handler() as usize;
        let handler = unsafe {
            core::mem::transmute::<usize, ExtensionHandler>(image.resolve_addr(handler_address))
        };
        if unsafe { self.api.bindings.add_extension(name, handler) } {
            Ok(())
        } else {
            Err(RegisterError::FirmwareRejected)
        }
    }

    /// Register multiple extensions whose handlers are relative to one loaded native image.
    ///
    /// # Safety
    ///
    /// `image` must describe the loaded native package image that owns every
    /// descriptor. Each resolved handler address must be a valid firmware LispBM
    /// extension function and remain callable by firmware after registration.
    pub(crate) unsafe fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: impl IntoIterator<Item = ExtensionDescriptor>,
    ) -> Result<(), RegisterError> {
        for descriptor in descriptors {
            unsafe { self.register_extension_from_image(image, descriptor)? };
        }
        Ok(())
    }
}

/// Error returned when app-data handler registration fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDataHandlerRegistrationError {
    /// Firmware rejected the handler update.
    FirmwareRejected,
}

/// Failure returned when an app-data payload cannot cross the firmware ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDataSendError {
    /// The payload does not fit the firmware's 512-byte command buffer.
    PayloadTooLarge,
}

impl core::fmt::Display for AppDataHandlerRegistrationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FirmwareRejected => f.write_str("firmware rejected app-data handler update"),
        }
    }
}
