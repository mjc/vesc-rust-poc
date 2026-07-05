use crate::bindings::LbmBindings;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle_core::PackageLifecycle;
use vescpkg_rs_sys::LibInfo;

/// Register one extension descriptor against loader metadata.
pub fn register_extension_from_image<B: LbmBindings>(
    info: &LibInfo,
    lifecycle: &PackageLifecycle<B>,
    descriptor: ExtensionDescriptor,
) -> Result<(), crate::RegisterError> {
    let image = vescpkg_rs_sys::NativeImage::from_info(info);
    lifecycle.register_extension_from_image(image, descriptor)
}

/// Register one extension through the live firmware binding set.
#[cfg(not(test))]
pub fn register_extension_from_image_real(
    info: &LibInfo,
    descriptor: ExtensionDescriptor,
) -> Result<(), crate::RegisterError> {
    register_extension_from_image(
        info,
        &PackageLifecycle::new(crate::RealBindings),
        descriptor,
    )
}
