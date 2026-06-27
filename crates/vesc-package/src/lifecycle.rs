use crate::ffi::{self, LbmBindings};

/// Register one extension descriptor against loader metadata.
pub fn register_extension_from_image<B: LbmBindings>(
    info: &ffi::LibInfo,
    lifecycle: &ffi::PackageLifecycle<B>,
    descriptor: ffi::ExtensionDescriptor,
) -> Result<(), ffi::RegisterError> {
    let image = ffi::NativeImage::from_info(info);
    lifecycle.register_extension_from_image(image, descriptor)
}

/// Register one extension through the live firmware binding set.
pub fn register_extension_from_image_real(
    info: &ffi::LibInfo,
    descriptor: ffi::ExtensionDescriptor,
) -> Result<(), ffi::RegisterError> {
    register_extension_from_image(
        info,
        &ffi::PackageLifecycle::new(ffi::RealBindings),
        descriptor,
    )
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::register_extension_from_image;
    use crate::ffi::test_support::stubs;
    use crate::ffi::test_support::FakeBindings;
    use crate::ffi::{self, ExtensionDescriptor, PackageLifecycle};

    const EXT_HOST_TEST_PROBE_NAME: &core::ffi::CStr = c"ext-c-probe-v12";

    #[test]
    fn register_extension_from_image_rebases_handler_before_firmware_call() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let handler_offset = 0x31_usize;
        let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, unsafe {
            core::mem::transmute::<usize, ffi::ExtensionHandler>(handler_offset)
        });
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert_eq!(
            register_extension_from_image(&info, &lifecycle, descriptor),
            Ok(())
        );
        assert_eq!(lifecycle.bindings().last_handler.get(), 0x2031);
    }

    #[test]
    fn registers_an_extension_through_the_lifecycle_helper() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor =
            ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert_eq!(
            register_extension_from_image(&info, &lifecycle, descriptor),
            Ok(())
        );
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
        assert_eq!(
            EXT_HOST_TEST_PROBE_NAME.to_bytes_with_nul(),
            b"ext-c-probe-v12\0"
        );
    }

    #[test]
    fn rejects_non_extension_names_before_calling_firmware() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(c"rust-probe-v5", stubs::extension_handler);

        assert!(matches!(
            descriptor.validate(),
            Err(crate::ffi::ExtensionNameError::MissingExtPrefix)
        ));
        assert_eq!(
            lifecycle.register_extension(descriptor),
            Err(ffi::RegisterError::InvalidExtensionName)
        );
        assert_eq!(lifecycle.bindings().add_calls.get(), 0);
    }

    #[test]
    fn rejects_firmware_extension_registration_false() {
        let bindings = FakeBindings::rejecting();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor =
            ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert_eq!(
            register_extension_from_image(&info, &lifecycle, descriptor),
            Err(ffi::RegisterError::FirmwareRejected)
        );
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    }

    #[test]
    fn repeated_registration_reports_each_firmware_result() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor =
            ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        assert_eq!(
            lifecycle.bindings().last_name.get(),
            EXT_HOST_TEST_PROBE_NAME.as_ptr() as usize
        );
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            stubs::extension_handler as *const () as usize
        );
    }
}
