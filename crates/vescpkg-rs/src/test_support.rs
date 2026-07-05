//! Host-side fake firmware bindings for unit tests in dependent crates.

use core::cell::Cell;
use core::ffi::c_char;

use crate::bindings::{
    AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings, LbmBindings,
};
use vescpkg_rs_sys::raw::{CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback};
use vescpkg_rs_sys::{AppDataHandler, ExtensionHandler, LbmValue};

pub use crate::imu::test_support::FakeImuBindings;
pub use crate::motor::test_support::{FakeMotorControlBindings, FakeMotorTelemetryBindings};
pub use crate::thread::test_support::FakeThreadBindings;

/// Fake extension registration bindings used by SDK tests.
pub struct FakeBindings {
    /// Number of extension add calls observed.
    pub add_calls: Cell<usize>,
    /// Number of decode callback calls observed.
    pub decode_calls: Cell<usize>,
    /// Number of encode callback calls observed.
    pub encode_calls: Cell<usize>,
    /// Last extension name pointer passed to registration.
    pub last_name: Cell<usize>,
    /// Last handler pointer passed to registration.
    pub last_handler: Cell<usize>,
    add_results: Cell<[bool; 2]>,
}

impl Default for FakeBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBindings {
    /// Creates fake bindings that accept both extension registrations.
    pub fn new() -> Self {
        Self::with_add_results([true, true])
    }

    /// Creates fake bindings that reject extension registrations.
    pub fn rejecting() -> Self {
        Self::with_add_results([false, false])
    }

    /// Creates fake bindings with explicit add results for two registrations.
    pub fn with_add_results(add_results: [bool; 2]) -> Self {
        Self {
            add_calls: Cell::new(0),
            decode_calls: Cell::new(0),
            encode_calls: Cell::new(0),
            last_name: Cell::new(0),
            last_handler: Cell::new(0),
            add_results: Cell::new(add_results),
        }
    }
}

impl LbmBindings for FakeBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        self.add_calls.set(self.add_calls.get() + 1);
        self.last_name.set(name as usize);
        self.last_handler.set(handler as usize);
        let index = self.add_calls.get().saturating_sub(1).min(1);
        self.add_results.get()[index]
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        self.decode_calls.set(self.decode_calls.get() + 1);
        value.0 as i32
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        self.encode_calls.set(self.encode_calls.get() + 1);
        LbmValue(value as u32)
    }

    unsafe fn is_number(&self, _value: LbmValue) -> bool {
        true
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        LbmValue(0xffff_ffff)
    }

    fn encode_true(&self) -> LbmValue {
        LbmValue(1)
    }

    fn encode_nil(&self) -> LbmValue {
        LbmValue(0)
    }
}

/// Fake app-data bindings used by lifecycle and loopback runtime tests.
pub struct FakeAppDataBindings {
    /// Number of app-data handler invocations observed.
    pub handler_calls: Cell<usize>,
    /// Tick count returned by the fake timer binding.
    pub ticks: Cell<u32>,
    /// Number of app-data send calls observed.
    pub send_calls: Cell<usize>,
    /// Last app-data handler pointer passed to registration.
    pub last_handler: Cell<usize>,
    /// Last outbound data pointer passed to send.
    pub last_data: Cell<usize>,
    /// Last outbound data length passed to send.
    pub last_len: Cell<u32>,
    /// Number of custom-config registration calls observed.
    pub custom_config_register_calls: Cell<usize>,
    /// Number of custom-config clear calls observed.
    pub custom_config_clear_calls: Cell<usize>,
    /// Number of IMU read callback registration calls observed.
    pub imu_read_callback_calls: Cell<usize>,
    /// Last IMU read callback pointer passed to registration.
    pub last_imu_read_callback: Cell<usize>,
    /// Fake package ARG pointer returned by the app-data binding.
    pub app_data_arg: Cell<usize>,
    set_handler_result: Cell<bool>,
    clear_handler_result: Cell<bool>,
    register_custom_config_result: Cell<bool>,
    clear_custom_configs_result: Cell<bool>,
}

#[derive(Clone, Copy)]
enum FirmwareCallResult {
    Accept,
    Reject,
}

impl FirmwareCallResult {
    const fn from_bool(value: bool) -> Self {
        if value { Self::Accept } else { Self::Reject }
    }

    const fn accepted(self) -> bool {
        matches!(self, Self::Accept)
    }
}

#[derive(Clone, Copy)]
struct FakeAppDataResults {
    set_handler: FirmwareCallResult,
    clear_handler: FirmwareCallResult,
    register_custom_config: FirmwareCallResult,
    clear_custom_configs: FirmwareCallResult,
}

impl FakeAppDataResults {
    const ACCEPT_ALL: Self = Self {
        set_handler: FirmwareCallResult::Accept,
        clear_handler: FirmwareCallResult::Accept,
        register_custom_config: FirmwareCallResult::Accept,
        clear_custom_configs: FirmwareCallResult::Accept,
    };
}

impl Default for FakeAppDataBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeAppDataBindings {
    /// Creates fake app-data bindings with zero timer ticks.
    pub fn new() -> Self {
        Self::with_ticks(0)
    }

    /// Creates fake app-data bindings returning `ticks` from the timer.
    pub fn with_ticks(ticks: u32) -> Self {
        Self::with_ticks_and_results(ticks, FakeAppDataResults::ACCEPT_ALL)
    }

    /// Creates fake app-data bindings with an explicit handler registration result.
    pub fn with_set_handler_result(set_handler_result: bool) -> Self {
        Self::with_ticks_and_results(
            0,
            FakeAppDataResults {
                set_handler: FirmwareCallResult::from_bool(set_handler_result),
                ..FakeAppDataResults::ACCEPT_ALL
            },
        )
    }

    /// Creates fake app-data bindings with an explicit handler clear result.
    pub fn with_clear_handler_result(clear_handler_result: bool) -> Self {
        Self::with_ticks_and_results(
            0,
            FakeAppDataResults {
                clear_handler: FirmwareCallResult::from_bool(clear_handler_result),
                ..FakeAppDataResults::ACCEPT_ALL
            },
        )
    }

    /// Creates fake app-data bindings with an explicit custom-config registration result.
    pub fn with_register_custom_config_result(register_custom_config_result: bool) -> Self {
        Self::with_ticks_and_results(
            0,
            FakeAppDataResults {
                register_custom_config: FirmwareCallResult::from_bool(
                    register_custom_config_result,
                ),
                ..FakeAppDataResults::ACCEPT_ALL
            },
        )
    }

    /// Creates fake app-data bindings with an explicit custom-config clear result.
    pub fn with_clear_custom_configs_result(clear_custom_configs_result: bool) -> Self {
        Self::with_ticks_and_results(
            0,
            FakeAppDataResults {
                clear_custom_configs: FirmwareCallResult::from_bool(clear_custom_configs_result),
                ..FakeAppDataResults::ACCEPT_ALL
            },
        )
    }

    fn with_ticks_and_results(ticks: u32, results: FakeAppDataResults) -> Self {
        Self {
            handler_calls: Cell::new(0),
            ticks: Cell::new(ticks),
            send_calls: Cell::new(0),
            last_handler: Cell::new(0),
            last_data: Cell::new(0),
            last_len: Cell::new(0),
            custom_config_register_calls: Cell::new(0),
            custom_config_clear_calls: Cell::new(0),
            imu_read_callback_calls: Cell::new(0),
            last_imu_read_callback: Cell::new(0),
            app_data_arg: Cell::new(0),
            set_handler_result: Cell::new(results.set_handler.accepted()),
            clear_handler_result: Cell::new(results.clear_handler.accepted()),
            register_custom_config_result: Cell::new(results.register_custom_config.accepted()),
            clear_custom_configs_result: Cell::new(results.clear_custom_configs.accepted()),
        }
    }
}

impl AppDataBindings for FakeAppDataBindings {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(handler as *const () as usize);
        self.set_handler_result.get()
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(0);
        self.clear_handler_result.get()
    }

    fn system_time_ticks(&self) -> u32 {
        self.ticks.get()
    }

    fn app_data_arg(&self, _prog_addr: u32) -> Option<core::ptr::NonNull<core::ffi::c_void>> {
        core::ptr::NonNull::new(self.app_data_arg.get() as *mut core::ffi::c_void)
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        self.send_calls.set(self.send_calls.get() + 1);
        self.last_data.set(data as usize);
        self.last_len.set(len);
    }
}

impl CustomConfigBindings for FakeAppDataBindings {
    unsafe fn register_custom_config(
        &self,
        _get_cfg: CustomConfigGet,
        _set_cfg: CustomConfigSet,
        _get_cfg_xml: CustomConfigXml,
    ) -> bool {
        self.custom_config_register_calls
            .set(self.custom_config_register_calls.get() + 1);
        self.register_custom_config_result.get()
    }

    unsafe fn clear_custom_configs(&self) -> bool {
        self.custom_config_clear_calls
            .set(self.custom_config_clear_calls.get() + 1);
        self.clear_custom_configs_result.get()
    }
}

impl ImuReadCallbackBindings for FakeAppDataBindings {
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback
            .set(callback as *const () as usize);
    }

    unsafe fn clear_imu_read_callback(&self) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback.set(0);
    }
}

/// C ABI stubs linked by host-side tests.
pub mod stubs {
    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real extension handler ABI.
    pub unsafe extern "C" fn extension_handler(_args: *mut u32, _count: u32) -> u32 {
        0
    }

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real stop handler ABI.
    pub unsafe extern "C" fn stop_handler(_arg: *mut core::ffi::c_void) {}

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real app-data handler ABI.
    pub unsafe extern "C" fn app_data_handler(_data: *mut u8, _len: u32) {}

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real IMU callback ABI.
    pub unsafe extern "C" fn imu_read_callback(
        _acc: *mut f32,
        _gyro: *mut f32,
        _mag: *mut f32,
        _dt: f32,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::{FakeAppDataBindings, FakeBindings, stubs};
    use crate::{AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings, LbmBindings};
    use vescpkg_rs_sys::raw::ImuReadCallback;
    use vescpkg_rs_sys::{ExtensionHandler, LbmValue};

    #[test]
    fn fake_bindings_default_and_rejecting_paths() {
        let accepting = FakeBindings::default();
        let rejecting = FakeBindings::rejecting();

        unsafe {
            assert!(accepting.add_extension(
                c"ext-a".as_ptr(),
                stubs::extension_handler as ExtensionHandler
            ));
            assert!(!rejecting.add_extension(
                c"ext-b".as_ptr(),
                stubs::extension_handler as ExtensionHandler
            ));
        }

        assert_eq!(accepting.add_calls.get(), 1);
        assert_eq!(rejecting.add_calls.get(), 1);
        unsafe {
            assert_eq!(accepting.decode_i32(LbmValue(7)), 7);
            assert_eq!(accepting.encode_i32(9), LbmValue(9));
            assert!(accepting.is_number(LbmValue(1)));
            assert_eq!(accepting.encode_eval_error(), LbmValue(0xffff_ffff));
        }
    }

    #[test]
    fn fake_app_data_bindings_track_handler_send_and_ticks() {
        let bindings = FakeAppDataBindings::with_ticks(999);
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(bindings.system_time_ticks(), 999);
        unsafe {
            assert!(bindings.set_app_data_handler(handler));
            bindings.send_app_data([1_u8, 2].as_ptr(), 2);
            assert!(bindings.clear_app_data_handler());
        }

        assert_eq!(bindings.handler_calls.get(), 2);
        assert_eq!(bindings.send_calls.get(), 1);
        assert_eq!(bindings.last_len.get(), 2);
        assert_eq!(bindings.last_handler.get(), 0);
    }

    #[test]
    fn fake_app_data_bindings_track_custom_config_registration() {
        let bindings = FakeAppDataBindings::new();

        unsafe extern "C" fn get_cfg(_data: *mut u8, _is_default: bool) -> core::ffi::c_int {
            0
        }

        unsafe extern "C" fn set_cfg(_data: *mut u8) -> bool {
            true
        }

        unsafe extern "C" fn get_cfg_xml(_data: *mut *mut u8) -> core::ffi::c_int {
            0
        }

        unsafe {
            // Refloat v1.2.1 registers these three callbacks at `src/main.c:2456`;
            // the VESC function-table slots are declared in
            // `vesc_pkg_lib/vesc_c_if.h:549-553`.
            assert!(bindings.register_custom_config(get_cfg, set_cfg, get_cfg_xml));
            assert!(bindings.clear_custom_configs());
        }

        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn fake_app_data_bindings_track_imu_read_callback_registration() {
        let bindings = FakeAppDataBindings::new();

        unsafe {
            // Refloat v1.2.1 registers `imu_ref_callback` at `src/main.c:2455`
            // and clears it during stop at `src/main.c:2401`.
            bindings.set_imu_read_callback(stubs::imu_read_callback as ImuReadCallback);
            bindings.clear_imu_read_callback();
        }

        assert_eq!(bindings.imu_read_callback_calls.get(), 2);
        assert_eq!(bindings.last_imu_read_callback.get(), 0);
    }

    #[test]
    fn fake_app_data_bindings_can_reject_custom_config_registration() {
        let bindings = FakeAppDataBindings::with_register_custom_config_result(false);

        unsafe extern "C" fn get_cfg(_data: *mut u8, _is_default: bool) -> core::ffi::c_int {
            0
        }

        unsafe extern "C" fn set_cfg(_data: *mut u8) -> bool {
            true
        }

        unsafe extern "C" fn get_cfg_xml(_data: *mut *mut u8) -> core::ffi::c_int {
            0
        }

        unsafe {
            assert!(!bindings.register_custom_config(get_cfg, set_cfg, get_cfg_xml));
        }

        assert_eq!(bindings.custom_config_register_calls.get(), 1);
    }

    #[test]
    fn fake_app_data_bindings_can_reject_custom_config_clear() {
        let bindings = FakeAppDataBindings::with_clear_custom_configs_result(false);

        unsafe {
            assert!(!bindings.clear_custom_configs());
        }

        assert_eq!(bindings.custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn stub_handlers_are_callable() {
        unsafe {
            stubs::extension_handler(core::ptr::null_mut(), 0);
            stubs::stop_handler(core::ptr::null_mut());
            stubs::app_data_handler(core::ptr::null_mut(), 0);
        }
    }
}
