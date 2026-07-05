//! Package lifecycle helpers built on binding traits.

use core::ffi::CStr;

use crate::bindings::{AppDataBindings, LbmBindings};
use crate::extension::{ExtensionDescriptor, RegisterError};
use vescpkg_rs_sys::{AppDataHandler, ExtensionHandler, LbmValue, NativeImage, StopHandler};

/// Thin wrapper around the LispBM binding set used by package code.
pub struct LbmApi<B> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    /// Construct a new LispBM API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the underlying bindings implementation.
    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// Register one LispBM extension name and handler.
    pub fn register_extension(&self, name: &CStr, handler: ExtensionHandler) -> bool {
        unsafe { self.bindings.add_extension(name.as_ptr(), handler) }
    }

    /// Decode a LispBM integer value into an `i32`.
    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    /// Encode an `i32` as a LispBM value.
    pub fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { self.bindings.encode_i32(value) }
    }

    /// Check whether a LispBM value is numeric.
    pub fn is_number(&self, value: LbmValue) -> bool {
        unsafe { self.bindings.is_number(value) }
    }

    /// Return the LispBM eval-error value.
    pub fn encode_eval_error(&self) -> LbmValue {
        unsafe { self.bindings.encode_eval_error() }
    }

    /// Return the LispBM true value.
    pub fn encode_true(&self) -> LbmValue {
        self.bindings.encode_true()
    }

    /// Return the LispBM nil value.
    pub fn encode_nil(&self) -> LbmValue {
        self.bindings.encode_nil()
    }
}

/// Package lifecycle controller that owns the shared LispBM API wrapper.
pub struct PackageLifecycle<B> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    /// Construct a package lifecycle controller.
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    /// Return the wrapped LispBM API helper.
    pub fn bindings(&self) -> &B {
        self.api.bindings()
    }

    /// Register a validated extension descriptor with firmware.
    pub fn register_extension(&self, descriptor: ExtensionDescriptor) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;

        if self
            .api
            .register_extension(descriptor.name(), descriptor.handler())
        {
            Ok(())
        } else {
            Err(RegisterError::FirmwareRejected)
        }
    }

    /// Register an extension whose name and handler addresses are relative to a loaded native image.
    pub fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;
        let name = image.rebase_ptr(descriptor.name().as_ptr());
        let handler_offset = descriptor.handler() as usize;
        let handler = unsafe {
            core::mem::transmute::<usize, ExtensionHandler>(image.rebase_addr(handler_offset))
        };
        if unsafe { self.api.bindings().add_extension(name, handler) } {
            Ok(())
        } else {
            Err(RegisterError::FirmwareRejected)
        }
    }

    /// Register multiple extensions whose handlers are relative to one loaded native image.
    pub fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: impl IntoIterator<Item = ExtensionDescriptor>,
    ) -> Result<(), RegisterError> {
        for descriptor in descriptors {
            self.register_extension_from_image(image, descriptor)?;
        }
        Ok(())
    }
}

/// Package lifecycle controller for loopback and app-data flows.
pub struct LoopbackLifecycle<B> {
    bindings: B,
}

/// Error returned when app-data handler registration fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDataHandlerRegistrationError {
    /// Firmware rejected the handler update.
    FirmwareRejected,
}

impl core::fmt::Display for AppDataHandlerRegistrationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FirmwareRejected => f.write_str("firmware rejected app-data handler update"),
        }
    }
}

impl<B: AppDataBindings> LoopbackLifecycle<B> {
    /// Construct a loopback lifecycle controller.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the wrapped app-data bindings.
    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// Install the package stop hook into loader metadata.
    pub fn install(
        &self,
        start: &mut crate::PackageStart,
        stop_handler: StopHandler,
        _app_data_handler: AppDataHandler,
    ) -> bool {
        if let Some(info) = start.loader_info_mut() {
            info.stop_fun = Some(stop_handler);
        }

        true
    }

    /// Clear the app-data callback through the binding set.
    pub fn clear_app_data_handler(&self) -> Result<(), AppDataHandlerRegistrationError> {
        unsafe { app_data_handler_result(self.bindings.clear_app_data_handler()) }
    }

    /// Register the app-data callback through the binding set.
    pub fn register_app_data_handler(
        &self,
        handler: AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        unsafe { app_data_handler_result(self.bindings.set_app_data_handler(handler)) }
    }

    /// Return the current firmware time tick counter.
    pub fn system_time_ticks(&self) -> u32 {
        self.bindings.system_time_ticks()
    }

    /// Send app-data bytes through the firmware callback.
    pub fn send_app_data(&self, data: &[u8]) -> bool {
        self.bindings.send_app_data_bytes(data)
    }
}

fn app_data_handler_result(accepted: bool) -> Result<(), AppDataHandlerRegistrationError> {
    accepted
        .then_some(())
        .ok_or(AppDataHandlerRegistrationError::FirmwareRejected)
}
