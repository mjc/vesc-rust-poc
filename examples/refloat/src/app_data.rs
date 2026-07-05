//! Refloat app-data packet processing.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) anchors:
//! - `src/main.c:2143-2295` handles incoming app-data commands.
//! - `src/main.c:2334-2403` owns custom config get/set/XML and stop cleanup.
//! - `src/main.c:2456-2457` registers custom config and app-data handlers.
//!
//! This module is currently disconnected from the hardware candidate's package
//! init path while the package-corruption failure is isolated. Reconnecting it
//! is not just a handler call: upstream shares the same `Data *` through `ARG`
//! for app-data, custom config, BMS, threads, and stop cleanup.

use crate::domain::{
    RefloatAllDataMode3Payload, RefloatAllDataMode4Payload, RefloatAllDataPayloads,
    RefloatAllDataRequest, RefloatAllDataResponse, RefloatAppDataCommand, RefloatFirmwareFaultCode,
    RefloatMode, RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
    RefloatRealtimeMotorTemperatures, RefloatRunState,
};
use core::ffi::c_int;
use vescpkg_rs::prelude::{BatteryCurrent, BatteryVoltage, Current, Voltage};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, CustomConfigBindings, LoopbackLifecycle,
    MotorTelemetryApi, MotorTelemetryBindings, ffi,
};

/// Refloat v1.2.1 generated custom-config XML blob.
///
/// Upstream generates this from `src/conf/settings.xml` via `src/Makefile:28-31`
/// and exposes `data_refloatconfig_` through `get_cfg_xml` at
/// `src/main.c:2388-2396`.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_config_xml")
)]
#[used]
static REFLOAT_CONFIG_XML: [u8; 25_723] = *include_bytes!("conf/refloatconfig.dat");

/// Refloat v1.2.1 generated serialized default custom config.
///
/// Upstream `get_cfg(..., is_default=true)` allocates `RefloatConfig`, fills
/// defaults, serializes it, then frees it at `src/main.c:2335-2356`.
/// `src/Makefile:28-31` generates the format from `src/conf/settings.xml`;
/// generated `conf/confparser.h:11-12` defines signature `2427955642` and
/// serialized length `276`, while generated `conf/confparser.c:8-178` and
/// `conf/confparser.c:363-531` serialize the default values.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_default_config")
)]
#[used]
static REFLOAT_DEFAULT_CONFIG: [u8; 276] = *include_bytes!("conf/default_config.dat");
const REFLOAT_CONFIG_SIGNATURE_BYTES: [u8; 4] = [0x90, 0xb7, 0xa9, 0xba];
// Upstream defines `disabled` in `src/conf/settings.xml:3890-3902`; its
// `<ser>disabled</ser>` entry at `src/conf/settings.xml:4064` lands at byte
// 243 in the 276-byte generated config image.
const REFLOAT_CONFIG_DISABLED_OFFSET: usize = 243;
// Upstream defines `meta.is_default` in `src/conf/settings.xml:3903-3914`; its
// `<ser>meta.is_default</ser>` entry at `src/conf/settings.xml:4083` lands at
// the final byte in the generated config image.
const REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET: usize = 275;

/// Process one Refloat app-data packet from a typed all-data payload snapshot.
pub fn process_refloat_app_data(
    payloads: RefloatAllDataPayloads,
    bytes: &[u8],
) -> Option<RefloatAllDataResponse> {
    let request = RefloatAllDataRequest::parse(bytes).ok()?;
    Some(payloads.encode_response(request))
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn handle_refloat_app_data_packet<B: AppDataBindings, M: MotorTelemetryBindings>(
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    data: *mut u8,
    len: u32,
) -> bool {
    let Some(data) = core::ptr::NonNull::new(data) else {
        return false;
    };
    let Ok(len) = usize::try_from(len) else {
        return false;
    };
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr().cast_const(), len) };
    state.handle_packet_with_telemetry(lifecycle, telemetry, bytes)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn prog_addr() -> u32 {
    let address: u32;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {prog_ptr}",
            address = out(reg) address,
            prog_ptr = sym crate::init::prog_ptr,
            options(nomem, nostack, preserves_flags),
        );
    }
    address
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_app_data_handler() -> ffi::AppDataHandler {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {handler}",
            address = out(reg) address,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
        core::mem::transmute::<usize, ffi::AppDataHandler>(address | 1)
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn refloat_state_from_arg() -> Option<&'static mut RefloatAppDataState> {
    let arg_slot = unsafe { ffi::raw::vesc_get_arg(prog_addr()) };
    if arg_slot.is_null() {
        return None;
    }
    let arg_slot = unsafe { arg_slot.as_mut()? };
    let state = (*arg_slot).cast::<RefloatAppDataState>();
    if state.is_null() {
        return None;
    }
    unsafe { state.as_mut() }
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// Upstream registers `on_command_received` in `src/main.c:2457`; the handler
/// dispatches command IDs in `src/main.c:2143-2295`.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn refloat_handle_app_data(data: *mut u8, len: u32) {
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
    let _ = unsafe { handle_refloat_app_data_packet(state, &lifecycle, &telemetry, data, len) };
}

/// Install source-startup Refloat app-data state through the supplied lifecycle.
///
/// Upstream allocates `Data`, stores it in loader metadata, and registers the
/// stop hook at `src/main.c:2419-2432`; handler registration follows at
/// `src/main.c:2457`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `state` and
/// `handler` must remain valid until firmware clears/replaces the handler and
/// stops the package.
#[cfg(test)]
pub(crate) unsafe fn install_refloat_startup_app_data_with<
    B: AppDataBindings + CustomConfigBindings,
>(
    info: *mut ffi::LibInfo,
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    *state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());
    unsafe { lifecycle.install_refloat_callbacks_with_state(info, state, handler) }.is_ok()
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn clear_refloat_app_data_loader_info(info: *mut ffi::LibInfo) {
    if let Some(info) = unsafe { info.as_mut() } {
        info.arg = core::ptr::null_mut();
        info.stop_fun = None;
    }
}

/// Allocate and install source-startup Refloat app-data state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `src/main.c:2419`, runs
/// `data_init` at `src/main.c:2424`, and stores the same pointer in
/// `info->arg` at `src/main.c:2432`. This Rust allocation path only installs a
/// narrow app-data snapshot, so it is not equivalent to upstream's shared
/// Refloat state.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `handler` must
/// remain valid until firmware clears/replaces the handler and stops the package.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) unsafe fn allocate_refloat_startup_app_data_with<
    A: vescpkg_rs::AllocBindings,
    B: AppDataBindings + CustomConfigBindings,
>(
    info: *mut ffi::LibInfo,
    allocator: &vescpkg_rs::FirmwareAllocator<'_, A>,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    let Ok(mut allocation) = allocator.allocate_for::<RefloatAppDataState>(1) else {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    };
    let state = allocation.as_mut_ptr();
    unsafe {
        state.write(RefloatAppDataState::new(
            RefloatAllDataPayloads::source_startup(),
        ))
    };
    let state = unsafe { &mut *state };

    if unsafe { lifecycle.install_refloat_callbacks_with_state(info, state, handler) }.is_err() {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    }

    let _ = allocation.into_raw();
    true
}

/// Allocate and install Refloat startup app-data state using firmware memory.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data(info: *mut ffi::LibInfo) -> bool {
    let alloc_bindings = vescpkg_rs::RealBindings;
    let allocator = vescpkg_rs::FirmwareAllocator::new(&alloc_bindings);
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let handler = runtime_refloat_app_data_handler();
    unsafe { allocate_refloat_startup_app_data_with(info, &allocator, &lifecycle, handler) }
}

/// Register Refloat custom-config callbacks with VESC Tool.
///
/// Upstream registers `get_cfg`, `set_cfg`, and `get_cfg_xml` at
/// `src/main.c:2456`; those callbacks are implemented at `src/main.c:2334-2396`.
/// The Rust port does not yet generate or serialize upstream `RefloatConfig`, so
/// these callbacks report no config payload instead of pretending to be the full
/// confparser path.
pub fn register_refloat_custom_config<B: CustomConfigBindings>(bindings: &B) -> bool {
    unsafe {
        bindings.register_custom_config(refloat_get_cfg, refloat_set_cfg, refloat_get_cfg_xml)
    }
}

unsafe extern "C" fn refloat_get_cfg(buffer: *mut u8, is_default: bool) -> c_int {
    let state = unsafe { runtime_refloat_config_state() };
    refloat_get_cfg_with_state(buffer, is_default, state)
}

fn refloat_get_cfg_with_state(
    buffer: *mut u8,
    is_default: bool,
    state: Option<&RefloatAppDataState>,
) -> c_int {
    if !is_default {
        // Upstream serializes `d->float_conf` at `src/main.c:2347-2350`;
        // `data_init` first populates it from EEPROM or generated defaults at
        // `src/main.c:1160-1185`. The Rust state stores the serialized image
        // until the typed `RefloatConfig` parser/deserializer is ported.
        let Some(state) = state else {
            return 0;
        };
        return copy_refloat_config(buffer, state.serialized_config());
    }

    // Upstream default path is `src/main.c:2339-2350`: allocate config, call
    // `confparser_set_defaults_refloatconfig`, then
    // `confparser_serialize_refloatconfig`.
    copy_refloat_config(buffer, &REFLOAT_DEFAULT_CONFIG)
}

fn copy_refloat_config(buffer: *mut u8, config: &[u8; 276]) -> c_int {
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return 0;
    };

    unsafe { core::ptr::copy_nonoverlapping(config.as_ptr(), buffer.as_ptr(), config.len()) };
    config.len() as c_int
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn runtime_refloat_config_state() -> Option<&'static RefloatAppDataState> {
    let state = unsafe { refloat_state_from_arg()? };
    Some(&*state)
}

#[cfg(any(test, not(target_arch = "arm")))]
unsafe fn runtime_refloat_config_state() -> Option<&'static RefloatAppDataState> {
    None
}

unsafe extern "C" fn refloat_set_cfg(buffer: *mut u8) -> bool {
    let state = unsafe { runtime_refloat_config_state_mut() };
    refloat_set_cfg_with_state(buffer, state)
}

fn refloat_set_cfg_with_state(buffer: *mut u8, state: Option<&mut RefloatAppDataState>) -> bool {
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return false;
    };
    let Some(state) = state else {
        return false;
    };
    let config = unsafe {
        core::slice::from_raw_parts(buffer.as_ptr().cast_const(), REFLOAT_DEFAULT_CONFIG.len())
    };
    // Upstream `set_cfg` gates special modes, deserializes, persists, and
    // reconfigures at `src/main.c:2360-2386`; generated
    // `conf/confparser.c:187-190` rejects bad signatures before field reads.
    // This byte-image step is intentionally only the deserialization/storage
    // part; EEPROM write and `configure(d)` remain separate parity work.
    state.store_serialized_config(config)
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatAppDataState> {
    unsafe { refloat_state_from_arg() }
}

#[cfg(any(test, not(target_arch = "arm")))]
unsafe fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatAppDataState> {
    None
}

unsafe extern "C" fn refloat_get_cfg_xml(buffer: *mut *mut u8) -> c_int {
    let xml = runtime_refloat_config_xml();
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        *buffer = xml.cast_mut();
    }
    // Upstream returns `data_refloatconfig_ + PROG_ADDR` and
    // `DATA_REFLOATCONFIG__SIZE` at `src/main.c:2388-2396`.
    REFLOAT_CONFIG_XML.len() as c_int
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_xml() -> *const u8 {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {xml}",
            address = out(reg) address,
            xml = sym REFLOAT_CONFIG_XML,
            options(nomem, nostack, preserves_flags),
        );
    }
    address as *const u8
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_xml() -> *const u8 {
    REFLOAT_CONFIG_XML.as_ptr()
}

/// Refloat package app-data state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAppDataState {
    all_data_payloads: RefloatAllDataPayloads,
    serialized_config: [u8; 276],
}

impl RefloatAppDataState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        Self {
            all_data_payloads,
            // Upstream `data_init` reads EEPROM and falls back to generated
            // defaults at `src/main.c:1160-1185`; full EEPROM parity remains a
            // later source-backed slice.
            serialized_config: REFLOAT_DEFAULT_CONFIG,
        }
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    fn serialized_config(&self) -> &[u8; 276] {
        &self.serialized_config
    }

    fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Ok(config) = <&[u8; 276]>::try_from(config) else {
            return false;
        };
        if config[..4] != REFLOAT_CONFIG_SIGNATURE_BYTES {
            return false;
        }

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        let mut config = *config;
        // Upstream clears `d->float_conf.disabled` while running at
        // `src/main.c:2369-2372`; `disabled` is serialized from
        // `src/conf/settings.xml:3890-3902` at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            config[REFLOAT_CONFIG_DISABLED_OFFSET] = 0;
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `src/main.c:2375-2377`; `meta.is_default` is serialized from
        // `src/conf/settings.xml:3903-3914` at byte 275.
        config[REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 0;
        self.serialized_config = config;
        true
    }

    /// Recover typed app-data state from VESC loader metadata.
    ///
    /// # Safety
    ///
    /// `info.arg` must either be null or contain a valid pointer to a live
    /// `RefloatAppDataState`.
    pub unsafe fn from_info_arg(info: &mut ffi::LibInfo) -> Option<&mut Self> {
        let ptr = core::ptr::NonNull::new(info.arg.cast::<Self>())?;
        Some(unsafe { ptr.as_ptr().as_mut()? })
    }

    /// Handle one app-data packet through the supplied lifecycle transport.
    pub fn handle_packet<B: AppDataBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }
        lifecycle.send_response(self.all_data_payloads, bytes)
    }

    /// Handle one app-data packet after refreshing live telemetry fields.
    pub fn handle_packet_with_telemetry<B: AppDataBindings, M: MotorTelemetryBindings>(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        bytes: &[u8],
    ) -> bool {
        if self.handle_charging_state_packet(bytes) {
            return true;
        }

        let Ok(request) = RefloatAllDataRequest::parse(bytes) else {
            return false;
        };
        let fault = telemetry.firmware_fault();
        if !fault.is_none() {
            return lifecycle.send_response_bytes(&RefloatAllDataResponse::fault(
                RefloatFirmwareFaultCode::from_compat_code(fault.compat_code()),
            ));
        }
        let mode = request.mode();
        let payloads = self
            .all_data_payloads
            .with_base_battery_voltage(BatteryVoltage::new(
                telemetry.input_voltage_filtered().voltage(),
            ));
        let payloads = if mode.includes_mode2() {
            self.runtime_all_data_payloads(payloads, telemetry, mode.includes_mode3())
        } else {
            payloads
        };
        lifecycle.send_all_data_response(payloads, request)
    }

    fn handle_charging_state_packet(&mut self, bytes: &[u8]) -> bool {
        // Refloat v1.2.1 routes COMMAND_CHARGING_STATE at `src/main.c:2267-2269`;
        // the command ID is defined in `src/charging.h:25`.
        let [package_id, command_id, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != crate::domain::REFLOAT_APP_DATA_PACKAGE_ID.get()
            || RefloatAppDataCommand::try_from_id(*command_id)
                != Ok(RefloatAppDataCommand::ChargingState)
            || payload.len() < 6
            || payload[0] != 151
        {
            return false;
        }

        let (voltage, current) = if payload[1] > 0 {
            (
                refloat_read_scaled_i16([payload[2], payload[3]], 10.0),
                refloat_read_scaled_i16([payload[4], payload[5]], 10.0),
            )
        } else {
            (0.0, 0.0)
        };
        self.all_data_payloads =
            self.all_data_payloads
                .with_mode4_charging(RefloatAllDataMode4Payload::new(
                    RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(
                        current,
                    ))),
                    RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(
                        voltage,
                    ))),
                ));
        true
    }

    fn runtime_all_data_payloads<M: MotorTelemetryBindings>(
        self,
        payloads: RefloatAllDataPayloads,
        telemetry: &MotorTelemetryApi<M>,
        include_mode3: bool,
    ) -> RefloatAllDataPayloads {
        let payloads = payloads
            .with_mode2_distance_abs(telemetry.distance_abs())
            .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                telemetry.mosfet_temperature(),
                telemetry.motor_temperature(),
            ));

        if include_mode3 {
            payloads.with_mode3_ride_totals(RefloatAllDataMode3Payload::new(
                telemetry.odometer(),
                telemetry.amp_hours_discharged(),
                telemetry.amp_hours_charged(),
                telemetry.watt_hours_discharged(),
                telemetry.watt_hours_charged(),
                telemetry.battery_level(),
            ))
        } else {
            payloads
        }
    }
}

fn refloat_read_scaled_i16(bytes: [u8; 2], scale: f32) -> f32 {
    i16::from_be_bytes(bytes) as f32 / scale
}

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

    /// Install Refloat state, stop cleanup, and app-data handler.
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
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::from_mut(state).cast();
        }
        unsafe { self.install(info, handler) }
    }

    /// Clear Refloat callbacks during package stop.
    ///
    /// Refloat `v1.2.1` clears app-data at `src/main.c:2402` and custom config
    /// at `src/main.c:2403`.
    pub fn stop(&self) -> Result<(), AppDataHandlerRegistrationError>
    where
        B: CustomConfigBindings,
    {
        let app_data_result = self.lifecycle.clear_app_data_handler();
        unsafe {
            let _ = self.lifecycle.bindings().clear_custom_configs();
        }
        app_data_result
    }

    /// Process one Refloat app-data packet and send a response when accepted.
    pub fn send_response(&self, payloads: RefloatAllDataPayloads, bytes: &[u8]) -> bool {
        let Some(response) = process_refloat_app_data(payloads, bytes) else {
            return false;
        };
        self.send_response_bytes(&response)
    }

    /// Encode and send one parsed Refloat all-data response.
    pub fn send_all_data_response(
        &self,
        payloads: RefloatAllDataPayloads,
        request: RefloatAllDataRequest,
    ) -> bool {
        self.send_response_bytes(&payloads.encode_response(request))
    }

    fn send_response_bytes(&self, response: &RefloatAllDataResponse) -> bool {
        let bytes = response.as_bytes();
        unsafe {
            self.lifecycle
                .send_app_data(bytes.as_ptr(), bytes.len() as u32)
        };
        true
    }
}

impl<B: AppDataBindings + CustomConfigBindings> RefloatAppDataLifecycle<B> {
    /// Install Refloat custom config, stop cleanup, and app-data handler.
    ///
    /// Upstream registers custom config before app-data at `src/main.c:2456-2457`.
    ///
    /// # Safety
    ///
    /// `info` must be null or point to live VESC loader metadata. The supplied
    /// handler must remain valid until firmware replaces or clears it.
    pub unsafe fn install_refloat_callbacks(
        &self,
        info: *mut ffi::LibInfo,
        handler: ffi::AppDataHandler,
    ) -> Result<(), AppDataHandlerRegistrationError> {
        let _ = register_refloat_custom_config(self.bindings());
        unsafe { self.install(info, handler) }
    }

    /// Install Refloat state plus custom config, stop cleanup, and app-data.
    ///
    /// Upstream stores `Data *` in `info->arg` at `src/main.c:2432` before
    /// registering custom config and app-data at `src/main.c:2456-2457`.
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
        if let Some(info) = unsafe { info.as_mut() } {
            info.arg = core::ptr::from_mut(state).cast();
        }
        unsafe { self.install_refloat_callbacks(info, handler) }
    }
}

unsafe extern "C" fn stop_refloat_app_data(_arg: *mut core::ffi::c_void) {
    // Upstream stop cleanup in `src/main.c:2398-2412` clears IMU/app-data/custom
    // config callbacks, terminates aux+main threads, destroys LEDs, and frees
    // `Data`. This isolated handler only clears app-data/custom config and frees
    // the narrow Rust app-data allocation if that experimental path was installed.
    #[cfg(not(test))]
    {
        let _ = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings).stop();
    }
    #[cfg(all(not(test), target_arch = "arm"))]
    if let Some(ptr) = core::ptr::NonNull::new(_arg.cast::<RefloatAppDataState>()) {
        let bindings = vescpkg_rs::RealBindings;
        let _allocation =
            unsafe { vescpkg_rs::FirmwareAllocation::from_raw_parts(ptr, 1, &bindings) };
    }
}

#[cfg(test)]
mod tests {
    use super::{RefloatAppDataLifecycle, RefloatAppDataState};
    use super::{
        allocate_refloat_startup_app_data_with, handle_refloat_app_data_packet,
        install_refloat_startup_app_data_with, process_refloat_app_data,
    };
    use crate::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature,
        RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
        RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataStatus,
        RefloatAppDataCommand, RefloatBeepReason, RefloatFocIdCurrent, RefloatMode,
        RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
        RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
        RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
        RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState,
        RefloatSetpointAdjustment, RefloatStopCondition,
    };
    use core::cell::Cell;
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FakeMotorTelemetryBindings;
    use vescpkg_rs::{AllocBindings, AppDataBindings, FirmwareAllocator, ffi};

    #[test]
    fn app_data_processes_all_data_requests_from_payload_snapshot() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        )
        .expect("all-data request should produce a response");

        assert_eq!(response.as_bytes().len(), 58);
        assert_eq!(&response.as_bytes()[..3], &[101, 10, 4]);
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::GetAllData.id(),
                ]
            ),
            None
        );
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::PrintInfo.id(),
                    4,
                ]
            ),
            None
        );
    }

    #[test]
    fn lifecycle_installs_app_data_handler_and_stop_cleanup() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(unsafe { lifecycle.install(&mut info, handler) }, Ok(()));
        assert!(info.stop_fun.is_some());
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            handler as *const () as usize
        );

        assert_eq!(lifecycle.stop(), Ok(()));
        assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
        assert_eq!(lifecycle.bindings().last_handler.get(), 0);
        // Refloat v1.2.1 stop clears app-data at `src/main.c:2402` and
        // custom config at `src/main.c:2403`.
        assert_eq!(lifecycle.bindings().custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn lifecycle_sends_refloat_app_data_responses_through_bindings() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());

        assert!(lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 4]);

        assert!(!lifecycle.send_response(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::PrintInfo.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    }

    #[test]
    fn app_data_state_handles_packets_through_lifecycle_send_boundary() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet(
            &lifecycle,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(state.all_data_payloads(), sample_all_data_payloads());
    }

    #[test]
    fn app_data_state_refreshes_mode2_distance_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_distance_abs(
            TripDistance::new(Distance::from_meters(12.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_distance_bits.get(),
            12.5_f32.to_bits()
        );
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 1);
    }

    #[test]
    fn app_data_state_refreshes_mode2_temperatures_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_temperatures(
            MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                2,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
        assert_eq!(
            lifecycle.bindings().last_sent_mode2_temperature_bytes.get(),
            [74, 97]
        );
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_mode3_ride_totals_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_ride_totals(
            OdometerMeters::from_meters(123_456),
            AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
            AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
            WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
            WattHoursCharged::new(Energy::from_watt_hours(18.5)),
            BatteryLevel::new(Ratio::from_ratio_const(0.72)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                3,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 54);
        assert_eq!(
            lifecycle.bindings().last_sent_mode3_ride_total_bytes.get(),
            [0, 1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144]
        );
        assert_eq!(telemetry.bindings().odometer_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 1);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 1);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 1);
    }

    #[test]
    fn app_data_state_sends_fault_response_before_refreshing_mode_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_firmware_fault(
            FirmwareFaultCode::from_compat_code(5),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 4);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 69]);
        assert_eq!(telemetry.bindings().firmware_fault_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn app_data_state_updates_mode4_charging_fields_from_charging_state_command() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::ChargingState.id(),
                151,
                1,
                1,
                244,
                0,
                123,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 0);

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(
            lifecycle.bindings().last_sent_mode4_charging_bytes.get(),
            [0, 123, 1, 244]
        );
    }

    #[test]
    fn app_data_state_does_not_refresh_distance_for_base_all_data() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_distance_abs(
            TripDistance::new(Distance::from_meters(12.5)),
        ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_base_battery_voltage_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_input_voltage_filtered(
                InputVoltage::new(Voltage::from_volts(84.2)),
            ));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_base_motor_voltage_bytes
                .get(),
            [3, 74]
        );
        assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
        assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
        assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
        assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
    }

    #[test]
    fn lifecycle_installs_typed_refloat_state_for_handler_retrieval() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(
            unsafe { lifecycle.install_with_state(&mut info, &mut state, handler) },
            Ok(())
        );
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            sample_all_data_payloads()
        );
        let mut empty_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        assert!(unsafe { RefloatAppDataState::from_info_arg(&mut empty_info) }.is_none());
    }

    #[test]
    fn raw_handler_boundary_rejects_null_and_sends_valid_packets() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(!unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                core::ptr::null_mut(),
                0,
            )
        });

        let mut request = [101, 10, 0];
        assert!(unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                request.as_mut_ptr(),
                request.len() as u32,
            )
        });
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 0]);
    }

    #[test]
    fn startup_app_data_install_seeds_state_and_registers_handler() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe {
            install_refloat_startup_app_data_with(&mut info, &mut state, &lifecycle, handler)
        });
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            state.all_data_payloads(),
            RefloatAllDataPayloads::source_startup()
        );
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("installed state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    #[test]
    fn startup_app_data_install_uses_firmware_allocated_state() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut backing = MaybeUninit::<RefloatAppDataState>::uninit();
        let alloc_bindings = RecordingAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&alloc_bindings);

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe {
            allocate_refloat_startup_app_data_with(&mut info, &allocator, &lifecycle, handler)
        });
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 1);
        assert_eq!(alloc_bindings.malloc_calls.get(), 1);
        assert_eq!(
            alloc_bindings.last_requested_len.get(),
            core::mem::size_of::<RefloatAppDataState>()
        );
        assert_eq!(alloc_bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert_eq!(
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }
                .expect("allocated state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    #[test]
    fn custom_config_xml_callback_returns_upstream_settings_blob() {
        let mut buffer = core::ptr::null_mut();

        let len = unsafe { super::refloat_get_cfg_xml(&mut buffer) };

        // Refloat v1.2.1 returns generated `data_refloatconfig_` at
        // `src/main.c:2388-2396`, produced from `src/conf/settings.xml` by
        // `src/Makefile:28-31`.
        assert_eq!(len, 25_723);
        assert!(!buffer.is_null());
        let bytes = unsafe { core::slice::from_raw_parts(buffer.cast_const(), len as usize) };
        assert_eq!(&bytes[..6], &[0x00, 0x05, 0x5c, 0xa1, 0x78, 0xda]);
    }

    #[test]
    fn custom_config_default_callback_returns_upstream_serialized_defaults() {
        let mut buffer = [0u8; 276];

        let len = unsafe { super::refloat_get_cfg(buffer.as_mut_ptr(), true) };

        // Refloat v1.2.1 default `get_cfg` allocates a temporary config,
        // applies generated defaults, and serializes it at `src/main.c:2339-2350`.
        // The generated format comes from `src/Makefile:28-31`;
        // generated `conf/confparser.h:11-12` fixes signature/length, and
        // generated `conf/confparser.c:8-178,363-531` writes these bytes.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("conf/default_config.dat"));
        assert_eq!(&buffer[..4], &[0x90, 0xb7, 0xa9, 0xba]);
    }

    #[test]
    fn custom_config_current_callback_reads_state_serialized_config() {
        let state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut buffer = [0u8; 276];

        let len = super::refloat_get_cfg_with_state(buffer.as_mut_ptr(), false, Some(&state));

        // Upstream current `get_cfg` serializes `d->float_conf` from shared
        // package state at `src/main.c:2347-2350`; `data_init` populates it
        // from EEPROM or generated defaults at `src/main.c:1160-1185`.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("conf/default_config.dat"));
    }

    #[test]
    fn custom_config_set_callback_stores_serialized_config_in_state() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream `set_cfg` deserializes into `d->float_conf` at
        // `src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
        // bad signature before reading the field bytes.
        incoming[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 0;
        assert_eq!(len, 276);
        assert_eq!(current, incoming);
    }

    #[test]
    fn custom_config_set_callback_resets_is_default_flag_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 1;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream clears `d->float_conf.meta.is_default` for every config
        // write at `src/main.c:2375-2377`; generated
        // `conf/confparser.c:179` serializes that flag as the final byte.
        assert_eq!(len, 276);
        assert_eq!(current[super::REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_keeps_package_enabled_while_running_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_DISABLED_OFFSET] = 1;

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream refuses to persist `disabled = true` while running at
        // `src/main.c:2369-2372`; `disabled` is serialized at
        // `src/conf/settings.xml:4064`.
        assert_eq!(len, 276);
        assert_eq!(current[super::REFLOAT_CONFIG_DISABLED_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_rejects_special_modes_like_refloat() {
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::HandTest,
        ));
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(!super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = super::refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream rejects VESC Tool config writes outside `MODE_NORMAL` at
        // `src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
        assert_eq!(len, 276);
        assert_eq!(current, *include_bytes!("conf/default_config.dat"));
    }

    struct RecordingAppDataBindings {
        handler_calls: Cell<usize>,
        last_handler: Cell<usize>,
        send_calls: Cell<usize>,
        last_sent_len: Cell<u32>,
        last_sent_prefix: Cell<[u8; 3]>,
        last_sent_base_motor_voltage_bytes: Cell<[u8; 2]>,
        last_sent_mode2_distance_bits: Cell<u32>,
        last_sent_mode2_temperature_bytes: Cell<[u8; 2]>,
        last_sent_mode3_ride_total_bytes: Cell<[u8; 13]>,
        last_sent_mode4_charging_bytes: Cell<[u8; 4]>,
        custom_config_register_calls: Cell<usize>,
        custom_config_clear_calls: Cell<usize>,
        handler_results: Cell<[bool; 2]>,
    }

    struct RecordingAllocBindings {
        malloc_calls: Cell<usize>,
        free_calls: Cell<usize>,
        next_ptr: Cell<*mut c_void>,
        last_requested_len: Cell<usize>,
    }

    impl RecordingAllocBindings {
        fn new(next_ptr: *mut c_void) -> Self {
            Self {
                malloc_calls: Cell::new(0),
                free_calls: Cell::new(0),
                next_ptr: Cell::new(next_ptr),
                last_requested_len: Cell::new(0),
            }
        }
    }

    impl AllocBindings for RecordingAllocBindings {
        unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
            self.malloc_calls.set(self.malloc_calls.get() + 1);
            self.last_requested_len.set(bytes);
            self.next_ptr.get()
        }

        unsafe fn free(&self, _ptr: *mut c_void) {
            self.free_calls.set(self.free_calls.get() + 1);
        }
    }

    impl RecordingAppDataBindings {
        fn accepting() -> Self {
            Self {
                handler_calls: Cell::new(0),
                last_handler: Cell::new(0),
                send_calls: Cell::new(0),
                last_sent_len: Cell::new(0),
                last_sent_prefix: Cell::new([0; 3]),
                last_sent_base_motor_voltage_bytes: Cell::new([0; 2]),
                last_sent_mode2_distance_bits: Cell::new(0),
                last_sent_mode2_temperature_bytes: Cell::new([0; 2]),
                last_sent_mode3_ride_total_bytes: Cell::new([0; 13]),
                last_sent_mode4_charging_bytes: Cell::new([0; 4]),
                custom_config_register_calls: Cell::new(0),
                custom_config_clear_calls: Cell::new(0),
                handler_results: Cell::new([true, true]),
            }
        }
    }

    impl AppDataBindings for RecordingAppDataBindings {
        unsafe fn set_app_data_handler(&self, handler: ffi::AppDataHandler) -> bool {
            self.handler_calls.set(self.handler_calls.get() + 1);
            self.last_handler.set(handler as *const () as usize);
            let index = self.handler_calls.get().saturating_sub(1).min(1);
            self.handler_results.get()[index]
        }

        unsafe fn clear_app_data_handler(&self) -> bool {
            self.handler_calls.set(self.handler_calls.get() + 1);
            self.last_handler.set(0);
            let index = self.handler_calls.get().saturating_sub(1).min(1);
            self.handler_results.get()[index]
        }

        fn system_time_ticks(&self) -> u32 {
            0
        }

        unsafe fn send_app_data(&self, data: *const u8, len: u32) {
            self.send_calls.set(self.send_calls.get() + 1);
            self.last_sent_len.set(len);
            if len >= 3 {
                let bytes = unsafe { core::slice::from_raw_parts(data, len as usize) };
                self.last_sent_prefix.set([bytes[0], bytes[1], bytes[2]]);
                if bytes.len() >= 24 {
                    self.last_sent_base_motor_voltage_bytes
                        .set([bytes[22], bytes[23]]);
                }
                if bytes.len() >= 38 {
                    self.last_sent_mode2_distance_bits.set(u32::from_be_bytes([
                        bytes[34], bytes[35], bytes[36], bytes[37],
                    ]));
                }
                if bytes.len() >= 40 {
                    self.last_sent_mode2_temperature_bytes
                        .set([bytes[38], bytes[39]]);
                }
                if bytes.len() >= 54 {
                    self.last_sent_mode3_ride_total_bytes.set([
                        bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46],
                        bytes[47], bytes[48], bytes[49], bytes[50], bytes[51], bytes[52],
                        bytes[53],
                    ]);
                }
                if bytes.len() >= 58 {
                    self.last_sent_mode4_charging_bytes
                        .set([bytes[54], bytes[55], bytes[56], bytes[57]]);
                }
            }
        }
    }

    impl CustomConfigBindings for RecordingAppDataBindings {
        unsafe fn register_custom_config(
            &self,
            _get_cfg: ffi::raw::CustomConfigGet,
            _set_cfg: ffi::raw::CustomConfigSet,
            _get_cfg_xml: ffi::raw::CustomConfigXml,
        ) -> bool {
            // Refloat v1.2.1 registers custom config during init at `src/main.c:2456`.
            self.custom_config_register_calls
                .set(self.custom_config_register_calls.get() + 1);
            true
        }

        unsafe fn clear_custom_configs(&self) -> bool {
            // Refloat v1.2.1 clears custom config during stop at `src/main.c:2403`.
            self.custom_config_clear_calls
                .set(self.custom_config_clear_calls.get() + 1);
            true
        }
    }

    fn sample_all_data_payloads() -> RefloatAllDataPayloads {
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal)
    }

    fn sample_all_data_payloads_with_ride_state(
        run_state: RefloatRunState,
        mode: RefloatMode,
    ) -> RefloatAllDataPayloads {
        let ride_state = RefloatRideState::new(
            run_state,
            mode,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.60)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.40)),
            FootpadSensorState::Both,
        );
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
        );

        RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
                    ImuRoll::new(AngleRadians::from_radians(-0.5)),
                    ImuPitch::new(AngleRadians::from_radians(2.3)),
                ),
                RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
                footpad,
                setpoints,
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
                RefloatAllDataMotorPayload::new(
                    BatteryVoltage::new(Voltage::from_volts(72.0)),
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
                    MotorCurrent::new(Current::from_amps(5.0)),
                    BatteryCurrent::new(Current::from_amps(-2.0)),
                    DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
                    RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
                ),
            ),
            RefloatAllDataMode2Payload::new(
                TripDistance::new(Distance::from_meters(64.0)),
                RefloatRealtimeMotorTemperatures::new(
                    MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
                    MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
                ),
                RefloatAllDataBatteryTemperature::unavailable(),
            ),
            RefloatAllDataMode3Payload::new(
                OdometerMeters::from_meters(123_456),
                AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
                AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
                WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
                WattHoursCharged::new(Energy::from_watt_hours(18.5)),
                BatteryLevel::new(Ratio::from_ratio_const(0.72)),
            ),
            RefloatAllDataMode4Payload::new(
                RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
                RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
            ),
        )
    }
}
