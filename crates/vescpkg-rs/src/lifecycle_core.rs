//! Package lifecycle helpers built on binding traits.

use crate::bindings::LbmBindings;
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use crate::extension::{ExtensionDescriptor, ExtensionRegistration, ExtensionRegistrationError};
use vescpkg_rs_sys::LbmValue;
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use vescpkg_rs_sys::{ExtensionHandler, NativeImage};

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
#[expect(
    clippy::as_conversions,
    reason = "Rust provides no non-cast conversion from a function pointer to its code address"
)]
pub(crate) fn extension_handler_address(handler: ExtensionHandler) -> usize {
    handler as usize
}

/// Thin wrapper around the `LispBM` binding set used by package code.
pub(crate) struct LbmApi<B> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    /// Construct a new `LispBM` API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Register one `LispBM` extension name and handler.
    #[cfg(test)]
    pub fn register_extension(
        &self,
        name: crate::ExtensionName,
        handler: ExtensionHandler,
    ) -> Result<(), ExtensionRegistrationError> {
        // SAFETY: the validated name is static and NUL-terminated, and the
        // handler is a static function with the firmware callback ABI.
        unsafe {
            self.bindings
                .add_extension(name.as_cstr().as_ptr(), handler)
        }
        .then_some(())
        .ok_or(ExtensionRegistrationError::FirmwareRejected)
    }

    /// Convert any `LispBM` numeric value to an `i32`.
    pub fn decode_number_as_i32(&self, value: LbmValue) -> Option<i32> {
        // SAFETY: the caller supplied a firmware `LispBM` value.
        if unsafe { self.bindings.is_number(value) } {
            // SAFETY: firmware just confirmed that the value is numeric.
            Some(unsafe { self.bindings.decode_i32(value) })
        } else {
            None
        }
    }

    /// Convert any `LispBM` numeric value to an `f32`.
    pub fn decode_number_as_f32(&self, value: LbmValue) -> Option<f32> {
        // SAFETY: the caller supplied a firmware `LispBM` value.
        if unsafe { self.bindings.is_number(value) } {
            // SAFETY: firmware just confirmed that the value is numeric.
            Some(unsafe { self.bindings.decode_f32(value) })
        } else {
            None
        }
    }

    /// Return the `LispBM` true value.
    #[cfg(all(not(test), target_arch = "arm"))]
    pub fn encode_true(&self) -> LbmValue {
        self.bindings.encode_true()
    }

    /// Return the `LispBM` nil value.
    #[cfg(all(not(test), target_arch = "arm"))]
    pub fn encode_nil(&self) -> LbmValue {
        self.bindings.encode_nil()
    }
}

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
/// Package lifecycle controller that owns the shared `LispBM` API wrapper.
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
    pub fn register_extension(
        &self,
        descriptor: ExtensionDescriptor,
    ) -> Result<(), ExtensionRegistrationError> {
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
    /// `LispBM` extension function and remain valid for as long as firmware may
    /// call the registered extension.
    pub(crate) unsafe fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<(), ExtensionRegistrationError> {
        let name = descriptor.name().as_cstr().as_ptr();
        let handler_address = extension_handler_address(descriptor.handler());
        // SAFETY: the caller guarantees that resolving this package-local
        // address produces a function with the `ExtensionHandler` C ABI.
        let handler = unsafe {
            core::mem::transmute::<usize, ExtensionHandler>(image.resolve_addr(handler_address))
        };
        // SAFETY: the name and resolved handler satisfy the registration contract.
        if unsafe { self.api.bindings.add_extension(name, handler) } {
            Ok(())
        } else {
            Err(ExtensionRegistrationError::FirmwareRejected)
        }
    }

    /// Register multiple extensions whose handlers are relative to one loaded native image.
    ///
    /// # Safety
    ///
    /// `image` must describe the loaded native package image that owns every
    /// descriptor. Each resolved handler address must be a valid firmware `LispBM`
    /// extension function and remain callable by firmware after registration.
    pub(crate) unsafe fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: impl IntoIterator<Item = ExtensionDescriptor>,
    ) -> ExtensionRegistration {
        let mut requested = 0_usize;
        let mut registered = 0_usize;
        for descriptor in descriptors {
            requested = requested.saturating_add(1);
            // SAFETY: the caller's image and lifetime guarantees apply to every descriptor.
            registered = registered.saturating_add(usize::from(
                unsafe { self.register_extension_from_image(image, descriptor) }.is_ok(),
            ));
        }
        ExtensionRegistration::new(requested, registered)
    }
}
