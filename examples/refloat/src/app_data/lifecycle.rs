use super::protocol::process_refloat_app_data;
use super::{RefloatAppDataState, register_refloat_custom_config, stop_refloat_app_data};
use crate::domain::{RefloatAllDataPayloads, RefloatAllDataRequest};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, CustomConfigBindings,
    ImuReadCallbackBindings, LoopbackLifecycle, ffi,
};

/// Refloat app-data lifecycle wiring.
pub struct RefloatAppDataLifecycle<B> {
    lifecycle: LoopbackLifecycle<B>,
}

impl<B: AppDataBindings> RefloatAppDataLifecycle<B> {
    /// Build Refloat app-data lifecycle wiring from firmware bindings.
    pub fn new(bindings: B) -> Self {
        Self {
            lifecycle: LoopbackLifecycle::new(bindings),
        }
    }

    /// Return the wrapped firmware bindings.
    pub fn bindings(&self) -> &B {
        self.lifecycle.bindings()
    }

    /// Install Refloat stop cleanup and app-data handler.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. The supplied
    /// handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install(
        &self,
        info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        unsafe {
            let _ = self.lifecycle.install(info, stop_refloat_app_data, handler);
        }
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Install Refloat stop cleanup and package-owned state without callbacks.
    ///
    /// Upstream stores `stop` and `Data *` in loader metadata at
    /// `third_party/refloat/src/main.c:2431-2432`, before registering custom config/app-data/LispBM
    /// callbacks at `third_party/refloat/src/main.c:2455-2459`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` must
    /// remain valid until the firmware stops the package. The supplied handler is
    /// not registered here; it is only passed through the SDK lifecycle install
    /// shape whose current implementation records the stop hook.
    pub unsafe fn install_refloat_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> bool {
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::from_mut(state).cast();
        }
        unsafe { self.lifecycle.install(info, stop_refloat_app_data, handler) }
    }

    /// Install Refloat state, stop cleanup, and app-data handler.
    ///
    /// Upstream stores `Data *`/`stop` in loader metadata at
    /// `third_party/refloat/src/main.c:2431-2432`; app-data registration follows later at
    /// `third_party/refloat/src/main.c:2456`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` and
    /// `handler` must remain valid until firmware clears/replaces the handler
    /// and stops the package.
    pub unsafe fn install_with_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = unsafe { self.install_refloat_state(info, state, handler) };
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Clear Refloat callbacks during package stop.
    ///
    /// Refloat `v1.2.1` clears IMU/app-data/custom config callbacks at
    /// `third_party/refloat/src/main.c:2401-2403`.
    pub fn stop(&self) -> Result<(), AppDataHandlerRegistrationError>
    where
        B: CustomConfigBindings + ImuReadCallbackBindings,
    {
        unsafe {
            self.lifecycle.bindings().clear_imu_read_callback();
        }
        let app_data_result = self.lifecycle.clear_app_data_handler();
        unsafe {
            let _ = self.lifecycle.bindings().clear_custom_configs();
        }
        app_data_result
    }

    /// Process one Refloat app-data packet and send a response when accepted.
    #[inline(never)]
    pub fn send_response(&self, payloads: &RefloatAllDataPayloads, bytes: &[u8]) -> bool {
        let Some(response) = process_refloat_app_data(payloads, bytes) else {
            return false;
        };
        self.send_response_bytes(response.as_bytes())
    }

    /// Encode and send one parsed Refloat all-data response.
    #[inline(never)]
    pub fn send_all_data_response(
        &self,
        payloads: &RefloatAllDataPayloads,
        request: RefloatAllDataRequest,
    ) -> bool {
        let response = payloads.encode_response(request);
        self.send_response_bytes(response.as_bytes())
    }

    pub(super) fn send_response_bytes(&self, bytes: &[u8]) -> bool {
        unsafe {
            self.lifecycle
                .send_app_data(bytes.as_ptr(), bytes.len() as u32)
        };
        true
    }
}

impl<B: AppDataBindings + CustomConfigBindings> RefloatAppDataLifecycle<B> {
    /// Install Refloat custom config and app-data callbacks.
    ///
    /// Upstream registers custom config before app-data at `third_party/refloat/src/main.c:2456-2457`,
    /// after loader metadata receives `stop`/`Data *` at `third_party/refloat/src/main.c:2431-2432`.
    ///
    /// # Safety
    ///
    /// The supplied handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install_refloat_callbacks(
        &self,
        _info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = register_refloat_custom_config(self.bindings());
        self.lifecycle.register_app_data_handler(handler)
    }

    /// Install Refloat state plus custom config and app-data callbacks.
    ///
    /// Upstream stores `Data *` in `info->arg` at `third_party/refloat/src/main.c:2432` before
    /// registering custom config and app-data at `third_party/refloat/src/main.c:2456-2457`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. `state` and
    /// `handler` must remain valid until firmware clears/replaces the handler
    /// and stops the package.
    pub unsafe fn install_refloat_callbacks_with_state(
        &self,
        info: *mut ffi::LibInfo,
        state: &mut RefloatAppDataState,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = unsafe { self.install_refloat_state(info, state, handler) };
        unsafe { self.install_refloat_callbacks(info, handler) }
    }
}
