use crate::bindings::LbmBindings;
use crate::extension::{ExtensionDescriptor, ExtensionRegistrationError};
use crate::lifecycle_core::{LbmApi, PackageLifecycle};
use crate::test_support::{FakeBindings, stubs};
use crate::thread::ThreadApi;
use crate::thread::test_support::FakeThreadBindings;
use crate::types::{FirmwareFaultCode, FirmwareFaultWireCode};
use rstest::rstest;
use vescpkg_rs_sys::{ExtensionHandler, LbmValue, LibInfo, NativeImage};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

struct NumericBindings {
    encoded: LbmValue,
    decoded: i32,
}

impl LbmBindings for NumericBindings {
    unsafe fn add_extension(
        &self,
        _name: *const core::ffi::c_char,
        _handler: ExtensionHandler,
    ) -> bool {
        unreachable!("numeric conversion does not register extensions")
    }

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        value == self.encoded
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        assert_eq!(value, self.encoded);
        self.decoded
    }
}

const EXT_HOST_TEST_PROBE_NAME: crate::ExtensionName = crate::extension_name!("ext-c-probe-v12");

fn handler_at_offset(offset: usize) -> ExtensionHandler {
    unsafe { core::mem::transmute(offset) }
}

#[rstest]
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
        "reject" => assert_eq!(result, Err(ExtensionRegistrationError::FirmwareRejected)),
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
        "reject" => assert_eq!(result, Err(ExtensionRegistrationError::FirmwareRejected)),
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
fn lbm_api_decodes_firmware_i32_values() {
    let encoded = LbmValue(0x2800_0001);
    let api = LbmApi::new(NumericBindings {
        encoded,
        decoded: i32::MAX,
    });

    assert_eq!(api.decode_i32(encoded), Some(i32::MAX));
    assert_eq!(api.decode_i32(LbmValue(0)), None);
}

#[test]
fn firmware_fault_code_preserves_raw_values_until_wire_encoding() {
    let valid = FirmwareFaultCode::from_raw_code(5);
    let negative = FirmwareFaultCode::from_raw_code(-1);
    let too_large = FirmwareFaultCode::from_raw_code(256);

    assert_eq!(
        FirmwareFaultWireCode::try_from(valid)
            .expect("valid firmware fault code")
            .wire_code(),
        5
    );
    assert!(FirmwareFaultWireCode::try_from(negative).is_err());
    assert!(FirmwareFaultWireCode::try_from(too_large).is_err());
}

struct GroupTestThread;
struct GroupTestStatelessThread;
struct GroupTestState(u32);
static GROUP_TEST_STATE: crate::PackageStateStore<GroupTestState> = crate::PackageStateStore::new();

impl crate::PackageRuntimeState for GroupTestState {
    fn runtime_store() -> &'static crate::PackageStateStore<Self> {
        &GROUP_TEST_STATE
    }
}

impl crate::FirmwareThread for GroupTestThread {
    type State = GroupTestState;

    fn run(_ctx: crate::ThreadContext<Self::State>) {}
}

impl crate::StatelessFirmwareThread for GroupTestStatelessThread {
    fn run(_ctx: crate::StatelessThreadContext) {}
}

#[test]
fn thread_api_spawns_and_terminates_typed_thread_group() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0x20, 0]);
    let api = ThreadApi::new(&bindings);
    let mut state = GroupTestState(42);

    let threads = api
        .spawn_threads(
            [
                crate::ThreadSpec::<GroupTestState>::new::<GroupTestThread>(
                    crate::ThreadStackSize::from_bytes(1_536),
                    crate::thread_name!("first"),
                ),
                crate::ThreadSpec::<GroupTestState>::stateless::<GroupTestStatelessThread>(
                    crate::ThreadStackSize::from_bytes(1_024),
                    crate::thread_name!("second"),
                ),
            ],
            core::ptr::NonNull::from(&mut state),
        )
        .expect("thread group");

    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.spawn_stacks.get(), [1_536, 1_024, 0]);
    let state_arg = core::ptr::from_mut(&mut state).cast::<core::ffi::c_void>() as usize;
    assert_eq!(bindings.spawn_args.get(), [state_arg, 0, 0]);
    assert_eq!(state.0, 42);

    threads.terminate_reverse(&api);

    assert_eq!(bindings.terminate_calls.get(), 2);
    assert_eq!(bindings.terminated_threads.get(), [0x20, 0x10, 0]);
}

#[test]
fn thread_api_terminates_first_thread_when_second_spawn_fails() {
    let bindings = FakeThreadBindings::with_spawn_results([0x10, 0, 0]);
    let api = ThreadApi::new(&bindings);
    let mut state = GroupTestState(42);

    let threads = api.spawn_threads(
        [
            crate::ThreadSpec::<GroupTestState>::new::<GroupTestThread>(
                crate::ThreadStackSize::from_bytes(1_536),
                crate::thread_name!("first"),
            ),
            crate::ThreadSpec::<GroupTestState>::stateless::<GroupTestStatelessThread>(
                crate::ThreadStackSize::from_bytes(1_024),
                crate::thread_name!("second"),
            ),
        ],
        core::ptr::NonNull::from(&mut state),
    );

    assert_eq!(threads, None);
    assert_eq!(bindings.spawn_calls.get(), 2);
    assert_eq!(bindings.terminate_calls.get(), 1);
    assert_eq!(bindings.terminated_threads.get()[0], 0x10);
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
        crate::ExtensionRegistration::new(2, 2)
    );
    assert_eq!(bindings.add_calls.get(), 2);
}

#[test]
fn extension_batch_reports_partial_registration_without_short_circuiting() {
    let bindings = FakeBindings::with_add_results([true, false]);
    let lifecycle = PackageLifecycle::new(&bindings);
    let image = NativeImage::new(0x2000);
    let first =
        ExtensionDescriptor::from_handler(crate::extension_name!("ext-rust-a"), stub_handler);
    let second =
        ExtensionDescriptor::from_handler(crate::extension_name!("ext-rust-b"), stub_handler);

    let registration = unsafe { lifecycle.register_extensions_from_image(image, [first, second]) };

    assert_eq!(registration, crate::ExtensionRegistration::new(2, 1));
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
