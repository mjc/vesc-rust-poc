//! Package lifecycle helpers built on binding traits.

use core::ffi::CStr;

use crate::bindings::{AppDataBindings, LbmBindings};
use crate::extension::{ExtensionDescriptor, RegisterError};
use vesc_ffi::{AppDataHandler, ExtensionHandler, LbmValue, LibInfo, NativeImage, StopHandler};

pub struct LbmApi<B> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    pub fn register_extension(&self, name: &CStr, handler: ExtensionHandler) -> bool {
        unsafe { self.bindings.add_extension(name.as_ptr(), handler) }
    }

    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    pub fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { self.bindings.encode_i32(value) }
    }

    pub fn is_number(&self, value: LbmValue) -> bool {
        unsafe { self.bindings.is_number(value) }
    }

    pub fn encode_eval_error(&self) -> LbmValue {
        unsafe { self.bindings.encode_eval_error() }
    }
}

pub struct PackageLifecycle<B> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn bindings(&self) -> &B {
        self.api.bindings()
    }

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

    pub fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;
        let handler_offset = descriptor.handler() as usize;
        let handler = unsafe {
            core::mem::transmute::<usize, ExtensionHandler>(image.rebase_addr(handler_offset))
        };
        if self.api.register_extension(descriptor.name(), handler) {
            Ok(())
        } else {
            Err(RegisterError::FirmwareRejected)
        }
    }

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

pub struct LoopbackLifecycle<B> {
    bindings: B,
}

impl<B: AppDataBindings> LoopbackLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// # Safety
    ///
    /// `info` must either be null or point to live loader metadata.
    /// `stop_handler` must remain valid for as long as the firmware may call it.
    /// The native image is built as PIC, matching refloat's VESC package model,
    /// so this callback pointer is already a runtime address when this code executes.
    pub unsafe fn install(
        &self,
        info: *mut LibInfo,
        stop_handler: StopHandler,
        _app_data_handler: AppDataHandler,
    ) -> bool {
        if let Some(info) = unsafe { info.as_mut() } {
            info.stop_fun = Some(stop_handler);
        }

        true
    }

    pub fn clear_app_data_handler(&self) -> bool {
        unsafe { self.bindings.set_app_data_handler(None) }
    }

    pub fn register_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { self.bindings.set_app_data_handler(Some(handler)) }
    }

    pub fn system_time_ticks(&self) -> u32 {
        self.bindings.system_time_ticks()
    }

    /// # Safety
    ///
    /// `data` must point to at least `len` bytes that remain valid for the duration
    /// of the firmware call.
    pub unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        unsafe { self.bindings.send_app_data(data, len) }
    }
}
