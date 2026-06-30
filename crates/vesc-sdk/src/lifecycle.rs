use crate::bindings::LbmBindings;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle_core::PackageLifecycle;
use vesc_ffi::LibInfo;

/// Register one extension descriptor against loader metadata.
///
/// # Safety
///
/// `info` must describe the loaded native image that owns `descriptor.handler()`.
/// The rebased handler address must use the firmware LispBM extension ABI and remain
/// valid for as long as firmware may call the registered extension.
pub unsafe fn register_extension_from_image<B: LbmBindings>(
    info: &LibInfo,
    lifecycle: &PackageLifecycle<B>,
    descriptor: ExtensionDescriptor,
) -> Result<(), crate::RegisterError> {
    let image = vesc_ffi::NativeImage::from_info(info);
    unsafe { lifecycle.register_extension_from_image(image, descriptor) }
}

/// Register one extension through the live firmware binding set.
///
/// # Safety
///
/// `info` must describe the loaded native image that owns `descriptor.handler()`.
/// The rebased handler address must use the firmware LispBM extension ABI and remain
/// valid for as long as firmware may call the registered extension.
#[cfg(not(test))]
pub unsafe fn register_extension_from_image_real(
    info: &LibInfo,
    descriptor: ExtensionDescriptor,
) -> Result<(), crate::RegisterError> {
    unsafe {
        register_extension_from_image(
            info,
            &PackageLifecycle::new(crate::RealBindings),
            descriptor,
        )
    }
}
