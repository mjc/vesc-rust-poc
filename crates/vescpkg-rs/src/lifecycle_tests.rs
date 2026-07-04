use crate::bindings::CustomConfigBindings;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle_core::{
    AppDataHandlerRegistrationError, LbmApi, LoopbackLifecycle, PackageLifecycle,
};
use crate::test_support::{FakeAppDataBindings, FakeBindings, stubs};
use crate::thread::ThreadApi;
use crate::thread::test_support::FakeThreadBindings;
use crate::types::FirmwareFaultCode;
use crate::{RegisterError, ffi};
use rstest::rstest;
use vescpkg_rs_sys::{ExtensionHandler, LibInfo, NativeImage};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

unsafe extern "C" fn stub_thread_entry(_arg: *mut core::ffi::c_void) {}

unsafe extern "C" fn custom_config_get(_buffer: *mut u8, _is_default: bool) -> core::ffi::c_int {
    0
}

unsafe extern "C" fn custom_config_set(_buffer: *mut u8) -> bool {
    true
}

unsafe extern "C" fn custom_config_xml(_buffer: *mut *mut u8) -> core::ffi::c_int {
    0
}

const EXT_HOST_TEST_PROBE_NAME: crate::ExtensionName = crate::extension_name!("ext-c-probe-v12");

fn handler_at_offset(offset: usize) -> ExtensionHandler {
    unsafe { core::mem::transmute(offset) }
}

#[rstest]
#[case::invalid_name(crate::extension_name!("bad-name"), "invalid", 0_usize)]
#[case::missing_ext_prefix(crate::extension_name!("rust-probe-v5"), "invalid", 0_usize)]
#[case::firmware_reject(crate::extension_name!("ext-rust-reject"), "reject", 1_usize)]
#[case::success(crate::extension_name!("ext-rust-ok"), "accept", 1_usize)]
fn register_extension_reports_outcome(
    #[case] name: crate::ExtensionName,
    #[case] mode: &'static str,
    #[case] expected_add_calls: usize,
) {
    let bindings = if mode == "reject" {
        FakeBindings::rejecting()
    } else {
        FakeBindings::new()
    };
    let lifecycle = PackageLifecycle::new(&bindings);
    let descriptor = ExtensionDescriptor::from_handler(name, stub_handler);

    let result = lifecycle.register_extension(descriptor);

    match mode {
        "invalid" => assert_eq!(result, Err(RegisterError::InvalidExtensionName)),
        "reject" => assert_eq!(result, Err(RegisterError::FirmwareRejected)),
        "accept" => assert_eq!(result, Ok(())),
        other => panic!("unexpected mode: {other}"),
    }
    assert_eq!(bindings.add_calls.get(), expected_add_calls);
}

#[rstest]
#[case::rebase_handler_only(crate::extension_name!("ext-test"), 0x31_usize, 0x2000_u32, "accept", true, 0x2031_usize)]
#[case::firmware_reject(crate::extension_name!("ext-c-probe-v12"), 0_usize, 0x2000_u32, "reject", false, 0_usize)]
fn register_extension_from_image_reports_outcome(
    #[case] name: crate::ExtensionName,
    #[case] handler_offset: usize,
    #[case] base_addr: u32,
    #[case] mode: &'static str,
    #[case] check_registered_pointers: bool,
    #[case] expected_last_handler: usize,
) {
    let bindings = if mode == "reject" {
        FakeBindings::rejecting()
    } else {
        FakeBindings::new()
    };
    let lifecycle = PackageLifecycle::new(&bindings);
    let handler = if handler_offset == 0 {
        stubs::extension_handler
    } else {
        handler_at_offset(handler_offset)
    };
    let descriptor = ExtensionDescriptor::from_handler(name, handler);
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr,
    };

    // SAFETY: test metadata models a loaded image that owns the extension handler.
    let image = NativeImage::from_info(&info);
    let result = unsafe { lifecycle.register_extension_from_image(image, descriptor) };

    match mode {
        "accept" => assert_eq!(result, Ok(())),
        "reject" => assert_eq!(result, Err(RegisterError::FirmwareRejected)),
        other => panic!("unexpected mode: {other}"),
    }
    if check_registered_pointers {
        assert_eq!(bindings.last_name.get(), name.as_cstr().as_ptr() as usize);
        assert_eq!(bindings.last_handler.get(), expected_last_handler);
    }
    if mode == "reject" {
        assert_eq!(bindings.add_calls.get(), 1);
    }
}

#[test]
fn lbm_api_registers_extensions_through_bindings() {
    let api = LbmApi::new(FakeBindings::new());

    assert_eq!(
        api.register_extension(crate::extension_name!("ext-rust-add"), stub_handler),
        Ok(())
    );
}

#[rstest]
#[case::default_ticks(0, 0)]
#[case::configured_ticks(1234, 1234)]
fn loopback_lifecycle_forwards_system_time_ticks(
    #[case] configured_ticks: u32,
    #[case] expected_ticks: u32,
) {
    let bindings = FakeAppDataBindings::with_ticks(configured_ticks);
    let lifecycle = LoopbackLifecycle::new(&bindings);
    assert_eq!(
        lifecycle.system_time_ticks(),
        crate::units::TimestampTicks::from_ticks(expected_ticks)
    );
}

#[test]
fn firmware_fault_code_preserves_raw_values_until_compat_encoding() {
    let valid = FirmwareFaultCode::from_raw_code(5);
    let negative = FirmwareFaultCode::from_raw_code(-1);
    let too_large = FirmwareFaultCode::from_raw_code(256);

    assert_eq!(valid.compat_code(), Some(5));
    assert_eq!(negative.compat_code(), None);
    assert_eq!(too_large.compat_code(), None);
}

#[test]
fn thread_api_clamps_sleep_duration_to_firmware_range() {
    let bindings = FakeThreadBindings::new();
    let api = ThreadApi::new(&bindings);

    api.sleep_for(core::time::Duration::MAX);

    assert_eq!(bindings.sleep_micros.get()[0], u32::MAX);
}

struct PairTestThread;

impl crate::FirmwareThread for PairTestThread {
    type State = u32;

    fn run(_ctx: crate::ThreadContext<'static, Self::State>) {}
}

#[test]
fn thread_api_spawns_and_terminates_typed_thread_pair() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0x20]);
    let api = ThreadApi::new(&bindings);
    let mut state = 42_u32;

    let pair = unsafe {
        api.spawn_thread_pair_with_state(
            crate::ThreadPairSpec::new(
                crate::ThreadSpec::<u32>::new::<PairTestThread>(
                    crate::ThreadStackSize::from_bytes(256),
                    crate::thread_name!("first"),
                ),
                crate::ThreadSpec::<u32>::from_entry(
                    stub_thread_entry,
                    crate::ThreadStackSize::from_bytes(128),
                    crate::thread_name!("second"),
                ),
            ),
            &mut state,
        )
    }
    .expect("thread pair");

    assert_eq!(
        pair.first().map(|thread| thread.as_ptr() as usize),
        Some(0x10)
    );
    assert_eq!(
        pair.second().map(|thread| thread.as_ptr() as usize),
        Some(0x20)
    );
    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.spawn_stacks.get(), [256, 128]);
    let state_arg = core::ptr::from_mut(&mut state).cast::<core::ffi::c_void>() as usize;
    assert_eq!(bindings.spawn_args.get(), [state_arg, state_arg]);

    pair.terminate_reverse(&api);

    assert_eq!(bindings.terminate_calls.get(), 2);
    assert_eq!(bindings.terminated_threads.get(), [0x20, 0x10]);
}

#[test]
fn thread_api_preserves_first_thread_when_second_spawn_fails() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0]);
    let api = ThreadApi::new(&bindings);
    let mut state = 42_u32;

    let pair = unsafe {
        api.spawn_thread_pair_with_state(
            crate::ThreadPairSpec::new(
                crate::ThreadSpec::<u32>::new::<PairTestThread>(
                    crate::ThreadStackSize::from_bytes(256),
                    crate::thread_name!("first"),
                ),
                crate::ThreadSpec::<u32>::from_entry(
                    stub_thread_entry,
                    crate::ThreadStackSize::from_bytes(128),
                    crate::thread_name!("second"),
                ),
            ),
            &mut state,
        )
    };

    let pair = pair.expect("first thread remains live");
    assert_eq!(
        pair.first().map(|thread| thread.as_ptr() as usize),
        Some(0x10)
    );
    assert_eq!(pair.second(), None);
    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.terminate_calls.get(), 0);
}

#[rstest]
#[case::three_byte_payload([1_u8, 2, 3], 3)]
fn loopback_lifecycle_forwards_send_app_data(#[case] payload: [u8; 3], #[case] expected_len: u32) {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(&bindings);
    assert_eq!(lifecycle.send_app_data(&payload), Ok(()));

    assert_eq!(bindings.send_calls.get(), 1);
    assert_eq!(bindings.last_len.get(), expected_len);
    assert_eq!(bindings.last_data.get(), payload.as_ptr() as usize);
}

#[rstest]
#[case::register_stub("register", 1, true)]
#[case::register_custom("register", 1, false)]
#[case::clear("clear", 1, false)]
fn loopback_lifecycle_app_data_handler_forwards_to_bindings(
    #[case] mode: &'static str,
    #[case] expected_handler_calls: usize,
    #[case] use_stub_handler: bool,
) {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(&bindings);

    unsafe extern "C" fn custom_handler(_data: *mut u8, _len: u32) {}

    if mode == "register" {
        let registered = if use_stub_handler {
            stubs::app_data_handler
        } else {
            custom_handler
        };
        assert_eq!(lifecycle.register_app_data_handler(registered), Ok(()));
        assert_eq!(
            bindings.last_handler.get(),
            registered as *const () as usize
        );
    } else {
        assert_eq!(lifecycle.clear_app_data_handler(), Ok(()));
        assert_eq!(bindings.last_handler.get(), 0);
    }

    assert_eq!(bindings.handler_calls.get(), expected_handler_calls);
}

#[test]
fn app_data_handler_registration_reports_firmware_rejection() {
    let bindings = FakeAppDataBindings::with_set_handler_result(false);
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(
        lifecycle.register_app_data_handler(stubs::app_data_handler),
        Err(AppDataHandlerRegistrationError::FirmwareRejected)
    );
    assert_eq!(bindings.handler_calls.get(), 1);
}

#[test]
fn loopback_lifecycle_registers_custom_config_before_app_data() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(
        lifecycle.register_custom_config_then_app_data(
            |bindings| {
                unsafe {
                    bindings.register_custom_config(
                        custom_config_get,
                        custom_config_set,
                        custom_config_xml,
                    )
                }
                .then_some(())
                .ok_or(AppDataHandlerRegistrationError::FirmwareRejected)
            },
            stubs::app_data_handler,
        ),
        Ok(())
    );

    assert_eq!(bindings.custom_config_register_calls.get(), 1);
    assert_eq!(bindings.handler_calls.get(), 1);
    assert_eq!(
        bindings.last_handler.get(),
        stubs::app_data_handler as *const () as usize
    );
}

#[test]
fn loopback_lifecycle_skips_app_data_when_custom_config_registration_fails() {
    let bindings = FakeAppDataBindings::with_register_custom_config_result(false);
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(
        lifecycle.register_custom_config_then_app_data(
            |bindings| {
                unsafe {
                    bindings.register_custom_config(
                        custom_config_get,
                        custom_config_set,
                        custom_config_xml,
                    )
                }
                .then_some(())
                .ok_or(AppDataHandlerRegistrationError::FirmwareRejected)
            },
            stubs::app_data_handler,
        ),
        Err(AppDataHandlerRegistrationError::FirmwareRejected)
    );

    assert_eq!(bindings.custom_config_register_calls.get(), 1);
    assert_eq!(bindings.handler_calls.get(), 0);
}

#[test]
fn app_data_handler_clear_reports_firmware_rejection() {
    let bindings = FakeAppDataBindings::with_clear_handler_result(false);
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(
        lifecycle.clear_app_data_handler(),
        Err(AppDataHandlerRegistrationError::FirmwareRejected)
    );
    assert_eq!(bindings.handler_calls.get(), 1);
    assert_eq!(bindings.last_handler.get(), 0);
}

#[test]
fn loopback_lifecycle_clear_package_callbacks_clears_common_callbacks() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(lifecycle.clear_package_callbacks(), Ok(()));
    assert_eq!(bindings.imu_read_callback_calls.get(), 1);
    assert_eq!(bindings.last_imu_read_callback.get(), 0);
    assert_eq!(bindings.handler_calls.get(), 1);
    assert_eq!(bindings.last_handler.get(), 0);
    assert_eq!(bindings.custom_config_clear_calls.get(), 1);
    // C map: Refloat clears IMU, app-data, and custom-config callbacks at
    // `third_party/refloat/src/main.c:2401-2403`.
}

#[test]
fn loopback_lifecycle_clear_package_callbacks_reports_custom_config_failure() {
    let bindings = FakeAppDataBindings::with_clear_custom_configs_result(false);
    let lifecycle = LoopbackLifecycle::new(&bindings);

    assert_eq!(
        lifecycle.clear_package_callbacks(),
        Err(AppDataHandlerRegistrationError::FirmwareRejected)
    );
    assert_eq!(bindings.imu_read_callback_calls.get(), 1);
    assert_eq!(bindings.handler_calls.get(), 1);
    assert_eq!(bindings.custom_config_clear_calls.get(), 1);
}

#[test]
fn extension_descriptor_validate_accepts_ext_prefix() {
    let descriptor =
        ExtensionDescriptor::from_handler(crate::extension_name!("ext-rust-ok"), stub_handler);

    assert!(descriptor.validate().is_ok());
}

#[test]
fn register_extensions_from_image_registers_each_descriptor() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(&bindings);
    let image = NativeImage::new(0x2000);
    let first =
        ExtensionDescriptor::from_handler(crate::extension_name!("ext-rust-a"), stub_handler);
    let second =
        ExtensionDescriptor::from_handler(crate::extension_name!("ext-rust-b"), stub_handler);

    assert_eq!(
        unsafe { lifecycle.register_extensions_from_image(image, [first, second]) },
        Ok(())
    );
    assert_eq!(bindings.add_calls.get(), 2);
}

#[test]
fn loopback_lifecycle_install_sets_stop_hook() {
    let mut info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };
    let mut start = crate::PackageStart::from_raw(&mut info);

    unsafe extern "C" fn stop(_arg: *mut core::ffi::c_void) {}
    assert_eq!(
        LoopbackLifecycle::<FakeAppDataBindings>::install(&mut start, stop),
        Ok(())
    );
    assert!(info.stop_fun.is_some());
}

#[test]
fn registers_an_extension_through_the_lifecycle_helper() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(&bindings);
    let descriptor =
        ExtensionDescriptor::from_handler(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    let image = NativeImage::from_info(&info);
    assert_eq!(
        unsafe { lifecycle.register_extension_from_image(image, descriptor) },
        Ok(())
    );
    assert_eq!(bindings.add_calls.get(), 1);
    assert_eq!(
        EXT_HOST_TEST_PROBE_NAME.as_cstr().to_bytes_with_nul(),
        b"ext-c-probe-v12\0"
    );
}

#[test]
fn lifecycle_descriptor_installs_the_stop_hook() {
    let bindings = FakeAppDataBindings::new();
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };
    let mut start = crate::PackageStart::from_raw(&mut info);

    assert_eq!(
        LoopbackLifecycle::<FakeAppDataBindings>::install(&mut start, stubs::stop_handler),
        Ok(())
    );

    assert_eq!(
        info.stop_fun.expect("stop hook") as *const () as usize,
        stubs::stop_handler as *const () as usize
    );
    assert_eq!(bindings.handler_calls.get(), 0);
}
