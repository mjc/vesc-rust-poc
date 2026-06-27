use crate::ble_loopback::register_loopback_app_data_handler_with;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle::register_extension_from_image;
use crate::lifecycle_core::{LbmApi, LoopbackLifecycle, PackageLifecycle};
use crate::test_support::{stubs, FakeAppDataBindings, FakeBindings};
use crate::{ffi, RegisterError};
use vesc_ffi::{ExtensionHandler, LbmValue, LibInfo, NativeImage};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

const EXT_HOST_TEST_PROBE_NAME: &core::ffi::CStr = c"ext-c-probe-v12";

#[test]
fn register_extension_from_image_rebases_handler_before_firmware_call() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let handler_offset = 0x31_usize;
    let descriptor = ExtensionDescriptor::new(c"ext-test", unsafe {
        core::mem::transmute::<usize, ExtensionHandler>(handler_offset)
    });
    let image = NativeImage::new(0x2000);

    assert_eq!(
        lifecycle.register_extension_from_image(image, descriptor),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().last_handler.get(), 0x2031);
}

#[test]
fn loopback_lifecycle_forwards_firmware_app_data_calls_through_bindings() {
    let bindings = FakeAppDataBindings::with_ticks(1234);
    let lifecycle = LoopbackLifecycle::new(bindings);
    let payload = [1_u8, 2, 3];

    assert_eq!(lifecycle.system_time_ticks(), 1234);
    unsafe { lifecycle.send_app_data(payload.as_ptr(), 3) };

    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_len.get(), 3);
    assert_eq!(
        lifecycle.bindings().last_data.get(),
        payload.as_ptr() as usize
    );
}

#[test]
fn wrapper_delegates_through_the_binding_trait() {
    let bindings = FakeBindings::new();
    let api = LbmApi::new(bindings);
    let name = c"ext-rust-add";

    assert!(api.register_extension(name, stub_handler));
    assert_eq!(api.decode_i32(LbmValue(3)), 3);
    assert_eq!(api.encode_i32(9), LbmValue(9));
    assert!(api.is_number(LbmValue(9)));
    assert_eq!(api.encode_eval_error(), LbmValue(0xffff_ffff));
}

#[test]
fn package_registration_reports_name_validation_and_firmware_rejection() {
    let bindings = FakeBindings::with_add_results([false, true]);
    let lifecycle = PackageLifecycle::new(bindings);

    let invalid = ExtensionDescriptor::new(c"bad-name", stub_handler);
    assert_eq!(
        lifecycle.register_extension(invalid),
        Err(RegisterError::InvalidExtensionName)
    );

    let rejected = ExtensionDescriptor::new(c"ext-rust-reject", stub_handler);
    assert_eq!(
        lifecycle.register_extension(rejected),
        Err(RegisterError::FirmwareRejected)
    );
}

#[test]
fn repeated_package_registration_reports_each_firmware_result() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);

    assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
}

#[test]
fn extension_descriptor_validate_accepts_ext_prefix() {
    let descriptor = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);

    assert!(descriptor.validate().is_ok());
}

#[test]
fn register_extensions_from_image_registers_each_descriptor() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let image = NativeImage::new(0x2000);
    let first = ExtensionDescriptor::new(c"ext-rust-a", stub_handler);
    let second = ExtensionDescriptor::new(c"ext-rust-b", stub_handler);

    assert_eq!(
        lifecycle.register_extensions_from_image(image, [first, second]),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 2);
}

#[test]
fn register_extension_reports_success_when_firmware_accepts() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);

    assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
}

#[test]
fn loopback_lifecycle_install_sets_stop_hook() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);
    let mut info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    unsafe extern "C" fn stop(_arg: *mut core::ffi::c_void) {}
    unsafe extern "C" fn app_data(_data: *mut u8, _len: u32) {}

    assert!(unsafe { lifecycle.install(&mut info, stop, app_data) });
    assert!(info.stop_fun.is_some());
}

#[test]
fn loopback_lifecycle_registers_and_clears_app_data_handler() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

    assert!(lifecycle.register_app_data_handler(handler));
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(
        lifecycle.bindings().last_handler.get(),
        handler as *const () as usize
    );

    assert!(lifecycle.clear_app_data_handler());
    assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
    assert_eq!(lifecycle.bindings().last_handler.get(), 0);
}

#[test]
fn register_extension_from_image_helper_rebases_handler_before_firmware_call() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let handler_offset = 0x31_usize;
    let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, unsafe {
        core::mem::transmute::<usize, ExtensionHandler>(handler_offset)
    });
    let info = LibInfo {
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
    let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
    let info = LibInfo {
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
        Err(crate::ExtensionNameError::MissingExtPrefix)
    ));
    assert_eq!(
        lifecycle.register_extension(descriptor),
        Err(RegisterError::InvalidExtensionName)
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 0);
}

#[test]
fn rejects_firmware_extension_registration_false() {
    let bindings = FakeBindings::rejecting();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    assert_eq!(
        register_extension_from_image(&info, &lifecycle, descriptor),
        Err(RegisterError::FirmwareRejected)
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
}

#[test]
fn repeated_registration_reports_each_firmware_result() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);

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

#[test]
fn lifecycle_descriptor_installs_the_stop_hook() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    assert!(unsafe { lifecycle.install(&mut info, stubs::stop_handler, stubs::app_data_handler) });

    assert_eq!(
        info.stop_fun.expect("stop hook") as *const () as usize,
        stubs::stop_handler as *const () as usize
    );
    assert_eq!(lifecycle.bindings().handler_calls.get(), 0);
}

#[test]
fn lifecycle_registers_the_app_data_handler_separately() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    assert!(lifecycle.register_app_data_handler(stubs::app_data_handler));

    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(
        lifecycle.bindings().last_handler.get(),
        stubs::app_data_handler as *const () as usize
    );
}

#[test]
fn lifecycle_cleanup_clears_the_package_app_data_handler() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    assert!(lifecycle.clear_app_data_handler());

    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_handler.get(), 0);
}

#[test]
fn register_loopback_app_data_handler_with_forwards_to_bindings() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    assert!(register_loopback_app_data_handler_with(
        &lifecycle,
        stubs::app_data_handler
    ));
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
}
