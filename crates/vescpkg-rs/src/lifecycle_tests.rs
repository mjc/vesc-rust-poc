use crate::RegisterError;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle_core::{LbmApi, PackageLifecycle};
use crate::test_support::{FakeBindings, stubs};
use crate::thread::ThreadApi;
use crate::thread::test_support::FakeThreadBindings;
use crate::types::{FirmwareFaultCode, FirmwareFaultCompatCode};
use rstest::rstest;
use vescpkg_rs_sys::{ExtensionHandler, LibInfo, NativeImage};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

unsafe extern "C" fn stub_thread_entry(_arg: *mut core::ffi::c_void) {}

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
#[case::resolve_handler_offset(crate::extension_name!("ext-test"), 0x31_usize, 0x2000_u32, "accept", true, 0x2031_usize)]
#[case::preserve_loaded_handler(crate::extension_name!("ext-loaded"), 0x2031_usize, 0x2000_u32, "accept", true, 0x2031_usize)]
#[case::firmware_reject(crate::extension_name!("ext-c-probe-v12"), 0_usize, 0x2000_u32, "reject", false, 0_usize)]
fn register_extension_from_image_reports_outcome(
    #[case] name: crate::ExtensionName,
    #[case] handler_address: usize,
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
    let handler = if handler_address == 0 {
        stubs::extension_handler
    } else {
        handler_at_offset(handler_address)
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

#[test]
fn firmware_fault_code_preserves_raw_values_until_compat_encoding() {
    let valid = FirmwareFaultCode::from_raw_code(5);
    let negative = FirmwareFaultCode::from_raw_code(-1);
    let too_large = FirmwareFaultCode::from_raw_code(256);

    assert_eq!(valid.compat_code(), Some(5));
    assert_eq!(negative.compat_code(), None);
    assert_eq!(too_large.compat_code(), None);
    assert_eq!(
        FirmwareFaultCompatCode::try_from(valid)
            .expect("valid firmware fault code")
            .compat_code(),
        5
    );
    assert!(FirmwareFaultCompatCode::try_from(negative).is_err());
    assert!(FirmwareFaultCompatCode::try_from(too_large).is_err());
}

#[test]
fn thread_api_clamps_sleep_duration_to_firmware_range() {
    let bindings = FakeThreadBindings::new();
    let api = ThreadApi::new(&bindings);

    api.sleep_for(core::time::Duration::MAX);

    assert_eq!(bindings.sleep_micros.get()[0], u32::MAX);
}

struct PairTestThread;
struct PairTestState(u32);
static PAIR_TEST_STATE: crate::PackageStateStore<PairTestState> = crate::PackageStateStore::new();

impl crate::PackageRuntimeState for PairTestState {
    fn runtime_store() -> &'static crate::PackageStateStore<Self> {
        &PAIR_TEST_STATE
    }

    fn stop(&mut self) {}
}

impl crate::FirmwareThread for PairTestThread {
    type State = PairTestState;

    fn run(_ctx: crate::ThreadContext<Self::State>) {}
}

#[test]
fn thread_api_spawns_and_terminates_typed_thread_pair() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0x20]);
    let api = ThreadApi::new(&bindings);
    let mut state = PairTestState(42);

    let pair = api
        .spawn_thread_pair(
            crate::ThreadPairSpec::new(
                crate::ThreadSpec::<PairTestState>::new::<PairTestThread>(
                    crate::ThreadStackSize::from_bytes(256),
                    crate::thread_name!("first"),
                ),
                crate::ThreadSpec::<()>::from_entry(
                    stub_thread_entry,
                    crate::ThreadStackSize::from_bytes(128),
                    crate::thread_name!("second"),
                ),
            ),
            core::ptr::NonNull::from(&mut state),
        )
        .expect("thread pair");

    assert_eq!(pair.first().as_ptr() as usize, 0x10);
    assert_eq!(pair.second().as_ptr() as usize, 0x20);
    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.spawn_stacks.get(), [256, 128]);
    let state_arg = core::ptr::from_mut(&mut state).cast::<core::ffi::c_void>() as usize;
    assert_eq!(bindings.spawn_args.get(), [state_arg, 0]);
    assert_eq!(state.0, 42);

    pair.terminate_reverse(&api);

    assert_eq!(bindings.terminate_calls.get(), 2);
    assert_eq!(bindings.terminated_threads.get(), [0x20, 0x10]);
}

#[test]
fn thread_api_preserves_first_thread_when_second_spawn_fails() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0]);
    let api = ThreadApi::new(&bindings);
    let mut state = PairTestState(42);

    let pair = api.spawn_thread_pair(
        crate::ThreadPairSpec::new(
            crate::ThreadSpec::<PairTestState>::new::<PairTestThread>(
                crate::ThreadStackSize::from_bytes(256),
                crate::thread_name!("first"),
            ),
            crate::ThreadSpec::<()>::from_entry(
                stub_thread_entry,
                crate::ThreadStackSize::from_bytes(128),
                crate::thread_name!("second"),
            ),
        ),
        core::ptr::NonNull::from(&mut state),
    );

    assert_eq!(pair, None);
    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.terminate_calls.get(), 1);
    assert_eq!(bindings.terminated_threads.get()[0], 0x10);
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
