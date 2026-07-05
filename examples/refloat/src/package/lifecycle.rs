use super::protocol::process_refloat_app_data;
use super::{refloat_stop_handler, register_refloat_custom_config};
use crate::domain::{RefloatAllDataPayloads, RefloatAllDataRequest};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, CustomConfigBindings,
    ImuReadCallbackBindings, LoopbackLifecycle, PackageStart, ffi,
};

/// Refloat app-data lifecycle wiring.
pub struct RefloatPackageLifecycle<B> {
    lifecycle: LoopbackLifecycle<B>,
}

impl<B: AppDataBindings> RefloatPackageLifecycle<B> {
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
    pub fn install(
        &self,
        start: &mut PackageStart,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = self
            .lifecycle
            .install(start, refloat_stop_handler(), handler);
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
        self.lifecycle.clear_package_callbacks()
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
        self.lifecycle.send_app_data(bytes)
    }
}

impl<B: AppDataBindings + CustomConfigBindings> RefloatPackageLifecycle<B> {
    /// Install Refloat custom config and app-data callbacks.
    ///
    /// Upstream registers custom config before app-data at `third_party/refloat/src/main.c:2456-2457`,
    /// after loader metadata receives `stop`/`Data *` at `third_party/refloat/src/main.c:2431-2432`.
    ///
    pub fn install_refloat_callbacks(
        &self,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = register_refloat_custom_config(self.bindings());
        self.lifecycle.register_app_data_handler(handler)
    }
}
