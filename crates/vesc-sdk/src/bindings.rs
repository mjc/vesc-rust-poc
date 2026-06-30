//! Testable binding traits and the live firmware binding set.

use core::ffi::c_char;

use vesc_ffi::{AppDataHandler, ExtensionHandler, LbmValue};

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
    /// `handler` must remain valid until it is replaced or cleared. Pass null to clear.
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool;

    /// Return the current firmware tick counter.
    fn system_time_ticks(&self) -> u32;

    /// # Safety
    ///
    /// `data` must point to at least `len` bytes that remain valid for the duration
    /// of the firmware call.
    unsafe fn send_app_data(&self, data: *const u8, len: u32);
}

#[cfg(not(test))]
/// Concrete bindings that forward calls into the live `vesc-ffi` ABI.
pub struct RealBindings;

#[cfg(not(test))]
impl LbmBindings for RealBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        unsafe { vesc_ffi::raw::lbm_add_extension(name, handler) }
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { vesc_ffi::raw::lbm_dec_as_i32(value) }
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { vesc_ffi::raw::lbm_enc_i(value) }
    }

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        unsafe { vesc_ffi::raw::lbm_is_number(value) }
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        unsafe { vesc_ffi::raw::lbm_enc_sym_eerror() }
    }
}

#[cfg(not(test))]
impl AppDataBindings for RealBindings {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { vesc_ffi::raw::vesc_set_app_data_handler(handler) }
    }

    fn system_time_ticks(&self) -> u32 {
        unsafe { vesc_ffi::raw::vesc_system_time_ticks() }
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        unsafe { vesc_ffi::raw::vesc_send_app_data(data, len) }
    }
}
