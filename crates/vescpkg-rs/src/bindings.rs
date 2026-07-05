//! Testable binding traits and the live firmware binding set.

use core::ffi::c_char;

use vescpkg_rs_sys::raw::{CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback};
use vescpkg_rs_sys::{AppDataHandler, ExtensionHandler, LbmValue};

/// LispBM-related firmware calls required by the SDK lifecycle layer.
pub trait LbmBindings {
    /// # Safety
    /// `name` must be a valid NUL-terminated string for the duration of the call,
    /// and `handler` must obey the firmware's extension callback ABI.
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool;
    /// # Safety
    /// `value` must be a valid firmware-provided LispBM value.
    unsafe fn decode_i32(&self, value: LbmValue) -> i32;
    /// # Safety
    /// The returned value is owned by the caller as an opaque LispBM value.
    unsafe fn encode_i32(&self, value: i32) -> LbmValue;
    /// # Safety
    /// `value` must be a valid firmware-provided LispBM value.
    unsafe fn is_number(&self, value: LbmValue) -> bool;
    /// # Safety
    /// The returned value is the firmware's eval-error symbol.
    unsafe fn encode_eval_error(&self) -> LbmValue;
}

/// Firmware calls used by the app-data and system-time helpers.
pub trait AppDataBindings {
    /// # Safety
    /// `handler` must remain valid until it is replaced or cleared.
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool;

    /// Clear the current app-data handler.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid.
    unsafe fn clear_app_data_handler(&self) -> bool;

    /// Return the current firmware tick counter.
    fn system_time_ticks(&self) -> u32;

    /// # Safety
    ///
    /// `data` must point to at least `len` bytes that remain valid for the duration
    /// of the firmware call.
    unsafe fn send_app_data(&self, data: *const u8, len: u32);
}

/// Firmware calls used by VESC Tool custom-config callback registration.
pub trait CustomConfigBindings {
    /// Register package-owned custom-config callbacks.
    ///
    /// Refloat `v1.2.1` registers `get_cfg`, `set_cfg`, and `get_cfg_xml` at
    /// `src/main.c:2456`; the VESC function-table slot is declared in
    /// `vesc_pkg_lib/vesc_c_if.h:549-552`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid. The
    /// callbacks must remain valid until package stop clears them or firmware
    /// replaces them.
    unsafe fn register_custom_config(
        &self,
        get_cfg: CustomConfigGet,
        set_cfg: CustomConfigSet,
        get_cfg_xml: CustomConfigXml,
    ) -> bool;

    /// Clear package-owned custom-config callbacks.
    ///
    /// Refloat `v1.2.1` calls this during stop at `src/main.c:2403`; the VESC
    /// function-table slot is declared in `vesc_pkg_lib/vesc_c_if.h:553`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid.
    unsafe fn clear_custom_configs(&self) -> bool;
}

/// Firmware calls used by package-owned IMU read callbacks.
pub trait ImuReadCallbackBindings {
    /// Register a package-owned IMU read callback.
    ///
    /// Refloat `v1.2.1` registers `imu_ref_callback` at `src/main.c:2455`;
    /// that callback updates the balance filter at `src/main.c:760-764`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid. The
    /// callback must remain valid until package stop clears it or firmware
    /// replaces it.
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback);

    /// Clear the package-owned IMU read callback.
    ///
    /// Refloat clears package callbacks during stop at `src/main.c:2401-2403`;
    /// the VESC callback slot is declared in `lispBM/c_libs/vesc_c_if.h:586`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid.
    unsafe fn clear_imu_read_callback(&self);
}

#[cfg(not(test))]
/// Concrete bindings that forward calls into the live `vescpkg-rs-sys` ABI.
pub struct RealBindings;

#[cfg(not(test))]
impl LbmBindings for RealBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        unsafe { vescpkg_rs_sys::raw::lbm_add_extension(name, handler) }
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { vescpkg_rs_sys::raw::lbm_dec_as_i32(value) }
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { vescpkg_rs_sys::raw::lbm_enc_i(value) }
    }

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        unsafe { vescpkg_rs_sys::raw::lbm_is_number(value) }
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        unsafe { vescpkg_rs_sys::raw::lbm_enc_sym_eerror() }
    }
}

#[cfg(not(test))]
impl CustomConfigBindings for RealBindings {
    unsafe fn register_custom_config(
        &self,
        get_cfg: CustomConfigGet,
        set_cfg: CustomConfigSet,
        get_cfg_xml: CustomConfigXml,
    ) -> bool {
        unsafe { vescpkg_rs_sys::raw::conf_custom_add_config(get_cfg, set_cfg, get_cfg_xml) }
    }

    unsafe fn clear_custom_configs(&self) -> bool {
        unsafe { vescpkg_rs_sys::raw::conf_custom_clear_configs() }
    }
}

#[cfg(not(test))]
impl AppDataBindings for RealBindings {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { vescpkg_rs_sys::raw::vesc_set_app_data_handler(handler) }
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        unsafe { vescpkg_rs_sys::raw::vesc_clear_app_data_handler() }
    }

    fn system_time_ticks(&self) -> u32 {
        unsafe { vescpkg_rs_sys::raw::vesc_system_time_ticks() }
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        unsafe { vescpkg_rs_sys::raw::vesc_send_app_data(data, len) }
    }
}

#[cfg(not(test))]
impl ImuReadCallbackBindings for RealBindings {
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback) {
        unsafe { vescpkg_rs_sys::raw::vesc_set_imu_read_callback(callback) }
    }

    unsafe fn clear_imu_read_callback(&self) {
        unsafe { vescpkg_rs_sys::raw::vesc_clear_imu_read_callback() }
    }
}
