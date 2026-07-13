//! Testable binding traits and the live firmware binding set.

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use core::ffi::c_char;

use crate::ffi::{CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback};
use crate::types::{PackageArgument, PackageProgramAddress};
use core::ptr::NonNull;
use vescpkg_rs_sys::AppDataHandler;
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
use vescpkg_rs_sys::ExtensionHandler;
#[cfg(not(test))]
use vescpkg_rs_sys::LbmValue;

/// LispBM-related firmware calls required by the SDK lifecycle layer.
pub(crate) trait LbmBindings {
    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    /// # Safety
    /// `name` must be a valid NUL-terminated string for the duration of the call,
    /// and `handler` must obey the firmware's extension callback ABI.
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool;
    /// # Safety
    /// `value` must be a valid firmware-provided LispBM value.
    #[cfg(not(test))]
    unsafe fn decode_i32(&self, value: LbmValue) -> i32;
    /// Return the firmware's true symbol.
    #[cfg(not(test))]
    fn encode_true(&self) -> LbmValue;
    /// Return the firmware's nil symbol.
    #[cfg(not(test))]
    fn encode_nil(&self) -> LbmValue;
}

impl<B: LbmBindings + ?Sized> LbmBindings for &B {
    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        unsafe { (**self).add_extension(name, handler) }
    }

    #[cfg(not(test))]
    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { (**self).decode_i32(value) }
    }

    #[cfg(not(test))]
    fn encode_true(&self) -> LbmValue {
        (**self).encode_true()
    }

    #[cfg(not(test))]
    fn encode_nil(&self) -> LbmValue {
        (**self).encode_nil()
    }
}

/// Firmware calls used by the app-data and system-time helpers.
pub(crate) trait AppDataBindings {
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

    /// Return the package `ARG` pointer stored by the firmware loader.
    ///
    /// C map: VESC package `ARG` calls `VESC_IF->get_arg(PROG_ADDR)` in
    /// `third_party/vesc_pkg_lib/vesc_c_if.h:697-700`; firmware resolves that
    /// by matching the loaded image base in
    /// `third_party/vesc/lispBM/lispif_c_lib.c:151-156`.
    fn arg(&self, prog_addr: PackageProgramAddress) -> Option<PackageArgument>;

    /// Return the typed package-state pointer stored in `ARG` for `prog_addr`.
    ///
    /// # Safety
    ///
    /// The firmware argument must point to a live `T`, and the caller must
    /// coordinate all shared and mutable access to that value.
    unsafe fn arg_state_ptr<T: 'static>(
        &self,
        prog_addr: PackageProgramAddress,
    ) -> Option<NonNull<T>> {
        self.arg(prog_addr)
            .map(|argument| unsafe { argument.state_ptr() })
    }

    /// # Safety
    ///
    /// `data` must point to at least `len` bytes that remain valid for the duration
    /// of the firmware call.
    unsafe fn send_app_data(&self, data: *const u8, len: u32);

    /// Send app-data bytes through the firmware callback.
    fn send_app_data_bytes(&self, data: &[u8]) -> bool {
        let Ok(len) = u32::try_from(data.len()) else {
            return false;
        };
        unsafe { self.send_app_data(data.as_ptr(), len) };
        true
    }
}

impl<B: AppDataBindings + ?Sized> AppDataBindings for &B {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { (**self).set_app_data_handler(handler) }
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        unsafe { (**self).clear_app_data_handler() }
    }

    fn system_time_ticks(&self) -> u32 {
        (**self).system_time_ticks()
    }

    fn arg(&self, prog_addr: PackageProgramAddress) -> Option<PackageArgument> {
        (**self).arg(prog_addr)
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        unsafe { (**self).send_app_data(data, len) }
    }
}

/// Firmware calls used by VESC Tool custom-config callback registration.
pub(crate) trait CustomConfigBindings {
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

    /// Register package-owned custom-config callbacks.
    fn register_custom_config_callbacks(
        &self,
        get_cfg: CustomConfigGet,
        set_cfg: CustomConfigSet,
        get_cfg_xml: CustomConfigXml,
    ) -> bool {
        unsafe { self.register_custom_config(get_cfg, set_cfg, get_cfg_xml) }
    }

    /// Clear package-owned custom-config callbacks.
    ///
    /// Refloat `v1.2.1` calls this during stop at `src/main.c:2403`; the VESC
    /// function-table slot is declared in `vesc_pkg_lib/vesc_c_if.h:553`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid.
    unsafe fn clear_custom_configs(&self) -> bool;

    /// Clear package-owned custom-config callbacks.
    fn clear_custom_config_callbacks(&self) -> bool {
        unsafe { self.clear_custom_configs() }
    }
}

impl<B: CustomConfigBindings + ?Sized> CustomConfigBindings for &B {
    unsafe fn register_custom_config(
        &self,
        get_cfg: CustomConfigGet,
        set_cfg: CustomConfigSet,
        get_cfg_xml: CustomConfigXml,
    ) -> bool {
        unsafe { (**self).register_custom_config(get_cfg, set_cfg, get_cfg_xml) }
    }

    unsafe fn clear_custom_configs(&self) -> bool {
        unsafe { (**self).clear_custom_configs() }
    }
}

/// Firmware calls used by package-owned IMU read callbacks.
pub(crate) trait ImuReadCallbackBindings {
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

    /// Register a package-owned IMU read callback.
    fn set_imu_read_callback_handler(&self, callback: ImuReadCallback) {
        unsafe { self.set_imu_read_callback(callback) }
    }

    /// Clear the package-owned IMU read callback.
    ///
    /// Refloat clears package callbacks during stop at `src/main.c:2401-2403`;
    /// the VESC callback slot is declared in `lispBM/c_libs/vesc_c_if.h:586`.
    ///
    /// # Safety
    ///
    /// Must only be called while the firmware `VESC_IF` table is valid.
    unsafe fn clear_imu_read_callback(&self);

    /// Clear the package-owned IMU read callback.
    fn clear_imu_read_callback_handler(&self) {
        unsafe { self.clear_imu_read_callback() }
    }
}

impl<B: ImuReadCallbackBindings + ?Sized> ImuReadCallbackBindings for &B {
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback) {
        unsafe { (**self).set_imu_read_callback(callback) }
    }

    unsafe fn clear_imu_read_callback(&self) {
        unsafe { (**self).clear_imu_read_callback() }
    }
}

#[cfg(not(test))]
/// Concrete bindings that forward calls into the live `vescpkg-rs-sys` ABI.
pub struct RealBindings;

#[cfg(not(test))]
impl LbmBindings for RealBindings {
    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        unsafe { crate::ffi::lbm_add_extension(name, handler) }
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { crate::ffi::lbm_dec_as_i32(value) }
    }

    fn encode_true(&self) -> LbmValue {
        unsafe { crate::ffi::lbm_enc_sym_true() }
    }

    fn encode_nil(&self) -> LbmValue {
        unsafe { crate::ffi::lbm_enc_sym_nil() }
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
        unsafe { crate::ffi::conf_custom_add_config(get_cfg, set_cfg, get_cfg_xml) }
    }

    unsafe fn clear_custom_configs(&self) -> bool {
        unsafe { crate::ffi::conf_custom_clear_configs() }
    }
}

#[cfg(not(test))]
impl AppDataBindings for RealBindings {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { crate::ffi::vesc_set_app_data_handler(handler) }
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        unsafe { crate::ffi::vesc_clear_app_data_handler() }
    }

    fn system_time_ticks(&self) -> u32 {
        unsafe { crate::ffi::vesc_system_time_ticks() }
    }

    fn arg(&self, prog_addr: PackageProgramAddress) -> Option<PackageArgument> {
        let slot = unsafe { crate::ffi::vesc_get_arg(prog_addr.get()).as_mut()? };
        core::ptr::NonNull::new(*slot).map(PackageArgument::new)
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        unsafe { crate::ffi::vesc_send_app_data(data, len) }
    }
}

#[cfg(not(test))]
impl ImuReadCallbackBindings for RealBindings {
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback) {
        unsafe { crate::ffi::vesc_set_imu_read_callback(callback) }
    }

    unsafe fn clear_imu_read_callback(&self) {
        unsafe { crate::ffi::vesc_clear_imu_read_callback() }
    }
}
