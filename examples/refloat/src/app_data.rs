//! Refloat app-data packet processing.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) anchors:
//! - `third_party/refloat/src/main.c:2143-2295` handles incoming app-data commands.
//! - `third_party/refloat/src/main.c:2334-2403` owns custom config get/set/XML and stop cleanup.
//! - `third_party/refloat/src/main.c:2456-2457` registers custom config and app-data handlers.
//!
//! The Rust state here is still a narrow `RefloatAppDataState`, not upstream's
//! full `Data`; upstream shares `Data *` through `ARG` for app-data, custom
//! config, BMS, threads, and stop cleanup.

#![cfg_attr(all(not(test), target_arch = "arm"), allow(dead_code))]

use crate::balance_loop::{
    RefloatBalanceLoopConfig, RefloatBalanceLoopInput, RefloatBalanceLoopState,
    refloat_balance_loop_step,
};
use crate::domain::{
    FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude,
    RefloatAllDataBasePayload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
    RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataRequest,
    RefloatAllDataResponse, RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
    RefloatDarkRideState, RefloatFirmwareFaultCode, RefloatFocIdCurrent, RefloatMode,
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
    RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};
use crate::motor_control::RefloatMotorControl;
use crate::runtime::RefloatRuntimeThreads;
use crate::state_transition::{
    RefloatStateTransitionInput, RefloatStopEvent, refloat_first_stop_event,
    refloat_state_transition,
};
use core::ffi::c_int;
use vescpkg_rs::prelude::{
    AngleDegrees, AngleRadians, BatteryCurrent, BatteryVoltage, Current, MotorCurrent, SampleRate,
    SystemTimestamp, TimestampTicks, Voltage,
};
use vescpkg_rs::{
    AppDataBindings, AppDataHandlerRegistrationError, CustomConfigBindings, ImuApi, ImuBindings,
    ImuReadCallbackBindings, LoopbackLifecycle, MotorControlApi, MotorControlBindings,
    MotorTelemetryApi, MotorTelemetryBindings, ffi,
};

use self::protocol::{encode_refloat_realtime_data_response, process_refloat_app_data};
use crate::config::*;

mod protocol;

fn refloat_ticks_elapsed(now: u32, then: u32, seconds: u32) -> bool {
    now.wrapping_sub(then) >= seconds.saturating_mul(10_000)
}

fn refloat_ticks_elapsed_ms(now: u32, then: u32, milliseconds: u32) -> bool {
    now.wrapping_sub(then) > milliseconds.saturating_mul(10)
}

fn refloat_ticks_elapsed_f32(now: u32, then: u32, seconds: f32) -> bool {
    now.wrapping_sub(then) > (seconds * 10_000.0) as u32
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn handle_refloat_app_data_packet<
    B: AppDataBindings,
    M: MotorTelemetryBindings,
    I: ImuBindings,
>(
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
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
    state.handle_packet_with_runtime(lifecycle, telemetry, imu, bytes)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn loaded_image_base() -> u32 {
    let loaded_handler: usize;
    unsafe {
        core::arch::asm!(
            "adr {loaded_handler}, {handler}",
            loaded_handler = out(reg) loaded_handler,
            handler = sym refloat_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
    }
    let loaded_handler = loaded_handler & !1;
    let image_handler = refloat_handle_app_data as *const () as usize & !1;
    (loaded_handler - image_handler) as u32
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
    // C map: closest visible state compatibility edge is `state_compat` at
    // Refloat v1.2.1 `third_party/refloat/src/state.c:50`; loader ARG storage happens at
    // `third_party/refloat/src/main.c:2432`.
    let state = vescpkg_rs::RealBindings
        .app_data_arg(loaded_image_base())?
        .cast::<RefloatAppDataState>();
    unsafe { state.as_ptr().as_mut() }
}

/// Device entrypoint invoked by firmware app-data delivery.
///
/// C map: upstream `on_command_received` starts at `third_party/refloat/src/main.c:2143` and is
/// registered in `third_party/refloat/src/main.c:2457`.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn refloat_handle_app_data(data: *mut u8, len: u32) {
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
    let imu = ImuApi::new(vescpkg_rs::RealImuBindings);
    let _ =
        unsafe { handle_refloat_app_data_packet(state, &lifecycle, &telemetry, &imu, data, len) };
}

/// Install source-startup Refloat state without registering callbacks.
///
/// Upstream allocates `Data`, runs `data_init`, and stores `stop`/`Data *` in
/// loader metadata at `third_party/refloat/src/main.c:2419-2432`; callback/LispBM registration
/// follows at `third_party/refloat/src/main.c:2455-2459`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `state` must
/// remain valid until firmware stops the package.
#[cfg(test)]
pub(crate) unsafe fn install_refloat_startup_state_with<B: AppDataBindings>(
    info: *mut ffi::LibInfo,
    state: &mut RefloatAppDataState,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    *state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());
    unsafe { lifecycle.install_refloat_state(info, state, handler) }
}

/// Install source-startup Refloat state and callback registrations.
///
/// Upstream stores loader metadata at `third_party/refloat/src/main.c:2431-2432` before registering
/// custom config/app-data callbacks at `third_party/refloat/src/main.c:2456-2457`.
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
    if !unsafe { install_refloat_startup_state_with(info, state, lifecycle, handler) } {
        return false;
    }
    unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_ok()
}

#[cfg(any(test, target_arch = "arm"))]
unsafe fn clear_refloat_app_data_loader_info(info: *mut ffi::LibInfo) {
    if let Some(info) = unsafe { info.as_mut() } {
        info.arg = core::ptr::null_mut();
        info.stop_fun = None;
    }
}

/// Allocate and install source-startup Refloat state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `third_party/refloat/src/main.c:2419`, runs
/// `data_init` at `third_party/refloat/src/main.c:2424`, and stores the same pointer in
/// `info->arg` at `third_party/refloat/src/main.c:2432`. This Rust path still allocates a narrow
/// `RefloatAppDataState`, but keeps the same loader metadata order before the
/// registration tail at `third_party/refloat/src/main.c:2455-2459`.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `handler` must
/// remain valid until firmware stops the package.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) unsafe fn allocate_refloat_startup_state_with<
    A: vescpkg_rs::AllocBindings,
    B: AppDataBindings,
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
    unsafe { RefloatAppDataState::write_source_startup_to(state) };
    let state = unsafe { &mut *state };

    if !unsafe { lifecycle.install_refloat_state(info, state, handler) } {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    }

    let _ = allocation.into_raw();
    true
}

/// Allocate source-startup Refloat state and register app-data callbacks.
///
/// Upstream performs state setup at `third_party/refloat/src/main.c:2419-2432`, starts runtime
/// threads at `third_party/refloat/src/main.c:2439-2449`, then registers custom config/app-data
/// callbacks at `third_party/refloat/src/main.c:2456-2457` after IMU setup. This compatibility
/// helper only keeps state-before-callback order for tests.
///
/// # Safety
///
/// `info` must be null or point to live VESC loader metadata. `handler` must
/// remain valid until firmware clears/replaces the handler and stops the package.
#[cfg(test)]
pub(crate) unsafe fn allocate_refloat_startup_app_data_with<
    A: vescpkg_rs::AllocBindings,
    B: AppDataBindings + CustomConfigBindings,
>(
    info: *mut ffi::LibInfo,
    allocator: &vescpkg_rs::FirmwareAllocator<'_, A>,
    lifecycle: &RefloatAppDataLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    if !unsafe { allocate_refloat_startup_state_with(info, allocator, lifecycle, handler) } {
        return false;
    }

    if unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_err() {
        unsafe { clear_refloat_app_data_loader_info(info) };
        return false;
    }

    true
}

/// Allocate and install Refloat startup state using firmware memory.
///
/// This matches the loader metadata step from upstream `third_party/refloat/src/main.c:2419-2432`;
/// callback/LispBM registration is a separate step at `third_party/refloat/src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data_state(info: *mut ffi::LibInfo) -> bool {
    let alloc_bindings = vescpkg_rs::RealBindings;
    let allocator = vescpkg_rs::FirmwareAllocator::new(&alloc_bindings);
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let handler = runtime_refloat_app_data_handler();
    unsafe { allocate_refloat_startup_state_with(info, &allocator, &lifecycle, handler) }
}

/// Register Refloat custom config and app-data callbacks.
///
/// Upstream registers these callbacks at `third_party/refloat/src/main.c:2456-2457`, after runtime
/// thread startup at `third_party/refloat/src/main.c:2439-2449` and IMU setup at
/// `third_party/refloat/src/main.c:2455`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_app_data_callbacks(info: *mut ffi::LibInfo) -> bool {
    let lifecycle = RefloatAppDataLifecycle::new(vescpkg_rs::RealBindings);
    let handler = runtime_refloat_app_data_handler();
    unsafe { lifecycle.install_refloat_callbacks(info, handler) }.is_ok()
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe extern "C" fn refloat_imu_read_callback(
    acc: *mut f32,
    gyro: *mut f32,
    _mag: *mut f32,
    dt: f32,
) {
    let Some(accel) = refloat_imu_vector(acc) else {
        return;
    };
    let Some(gyro) = refloat_imu_vector(gyro) else {
        return;
    };
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    refloat_imu_callback_with_state(state, accel, gyro, dt);
}

#[cfg(test)]
unsafe extern "C" fn refloat_imu_read_callback(
    _acc: *mut f32,
    _gyro: *mut f32,
    _mag: *mut f32,
    _dt: f32,
) {
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn refloat_imu_callback_with_state(
    state: &mut RefloatAppDataState,
    accel: [f32; 3],
    gyro: [f32; 3],
    dt: f32,
) {
    // C `imu_ref_callback` ignores mag and feeds gyro/accel/dt into
    // `balance_filter_update` at `third_party/refloat/src/main.c:760-765`.
    state.balance_filter.update(gyro, accel, dt);
}

#[cfg(all(not(test), target_arch = "arm"))]
fn refloat_imu_vector(values: *mut f32) -> Option<[f32; 3]> {
    if values.is_null() {
        return None;
    }
    let values = unsafe { core::slice::from_raw_parts(values as *const f32, 3) };
    Some([*values.get(0)?, *values.get(1)?, *values.get(2)?])
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn register_refloat_imu_callback_with<B: ImuReadCallbackBindings>(bindings: &B) -> bool {
    unsafe {
        // C registers `imu_ref_callback` between thread startup and app-data
        // registration at `third_party/refloat/src/main.c:2455-2457`.
        bindings.set_imu_read_callback(refloat_imu_read_callback);
    }
    true
}

/// Register Refloat's IMU read callback.
///
/// Upstream registers `imu_ref_callback` at `third_party/refloat/src/main.c:2455`; that callback
/// maintains the balance filter used by `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_imu_callback(_info: *mut ffi::LibInfo) -> bool {
    register_refloat_imu_callback_with(&vescpkg_rs::RealBindings)
}

/// Allocate startup state and register Refloat app-data callbacks.
///
/// Kept as the old combined entrypoint for callers that do not need the
/// upstream split between `third_party/refloat/src/main.c:2431-2432` and `third_party/refloat/src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_app_data(info: *mut ffi::LibInfo) -> bool {
    install_refloat_app_data_state(info)
        && register_refloat_imu_callback(info)
        && register_refloat_app_data_callbacks(info)
}

/// Register Refloat custom-config callbacks with VESC Tool.
///
/// Upstream registers `get_cfg`, `set_cfg`, and `get_cfg_xml` at
/// `third_party/refloat/src/main.c:2456`; those callbacks are implemented at `third_party/refloat/src/main.c:2334-2396`.
/// The Rust port does not yet generate or serialize upstream `RefloatConfig`, so
/// these callbacks report no config payload instead of pretending to be the full
/// confparser path.
pub fn register_refloat_custom_config<B: CustomConfigBindings>(bindings: &B) -> bool {
    unsafe {
        bindings.register_custom_config(refloat_get_cfg, refloat_set_cfg, refloat_get_cfg_xml)
    }
}

unsafe extern "C" fn refloat_get_cfg(buffer: *mut u8, is_default: bool) -> c_int {
    // C map: Refloat v1.2.1 `get_cfg` starts at `third_party/refloat/src/main.c:2335`.
    let state = unsafe { runtime_refloat_config_state() };
    refloat_get_cfg_with_state(buffer, is_default, state)
}

fn refloat_get_cfg_with_state(
    buffer: *mut u8,
    is_default: bool,
    state: Option<&RefloatAppDataState>,
) -> c_int {
    if !is_default {
        // Upstream serializes `d->float_conf` at `third_party/refloat/src/main.c:2347-2350`;
        // `data_init` first populates it from EEPROM or generated defaults at
        // `third_party/refloat/src/main.c:1160-1185`. The Rust state stores the serialized image
        // until the typed `RefloatConfig` parser/deserializer is ported.
        let Some(state) = state else {
            return 0;
        };
        return copy_refloat_config(buffer, state.serialized_config());
    }

    // Upstream default path is `third_party/refloat/src/main.c:2339-2350`: allocate config, call
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
    // C map: Refloat v1.2.1 `set_cfg` starts at `third_party/refloat/src/main.c:2360`.
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
    // reconfigures at `third_party/refloat/src/main.c:2360-2386`; generated
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
    // C map: Refloat v1.2.1 `get_cfg_xml` starts at `third_party/refloat/src/main.c:2389`.
    let xml = runtime_refloat_config_xml();
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        *buffer = xml.cast_mut();
    }
    // Upstream returns `data_refloatconfig_ + PROG_ADDR` and
    // `DATA_REFLOATCONFIG__SIZE` at `third_party/refloat/src/main.c:2388-2396`.
    REFLOAT_CONFIG_XML.len() as c_int
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_xml() -> *const u8 {
    (loaded_image_base() as usize + REFLOAT_CONFIG_XML.as_ptr() as usize) as *const u8
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_xml() -> *const u8 {
    REFLOAT_CONFIG_XML.as_ptr()
}

/// Refloat-owned balance filter state.
///
/// C map: `BalanceFilterData` is initialized from firmware quaternions at
/// `third_party/refloat/src/balance_filter.c:53-61`, configured at `third_party/refloat/src/balance_filter.c:64-70`,
/// updated from `imu_ref_callback` at `third_party/refloat/src/main.c:760-765`, and read by
/// `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatBalanceFilter {
    q0: f32,
    q1: f32,
    q2: f32,
    q3: f32,
    acc_mag: f32,
    kp_pitch: f32,
    kp_roll: f32,
    kp_yaw: f32,
}

impl RefloatBalanceFilter {
    const fn source_startup() -> Self {
        Self {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
            acc_mag: 1.0,
            kp_pitch: 2.0,
            kp_roll: 1.4,
            kp_yaw: 1.7,
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn from_quaternions([q0, q1, q2, q3]: [f32; 4]) -> Self {
        Self {
            q0,
            q1,
            q2,
            q3,
            ..Self::source_startup()
        }
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    fn from_firmware_quaternions() -> Self {
        Self::from_quaternions(vescpkg_rs::RealImuBindings.quaternions())
    }

    fn configure(&mut self, mahony_kp: f32, mahony_kp_roll: f32) {
        // Refloat copies `mahony_kp`/`mahony_kp_roll` into the filter and
        // averages yaw KP at `third_party/refloat/src/balance_filter.c:64-70`.
        self.kp_pitch = mahony_kp;
        self.kp_roll = mahony_kp_roll;
        self.kp_yaw = (mahony_kp + mahony_kp_roll) / 2.0;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn update(&mut self, gyro: [f32; 3], accel: [f32; 3], dt: f32) {
        // Refloat's callback feeds gyro first, accel second at
        // `third_party/refloat/src/main.c:760-765`; the Mahony update itself is
        // `third_party/refloat/src/balance_filter.c:73-134`.
        let [mut gx, mut gy, mut gz] = gyro;
        let [mut ax, mut ay, mut az] = accel;
        let accel_norm = libm::sqrtf(ax * ax + ay * ay + az * az);

        if accel_norm > 0.01 {
            let accel_confidence = self.accel_confidence(accel_norm);
            let two_kp_pitch = 2.0 * self.kp_pitch * accel_confidence;
            let two_kp_roll = 2.0 * self.kp_roll * accel_confidence;
            let two_kp_yaw = 2.0 * self.kp_yaw * accel_confidence;
            let recip_norm = Self::inv_sqrt(ax * ax + ay * ay + az * az);
            ax *= recip_norm;
            ay *= recip_norm;
            az *= recip_norm;

            let halfvx = self.q1 * self.q3 - self.q0 * self.q2;
            let halfvy = self.q0 * self.q1 + self.q2 * self.q3;
            let halfvz = self.q0 * self.q0 - 0.5 + self.q3 * self.q3;
            let halfex = ay * halfvz - az * halfvy;
            let halfey = az * halfvx - ax * halfvz;
            let halfez = ax * halfvy - ay * halfvx;

            gx += two_kp_roll * halfex;
            gy += two_kp_pitch * halfey;
            gz += two_kp_yaw * halfez;
        }

        gx *= 0.5 * dt;
        gy *= 0.5 * dt;
        gz *= 0.5 * dt;
        let qa = self.q0;
        let qb = self.q1;
        let qc = self.q2;
        self.q0 += -qb * gx - qc * gy - self.q3 * gz;
        self.q1 += qa * gx + qc * gz - self.q3 * gy;
        self.q2 += qa * gy - qb * gz + self.q3 * gx;
        self.q3 += qa * gz + qb * gy - qc * gx;

        let recip_norm = Self::inv_sqrt(
            self.q0 * self.q0 + self.q1 * self.q1 + self.q2 * self.q2 + self.q3 * self.q3,
        );
        self.q0 *= recip_norm;
        self.q1 *= recip_norm;
        self.q2 *= recip_norm;
        self.q3 *= recip_norm;
    }

    fn balance_pitch(&self) -> RefloatRealtimeBalancePitch {
        RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(self.pitch_radians()))
    }

    fn pitch_radians(&self) -> f32 {
        // Refloat computes pitch as `asin(-2 * (q1*q3 - q0*q2))`, clamped to
        // +/- pi/2, at `third_party/refloat/src/balance_filter.c:145-154`.
        let sin = -2.0 * (self.q1 * self.q3 - self.q0 * self.q2);
        if sin < -1.0 {
            -core::f32::consts::FRAC_PI_2
        } else if sin > 1.0 {
            core::f32::consts::FRAC_PI_2
        } else {
            libm::asinf(sin)
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn accel_confidence(&mut self, new_acc_mag: f32) -> f32 {
        // Refloat filters accelerometer magnitude and clamps confidence at
        // zero in `third_party/refloat/src/balance_filter.c:42-50`.
        self.acc_mag = self.acc_mag * 0.9 + new_acc_mag * 0.1;
        let confidence = 1.0 - 0.02 * libm::sqrtf((self.acc_mag - 1.0).abs());
        if confidence > 0.0 { confidence } else { 0.0 }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn inv_sqrt(value: f32) -> f32 {
        // Refloat uses `1.0 / sqrtf(x)` at `third_party/refloat/src/balance_filter.c:38-40`.
        1.0 / libm::sqrtf(value)
    }
}

/// Refloat package app-data state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefloatAppDataState {
    all_data_payloads: RefloatAllDataPayloads,
    serialized_config: [u8; 276],
    handtest_config_backup: Option<[u8; 276]>,
    runtime_threads: RefloatRuntimeThreads,
    motor_control: RefloatMotorControl,
    balance_filter: RefloatBalanceFilter,
    traction_control: bool,
    pid_integral_current: f32,
    pid_kp_brake_scale: f32,
    pid_kp2_brake_scale: f32,
    pid_kp_accel_scale: f32,
    pid_kp2_accel_scale: f32,
    softstart_pid_limit: f32,
    reverse_total_erpm: f32,
    motor_last_erpm: f32,
    motor_acceleration: f32,
    motor_accel_history: [f32; 40],
    motor_accel_idx: usize,
    remote_input: f32,
    rc_current: f32,
    rc_steps: u16,
    rc_counter: u16,
    rc_current_target_deciamps: i16,
    engage_ticks: u32,
    disengage_ticks: u32,
    fault_switch_ticks: u32,
    fault_switch_half_ticks: u32,
    reverse_ticks: u32,
    fault_angle_pitch_ticks: u32,
    fault_angle_roll_ticks: u32,
    motor_current_max: MotorCurrent,
    motor_current_min: MotorCurrent,
}

impl RefloatAppDataState {
    /// Build app-data state from the current all-data payload snapshot.
    pub fn new(all_data_payloads: RefloatAllDataPayloads) -> Self {
        Self {
            all_data_payloads,
            // Upstream `data_init` reads EEPROM and falls back to generated
            // defaults at `third_party/refloat/src/main.c:1160-1185`; full EEPROM parity remains a
            // later source-backed slice.
            serialized_config: REFLOAT_DEFAULT_CONFIG,
            handtest_config_backup: None,
            // Upstream stores these in `Data` after spawning at
            // `third_party/refloat/src/main.c:2439-2445`; this Rust state only tracks the handles
            // until the full `Data` layout is ported.
            runtime_threads: RefloatRuntimeThreads::empty(),
            motor_control: RefloatMotorControl::new(),
            balance_filter: RefloatBalanceFilter::source_startup(),
            traction_control: false,
            pid_integral_current: 0.0,
            pid_kp_brake_scale: 1.0,
            pid_kp2_brake_scale: 1.0,
            pid_kp_accel_scale: 1.0,
            pid_kp2_accel_scale: 1.0,
            softstart_pid_limit: 100.0,
            reverse_total_erpm: 0.0,
            motor_last_erpm: 0.0,
            motor_acceleration: 0.0,
            motor_accel_history: [0.0; 40],
            motor_accel_idx: 0,
            remote_input: 0.0,
            rc_current: 0.0,
            rc_steps: 0,
            rc_counter: 0,
            rc_current_target_deciamps: 0,
            engage_ticks: 0,
            disengage_ticks: 0,
            fault_switch_ticks: 0,
            fault_switch_half_ticks: 0,
            reverse_ticks: 0,
            fault_angle_pitch_ticks: 0,
            fault_angle_roll_ticks: 0,
            motor_current_max: MotorCurrent::new(Current::from_amps(100.0)),
            motor_current_min: MotorCurrent::new(Current::from_amps(100.0)),
        }
    }

    /// Initialize firmware-allocated startup state without a full stack copy.
    ///
    /// C map: upstream allocates `Data` at `third_party/refloat/src/main.c:2419`, zeroes it at
    /// `third_party/refloat/src/main.c:2421`, and initializes fields through that heap pointer in
    /// `data_init` at `third_party/refloat/src/main.c:1160-1205` before storing `info->arg` at
    /// `third_party/refloat/src/main.c:2432`. This mirrors that pointer-first shape for the Rust
    /// state allocation path.
    ///
    /// # Safety
    ///
    /// `state` must point to writable, properly aligned, uninitialized storage
    /// for one `RefloatAppDataState`.
    #[cfg(any(test, target_arch = "arm"))]
    unsafe fn write_source_startup_to(state: *mut Self) {
        unsafe {
            core::ptr::addr_of_mut!((*state).all_data_payloads)
                .write(RefloatAllDataPayloads::source_startup());
            // C source defaults: `confparser_set_defaults_refloatconfig` and
            // `confparser_serialize_refloatconfig` feed startup config at
            // `third_party/refloat/src/main.c:1160-1185`.
            core::ptr::addr_of_mut!((*state).serialized_config).write(REFLOAT_DEFAULT_CONFIG);
            core::ptr::addr_of_mut!((*state).runtime_threads).write(RefloatRuntimeThreads::empty());
            core::ptr::addr_of_mut!((*state).motor_control).write(RefloatMotorControl::new());
            #[cfg(all(not(test), target_arch = "arm"))]
            core::ptr::addr_of_mut!((*state).balance_filter)
                .write(RefloatBalanceFilter::from_firmware_quaternions());
            #[cfg(any(test, not(target_arch = "arm")))]
            core::ptr::addr_of_mut!((*state).balance_filter)
                .write(RefloatBalanceFilter::source_startup());
            core::ptr::addr_of_mut!((*state).traction_control).write(false);
            core::ptr::addr_of_mut!((*state).pid_integral_current).write(0.0);
            core::ptr::addr_of_mut!((*state).pid_kp_brake_scale).write(1.0);
            core::ptr::addr_of_mut!((*state).pid_kp2_brake_scale).write(1.0);
            core::ptr::addr_of_mut!((*state).pid_kp_accel_scale).write(1.0);
            core::ptr::addr_of_mut!((*state).pid_kp2_accel_scale).write(1.0);
            core::ptr::addr_of_mut!((*state).softstart_pid_limit).write(100.0);
            core::ptr::addr_of_mut!((*state).reverse_total_erpm).write(0.0);
            core::ptr::addr_of_mut!((*state).motor_last_erpm).write(0.0);
            core::ptr::addr_of_mut!((*state).motor_acceleration).write(0.0);
            core::ptr::addr_of_mut!((*state).motor_accel_history).write([0.0; 40]);
            core::ptr::addr_of_mut!((*state).motor_accel_idx).write(0);
            core::ptr::addr_of_mut!((*state).remote_input).write(0.0);
            core::ptr::addr_of_mut!((*state).rc_current).write(0.0);
            core::ptr::addr_of_mut!((*state).rc_steps).write(0);
            core::ptr::addr_of_mut!((*state).rc_counter).write(0);
            core::ptr::addr_of_mut!((*state).rc_current_target_deciamps).write(0);
            core::ptr::addr_of_mut!((*state).engage_ticks).write(0);
            core::ptr::addr_of_mut!((*state).disengage_ticks).write(0);
            core::ptr::addr_of_mut!((*state).fault_switch_ticks).write(0);
            core::ptr::addr_of_mut!((*state).fault_switch_half_ticks).write(0);
            core::ptr::addr_of_mut!((*state).reverse_ticks).write(0);
            core::ptr::addr_of_mut!((*state).fault_angle_pitch_ticks).write(0);
            core::ptr::addr_of_mut!((*state).fault_angle_roll_ticks).write(0);
            core::ptr::addr_of_mut!((*state).motor_current_max)
                .write(MotorCurrent::new(Current::from_amps(100.0)));
            core::ptr::addr_of_mut!((*state).motor_current_min)
                .write(MotorCurrent::new(Current::from_amps(100.0)));
        }
    }

    /// Return the current all-data payload snapshot.
    pub const fn all_data_payloads(self) -> RefloatAllDataPayloads {
        self.all_data_payloads
    }

    /// Return the runtime thread handles currently owned by this package state.
    pub const fn runtime_threads(self) -> RefloatRuntimeThreads {
        self.runtime_threads
    }

    /// Request a motor current for the next motor-control apply step.
    pub fn request_motor_current(&mut self, current: MotorCurrent) {
        self.motor_control.request_current(current);
    }

    #[cfg(test)]
    fn set_remote_input_for_test(&mut self, remote_input: f32) {
        self.remote_input = remote_input;
    }

    /// Apply and clear a pending motor-current request.
    pub fn apply_requested_motor_current<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
    ) -> bool {
        self.motor_control.apply_requested_current(motor)
    }

    /// Apply motor control for the current run state.
    pub fn apply_motor_control<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
        run_state: RefloatRunState,
        system_time_ticks: u32,
    ) -> bool {
        let base = self.all_data_payloads.base();
        // Upstream `motor_control_configure` copies brake and parking config at
        // `third_party/refloat/src/motor_control.c:36-40`; this Rust state keeps
        // the serialized config as source of truth until full `Data` parity.
        self.motor_control.apply(
            motor,
            run_state,
            base.motor()
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute()
                .abs(),
            system_time_ticks,
            self.config_byte(REFLOAT_CONFIG_PARKING_BRAKE_MODE_OFFSET),
            MotorCurrent::new(Current::from_amps(
                self.config_scaled_i16(REFLOAT_CONFIG_BRAKE_CURRENT_OFFSET, 100.0),
            )),
        )
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn set_runtime_threads(&mut self, runtime_threads: RefloatRuntimeThreads) {
        self.runtime_threads = runtime_threads;
    }

    fn serialized_config(&self) -> &[u8; 276] {
        &self.serialized_config
    }

    fn config_byte(&self, offset: usize) -> u8 {
        let Some(byte) = self.serialized_config.get(offset) else {
            return 0;
        };
        *byte
    }

    fn config_be_u16(&self, offset: usize) -> u16 {
        u16::from_be_bytes([self.config_byte(offset), self.config_byte(offset + 1)])
    }

    fn config_scaled_i16(&self, offset: usize, scale: f32) -> f32 {
        refloat_read_scaled_i16(self.config_be_u16(offset).to_be_bytes(), scale)
    }

    fn config_scaled_field(&self, field: RefloatScaledConfigField) -> f32 {
        self.config_scaled_i16(field.offset.get(), field.scale.get())
    }

    fn set_config_byte(config: &mut [u8; 276], offset: usize, value: u8) -> bool {
        let Some(byte) = config.get_mut(offset) else {
            return false;
        };
        *byte = value;
        true
    }

    // HANDTEST writes mirror `third_party/refloat/src/main.c:1431-1446`; keep the serialized
    // u16 store checked so corrupt offsets cannot panic on target.
    fn set_config_be_u16(config: &mut [u8; 276], offset: usize, value: u16) -> bool {
        let Some(bytes) = offset
            .checked_add(2)
            .and_then(|end| config.get_mut(offset..end))
            .and_then(|bytes| <&mut [u8; 2]>::try_from(bytes).ok())
        else {
            return false;
        };
        *bytes = value.to_be_bytes();
        true
    }

    fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Ok(config) = <&[u8; 276]>::try_from(config) else {
            return false;
        };
        if !config.starts_with(&REFLOAT_CONFIG_SIGNATURE_BYTES) {
            return false;
        }

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `third_party/refloat/src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        let mut config = *config;
        // Upstream clears `d->float_conf.disabled` while running at
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is
        // serialized from `third_party/refloat/src/conf/settings.xml:3890-3902`
        // at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            Self::set_config_byte(&mut config, REFLOAT_CONFIG_DISABLED_OFFSET, 0);
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `third_party/refloat/src/main.c:2375-2377`; `meta.is_default`
        // is serialized from `third_party/refloat/src/conf/settings.xml:3903-3914`
        // at byte 275.
        Self::set_config_byte(&mut config, REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET, 0);
        self.serialized_config = config;
        // After a successful write, C calls `configure(d)` at
        // `third_party/refloat/src/main.c:2380-2382`, which refreshes the balance filter KP at
        // `third_party/refloat/src/main.c:158-160`.
        self.refresh_balance_filter_config();
        true
    }

    fn refresh_balance_filter_config(&mut self) {
        let mahony_kp = self.config_scaled_i16(REFLOAT_CONFIG_MAHONY_KP_OFFSET, 10000.0);
        let mahony_kp_roll = self.config_scaled_i16(REFLOAT_CONFIG_MAHONY_KP_ROLL_OFFSET, 10000.0);
        self.balance_filter.configure(mahony_kp, mahony_kp_roll);
    }

    fn refresh_config_runtime_state(&mut self) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let disabled = self.config_byte(REFLOAT_CONFIG_DISABLED_OFFSET) != 0;
        let run_state = match (ride_state.run_state(), disabled) {
            // Refloat applies `float_conf.disabled` from `configure(d)` at
            // `third_party/refloat/src/main.c:184-190`; `state_set_disabled`
            // keeps RUNNING alive and toggles DISABLED/STARTUP at
            // `third_party/refloat/src/state.c:41-47`.
            (RefloatRunState::Running, true) => RefloatRunState::Running,
            (RefloatRunState::Disabled, false) => RefloatRunState::Startup,
            (_, true) => RefloatRunState::Disabled,
            (run_state, false) => run_state,
        };
        if run_state == ride_state.run_state() {
            return;
        }

        let ride_state = RefloatRideState::new(
            run_state,
            ride_state.mode(),
            ride_state.setpoint_adjustment(),
            ride_state.stop_condition(),
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(ride_state.wheelslip())
        .with_darkride(ride_state.darkride());
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        let hertz = self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET);
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/refloat/src/main.c:190-191`, then `refloat_thd`
        // sleeps that value at `third_party/refloat/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        1_000_000 / u32::from(hertz.max(1))
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
        lifecycle.send_response(&self.all_data_payloads, bytes)
    }

    /// Handle one app-data packet in the firmware callback context.
    ///
    /// Upstream `on_command_received` dispatches commands at
    /// `third_party/refloat/src/main.c:2143-2225`; the main
    /// `refloat_thd` owns `time_update`, `imu_update`, `motor_data_update`, and
    /// control-loop transitions at `third_party/refloat/src/main.c:772-1080`.
    pub fn handle_packet_with_runtime<
        B: AppDataBindings,
        M: MotorTelemetryBindings,
        I: ImuBindings,
    >(
        &mut self,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        _imu: &ImuApi<I>,
        bytes: &[u8],
    ) -> bool {
        #[cfg(all(not(test), not(target_arch = "arm")))]
        self.refresh_runtime_state(telemetry, _imu, lifecycle.bindings().system_time_ticks());

        self.handle_packet_with_telemetry(lifecycle, telemetry, bytes)
    }

    /// Refresh the source-backed runtime slices that Refloat updates near the
    /// top of `refloat_thd`.
    ///
    /// C map: Refloat v1.2.1 `imu_ref_callback` starts at `third_party/refloat/src/main.c:760`.
    ///
    /// Upstream applies `configure(d)` before runtime work at
    /// `third_party/refloat/src/main.c:184-191`, updates IMU at `third_party/refloat/src/main.c:775`, motor data at
    /// `third_party/refloat/src/main.c:796`, and performs the `STATE_STARTUP` -> `STATE_READY`
    /// gate at `third_party/refloat/src/main.c:833-838`.
    pub(crate) fn refresh_runtime_state<M: MotorTelemetryBindings, I: ImuBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        system_time_ticks: u32,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    /// Refresh the runtime slices in the target main-loop order.
    ///
    /// C map: Refloat v1.2.1 `refloat_thd` updates motor data at
    /// `third_party/refloat/src/main.c:796`, footpad ADC state at
    /// `third_party/refloat/src/main.c:802`, then uses that state in the
    /// later control/fault path.
    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    pub(crate) fn refresh_main_loop_runtime_state<M: MotorTelemetryBindings, I: ImuBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        footpad_adc1: f32,
        footpad_adc2: f32,
        system_time_ticks: u32,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.refresh_footpad_runtime_state(footpad_adc1, footpad_adc2);
        self.refresh_imu_runtime_state(imu, system_time_ticks);
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
        if self.handle_handtest_packet(bytes) {
            return true;
        }
        if let [package_id, command_id, direction, current, time, sum, ..] = bytes
            && *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get()
            && *command_id == RefloatAppDataCommand::RcMove.id()
        {
            if matches!(
                self.all_data_payloads
                    .base()
                    .status()
                    .ride_state()
                    .run_state(),
                RefloatRunState::Ready
            ) {
                self.rc_counter = 0;
                self.rc_current_target_deciamps = if *sum != time.wrapping_add(*current) {
                    0
                } else if *direction == 0 {
                    -i16::from(*current)
                } else {
                    i16::from(*current)
                };
                if self.rc_current_target_deciamps == 0 {
                    self.rc_steps = 1;
                    self.rc_current = 0.0;
                } else {
                    self.rc_steps = u16::from(*time) * 100;
                    if self.rc_current_target_deciamps > 80 {
                        self.rc_current_target_deciamps = 20;
                    }
                }
            }
            return true;
        }

        if matches!(
            bytes,
            [
                package_id,
                command_id,
                ..
            ] if *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get()
                && matches!(
                    RefloatAppDataCommand::try_from_id(*command_id),
                    Ok(RefloatAppDataCommand::Info | RefloatAppDataCommand::RealtimeDataIds)
                )
        ) {
            return lifecycle.send_response(&self.all_data_payloads, bytes);
        }

        if matches!(
            bytes,
            [package_id, command_id, ..]
                if *package_id == REFLOAT_APP_DATA_PACKAGE_ID.get()
                    && matches!(
                        RefloatAppDataCommand::try_from_id(*command_id),
                        Ok(RefloatAppDataCommand::RealtimeData)
                    )
        ) {
            let payloads = self
                .all_data_payloads
                .with_base_battery_voltage(BatteryVoltage::new(
                    telemetry.input_voltage_filtered().voltage(),
                ))
                .with_mode2_temperatures(RefloatRealtimeMotorTemperatures::new(
                    telemetry.mosfet_temperature(),
                    telemetry.motor_temperature(),
                ));
            // Refloat's main loop updates `d->time.now` before app-data reads it
            // in `cmd_realtime_data` at `third_party/refloat/src/main.c:1931`.
            let system_timestamp = SystemTimestamp::new(TimestampTicks::from_ticks(
                lifecycle.bindings().system_time_ticks(),
            ));
            let response = encode_refloat_realtime_data_response(&payloads, system_timestamp);
            return lifecycle.send_response_bytes(response.as_bytes());
        }

        let Ok(request) = RefloatAllDataRequest::parse(bytes) else {
            return false;
        };
        let fault = telemetry.firmware_fault();
        if !fault.is_none() {
            let Some(fault_code) = fault.compat_code() else {
                return false;
            };
            let response = RefloatAllDataResponse::fault(
                RefloatFirmwareFaultCode::from_compat_code(fault_code),
            );
            return lifecycle.send_response_bytes(response.as_bytes());
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
        lifecycle.send_all_data_response(&payloads, request)
    }

    fn refresh_motor_runtime_state<M: MotorTelemetryBindings>(
        &mut self,
        telemetry: &MotorTelemetryApi<M>,
    ) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let motor = base.motor();
        // Refloat v1.2.1 updates motor fields in `motor_data_update` at
        // `third_party/refloat/src/motor_data.c:108-145`. Battery current uses the same first-order
        // smoothing expression from `third_party/refloat/src/motor_data.c:140`; this app-data
        // refresh is still a runtime proxy until the real source main loop runs.
        let previous_battery_current = motor.battery_current().current().as_amps();
        let next_battery_current = telemetry.battery_current().current().as_amps();
        self.motor_current_max = telemetry.motor_current_max();
        self.motor_current_min = telemetry.motor_current_min();
        let electrical_speed = telemetry.electrical_speed();
        let motor_erpm = electrical_speed.rpm().as_revolutions_per_minute();
        let current_acceleration = motor_erpm - self.motor_last_erpm;
        self.motor_last_erpm = motor_erpm;
        // Upstream averages acceleration over `ACCEL_ARRAY_SIZE == 40` samples
        // in `third_party/refloat/src/motor_data.c:128-133`.
        let accel_idx = self.motor_accel_idx.min(self.motor_accel_history.len() - 1);
        let Some(previous_acceleration) = self.motor_accel_history.get(accel_idx).copied() else {
            return;
        };
        self.motor_acceleration += (current_acceleration - previous_acceleration) / 40.0;
        let Some(history) = self.motor_accel_history.get_mut(accel_idx) else {
            return;
        };
        *history = current_acceleration;
        self.motor_accel_idx = (accel_idx + 1) % 40;
        let motor = RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(telemetry.input_voltage_filtered().voltage()),
            electrical_speed,
            telemetry.vehicle_speed(),
            telemetry.motor_current(),
            BatteryCurrent::new(Current::from_amps(
                previous_battery_current + 0.01 * (next_battery_current - previous_battery_current),
            )),
            telemetry.duty_cycle_now(),
            // Upstream compact all-data reads optional `VESC_IF->foc_get_id` at
            // `third_party/refloat/src/main.c:1364-1368` and writes 222 when the slot is absent.
            telemetry.foc_id_current().map_or(
                RefloatFocIdCurrent::unavailable(),
                RefloatFocIdCurrent::measured,
            ),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            motor,
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    #[cfg(any(test, target_arch = "arm"))]
    #[inline(always)]
    pub(crate) fn refresh_footpad_runtime_state(&mut self, adc1: f32, adc2: f32) {
        let adc2 = if adc2 < 0.0 { 0.0 } else { adc2 };
        let fault_adc1 = f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC1_OFFSET)) / 1000.0;
        let fault_adc2 = f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC2_OFFSET)) / 1000.0;
        // C map: Refloat v1.2.1 `footpad_sensor_update` decodes the switch
        // state from raw ADC volts at `third_party/refloat/src/footpad_sensor.c:28-61`.
        let mut state = FootpadSensorState::None;
        if fault_adc1 == 0.0 && fault_adc2 == 0.0 {
            state = FootpadSensorState::Both;
        } else if fault_adc2 == 0.0 {
            if adc1 > fault_adc1 {
                state = FootpadSensorState::Both;
            }
        } else if fault_adc1 == 0.0 {
            if adc2 > fault_adc2 {
                state = FootpadSensorState::Both;
            }
        } else if adc1 > fault_adc1 {
            state = if adc2 > fault_adc2 {
                FootpadSensorState::Both
            } else {
                FootpadSensorState::Left
            };
        } else if adc2 > fault_adc2 {
            state = FootpadSensorState::Right;
        }
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            crate::domain::FootpadSensorSample::from_adc_volts(adc1, adc2, state),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn refresh_imu_runtime_state<I: ImuBindings>(
        &mut self,
        imu: &ImuApi<I>,
        system_time_ticks: u32,
    ) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let resets_runtime_vars =
            matches!(ride_state.run_state(), RefloatRunState::Startup) && imu.startup_done();
        let run_state = match (ride_state.run_state(), imu.startup_done()) {
            (RefloatRunState::Startup, true) => RefloatRunState::Ready,
            (run_state, _) => run_state,
        };
        let flywheel_both_footpads_fault = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Running,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let reverse_stop_no_footpads_fault = matches!(
            (
                run_state,
                ride_state.setpoint_adjustment(),
                base.footpad().state()
            ),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop,
                FootpadSensorState::None
            )
        );
        let reverse_stop_pitch_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && imu.pitch().angle().as_radians().abs()
            > 18.0_f32.to_radians();
        let reverse_stop_timer_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && {
            let pitch = imu.pitch().angle().as_radians().abs();
            (pitch > 10.0_f32.to_radians()
                && refloat_ticks_elapsed(system_time_ticks, self.reverse_ticks, 1))
                || (pitch > 5.0_f32.to_radians()
                    && refloat_ticks_elapsed(system_time_ticks, self.reverse_ticks, 2))
        };
        let reverse_stop_total_erpm_fault = matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) && self.reverse_total_erpm.abs() > 200_000.0;
        let motor_erpm = base
            .motor()
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute();
        let pitch = imu.pitch().angle().as_radians();
        // C updates `imu.balance_pitch` from the Refloat-owned balance filter
        // before control at `third_party/refloat/src/main.c:760-775`, `third_party/refloat/src/imu.c:35-41`, and
        // `third_party/refloat/src/balance_filter.c:145-154`; FLYWHEEL then overrides it with raw
        // pitch at `third_party/refloat/src/imu.c:56-58`.
        let balance_pitch = if matches!(ride_state.mode(), RefloatMode::Flywheel) {
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(pitch))
        } else {
            self.balance_filter.balance_pitch()
        };
        let balance_pitch_radians = balance_pitch.angle().as_radians();
        let balance_pitch_degrees = balance_pitch_radians * 180.0 / core::f32::consts::PI;
        let quickstop_fault = matches!(
            (run_state, base.footpad().state(), ride_state.mode()),
            (
                RefloatRunState::Running,
                FootpadSensorState::None,
                mode
            ) if !matches!(mode, RefloatMode::Flywheel)
        ) && self.config_byte(REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET) != 0
            && motor_erpm.abs() < 200.0
            && pitch.abs() > 14.0_f32.to_radians()
            && base.setpoints().remote().angle().as_degrees().abs() < 30.0
            && (pitch >= 0.0) == (motor_erpm >= 0.0);
        let single_footpad = matches!(
            base.footpad().state(),
            FootpadSensorState::Left | FootpadSensorState::Right
        );
        let dual_switch = self.config_byte(REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET) != 0;
        let simple_start = self.config_byte(REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET) != 0
            && (refloat_ticks_elapsed(system_time_ticks, self.disengage_ticks, 2)
                || !refloat_ticks_elapsed(system_time_ticks, self.engage_ticks, 1));
        let can_engage = matches!(ride_state.charging(), RefloatChargingState::NotCharging)
            && (matches!(base.footpad().state(), FootpadSensorState::Both)
                || single_footpad && (dual_switch || simple_start)
                || matches!(ride_state.mode(), RefloatMode::Flywheel));
        let fault_adc_half_erpm =
            f32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_ADC_HALF_ERPM_OFFSET));
        let fault_delay_switch_half =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_SWITCH_HALF_OFFSET));
        let fault_delay_switch_full =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_SWITCH_FULL_OFFSET));
        let switch_faults_disabled =
            self.config_byte(REFLOAT_CONFIG_FAULT_MOVING_FAULT_DISABLED_OFFSET) != 0
                && motor_erpm > fault_adc_half_erpm * 2.0
                && imu.roll().angle().as_radians().abs() < 40.0_f32.to_radians();
        let full_switch_pending = matches!(run_state, RefloatRunState::Running)
            && matches!(base.footpad().state(), FootpadSensorState::None)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel);
        let full_switch_fault = full_switch_pending
            && !switch_faults_disabled
            && (refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_switch_ticks,
                fault_delay_switch_full,
            ) || motor_erpm.abs() < fault_adc_half_erpm * 6.0
                && refloat_ticks_elapsed_ms(
                    system_time_ticks,
                    self.fault_switch_ticks,
                    fault_delay_switch_half,
                ));
        let half_switch_pending = matches!(run_state, RefloatRunState::Running)
            && !dual_switch
            && !can_engage
            && motor_erpm.abs() < fault_adc_half_erpm;
        let half_switch_fault = half_switch_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_switch_half_ticks,
                fault_delay_switch_half,
            );
        let fault_roll = self.config_scaled_i16(REFLOAT_CONFIG_FAULT_ROLL_OFFSET, 10.0);
        let fault_delay_roll =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET));
        let roll_fault_pending = matches!(run_state, RefloatRunState::Running)
            && imu.roll().angle().as_radians().abs() > fault_roll.to_radians();
        let roll_fault = roll_fault_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_angle_roll_ticks,
                fault_delay_roll,
            );
        let fault_pitch = self.config_scaled_i16(REFLOAT_CONFIG_FAULT_PITCH_OFFSET, 10.0);
        let fault_delay_pitch =
            u32::from(self.config_be_u16(REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET));
        let pitch_fault_pending = matches!(run_state, RefloatRunState::Running)
            && imu.pitch().angle().as_radians().abs() > fault_pitch.to_radians()
            && base.setpoints().remote().angle().as_degrees().abs() < 30.0;
        let pitch_fault = pitch_fault_pending
            && refloat_ticks_elapsed_ms(
                system_time_ticks,
                self.fault_angle_pitch_ticks,
                fault_delay_pitch,
            );
        let ready_flywheel_stop = matches!(
            (run_state, ride_state.mode(), base.footpad().state()),
            (
                RefloatRunState::Ready,
                RefloatMode::Flywheel,
                FootpadSensorState::Both
            )
        );
        let darkride_high_erpm_fault = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Active)
        ) && motor_erpm > 2000.0;
        let darkride_can_engage_fault = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Running, RefloatDarkRideState::Active)
        ) && can_engage;
        let darkride_roll_fault =
            matches!(
                (run_state, ride_state.darkride()),
                (RefloatRunState::Running, RefloatDarkRideState::Upright)
            ) && self.config_byte(REFLOAT_CONFIG_FAULT_DARKRIDE_ENABLED_OFFSET) != 0
                && {
                    let roll = imu.roll().angle().as_radians().abs();
                    roll > 100.0_f32.to_radians() && roll < 135.0_f32.to_radians()
                };
        let startup_pitch_tolerance =
            self.config_scaled_i16(REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET, 100.0);
        let startup_roll_tolerance =
            self.config_scaled_i16(REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET, 100.0);
        let ready_engage = matches!(run_state, RefloatRunState::Ready)
            && !ready_flywheel_stop
            && can_engage
            && balance_pitch_radians.abs() < startup_pitch_tolerance.to_radians()
            && imu.roll().angle().as_radians().abs() < startup_roll_tolerance.to_radians();
        let ready_darkride_engage = matches!(
            (run_state, ride_state.darkride()),
            (RefloatRunState::Ready, RefloatDarkRideState::Active)
        ) && balance_pitch_radians.abs()
            < startup_pitch_tolerance.to_radians()
            && !refloat_ticks_elapsed(system_time_ticks, self.disengage_ticks, 1)
            && !matches!(
                ride_state.stop_condition(),
                RefloatStopCondition::ReverseStop
            );
        let ready_push_start = matches!(run_state, RefloatRunState::Ready)
            && self.config_byte(REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET) != 0
            && motor_erpm.abs() > 1000.0
            && can_engage
            && balance_pitch_radians.abs() < core::f32::consts::FRAC_PI_4
            && imu.roll().angle().as_radians().abs() < core::f32::consts::FRAC_PI_4
            && !(self.config_byte(REFLOAT_CONFIG_FAULT_REVERSESTOP_ENABLED_OFFSET) != 0
                && motor_erpm < 0.0);
        let state_engage = ready_engage || ready_darkride_engage || ready_push_start;
        let traction_loss_detected = matches!(run_state, RefloatRunState::Running)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel)
            && self.motor_acceleration.abs() > 15.0
            && self.motor_acceleration.signum() == motor_erpm.signum()
            && base.motor().duty_cycle().ratio().as_ratio() > 0.3
            && motor_erpm.abs() > 2000.0;
        if traction_loss_detected {
            self.traction_control = matches!(ride_state.darkride(), RefloatDarkRideState::Active);
        } else if matches!(ride_state.wheelslip(), RefloatWheelSlipState::Detected)
            && self.motor_acceleration.abs() < 10.0
        {
            self.traction_control = false;
        }
        // Upstream `check_faults(d)` returns immediately after each stop branch
        // in `third_party/refloat/src/main.c:357-509`; this call preserves the
        // same Rust condition priority before `state_stop` writes READY and
        // clears wheelslip at `third_party/refloat/src/state.c:29-33`.
        let stop_event = refloat_first_stop_event(&[
            (
                RefloatStopEvent::FlywheelBothFootpads,
                flywheel_both_footpads_fault,
            ),
            (
                RefloatStopEvent::ReverseStopNoFootpads,
                reverse_stop_no_footpads_fault,
            ),
            (RefloatStopEvent::ReverseStopPitch, reverse_stop_pitch_fault),
            (RefloatStopEvent::ReverseStopTimer, reverse_stop_timer_fault),
            (
                RefloatStopEvent::ReverseStopTotalErpm,
                reverse_stop_total_erpm_fault,
            ),
            (RefloatStopEvent::FullSwitch, full_switch_fault),
            (RefloatStopEvent::QuickStop, quickstop_fault),
            (RefloatStopEvent::HalfSwitch, half_switch_fault),
            (RefloatStopEvent::DarkrideHighErpm, darkride_high_erpm_fault),
            (
                RefloatStopEvent::DarkrideCanEngage,
                darkride_can_engage_fault,
            ),
            (RefloatStopEvent::Roll, roll_fault),
            (RefloatStopEvent::Pitch, pitch_fault),
            (RefloatStopEvent::DarkrideRoll, darkride_roll_fault),
        ]);
        let state_transition = refloat_state_transition(RefloatStateTransitionInput {
            previous: ride_state,
            run_state,
            ready_flywheel_stop,
            state_engage,
            traction_loss_detected,
            stop_event,
        });
        let state_stop_fault = state_transition.state_stopped;
        if state_transition.state_stopped {
            self.disengage_ticks = system_time_ticks;
        } else if state_transition.state_engaged {
            self.engage_ticks = system_time_ticks;
        }
        if !full_switch_pending {
            self.fault_switch_ticks = system_time_ticks;
        }
        if !half_switch_pending {
            self.fault_switch_half_ticks = system_time_ticks;
        }
        if !matches!(
            (run_state, ride_state.setpoint_adjustment()),
            (
                RefloatRunState::Running,
                RefloatSetpointAdjustment::ReverseStop
            )
        ) || imu.pitch().angle().as_radians().abs() < 5.0_f32.to_radians()
        {
            self.reverse_ticks = system_time_ticks;
        }
        if !roll_fault_pending {
            self.fault_angle_roll_ticks = system_time_ticks;
        }
        if !pitch_fault_pending {
            self.fault_angle_pitch_ticks = system_time_ticks;
        }
        // Upstream READY engages at `third_party/refloat/src/main.c:1033-1067`;
        // `state_engage` writes RUNNING/CENTERING/STOP_NONE at
        // `third_party/refloat/src/state.c:36-39`; READY flywheel abort returns
        // to NORMAL before startup checks at `third_party/refloat/src/main.c:957-963`.
        let mut ride_state = state_transition.ride_state;
        let reset_runtime_vars = resets_runtime_vars || state_engage;
        let (mut balance_current, mut setpoints, mut booster_current) = if reset_runtime_vars {
            // Upstream `STATE_STARTUP` calls `reset_runtime_vars(d)` before
            // `STATE_READY` at `third_party/refloat/src/main.c:833-837`, and
            // `engage(d)` calls it before `state_engage(d)` at
            // `third_party/refloat/src/main.c:263-270`; reset clears
            // `balance_current` at `third_party/refloat/src/main.c:246`,
            // resets module setpoints at `third_party/refloat/src/main.c:239-244`,
            // and seeds only the board setpoint from `d->imu.balance_pitch` at
            // `third_party/refloat/src/main.c:249-252`.
            self.pid_integral_current = 0.0;
            self.softstart_pid_limit = 0.0;
            self.reverse_total_erpm = 0.0;
            self.traction_control = false;
            self.rc_current = 0.0;
            self.rc_steps = 0;
            let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
                balance_pitch_degrees,
            ));
            let zero_setpoint =
                RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
            (
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                RefloatRealtimeRuntimeSetpoints::new(
                    setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                    zero_setpoint,
                ),
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            )
        } else {
            (
                base.balance_current(),
                base.setpoints(),
                base.booster_current(),
            )
        };
        if matches!(run_state, RefloatRunState::Running) && !state_engage {
            if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::Centering
            ) {
                let board_setpoint_degrees = setpoints.board().angle().as_degrees();
                if board_setpoint_degrees == 0.0 {
                    // Upstream `calculate_setpoint_target(d)` exits
                    // `SAT_CENTERING` when `setpoint_target_interpolated`
                    // already equals target zero at
                    // `third_party/refloat/src/main.c:517-520`.
                    ride_state = RefloatRideState::new(
                        ride_state.run_state(),
                        ride_state.mode(),
                        RefloatSetpointAdjustment::None,
                        ride_state.stop_condition(),
                    )
                    .with_charging(ride_state.charging())
                    .with_wheelslip(ride_state.wheelslip())
                    .with_darkride(ride_state.darkride());
                } else {
                    let startup_step = self
                        .config_scaled_i16(REFLOAT_CONFIG_STARTUP_SPEED_OFFSET, 100.0)
                        / f32::from(self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET).max(1));
                    let centered_board_degrees = if board_setpoint_degrees.abs() < startup_step {
                        0.0
                    } else {
                        board_setpoint_degrees - startup_step * board_setpoint_degrees.signum()
                    };
                    // Upstream stores `startup_speed / hertz` at
                    // `third_party/refloat/src/main.c:172`, selects it for
                    // `SAT_CENTERING` at `third_party/refloat/src/main.c:304-310`,
                    // applies `rate_limitf` at
                    // `third_party/refloat/src/utils.c:25-33`, and assigns the
                    // centered setpoint before PID at
                    // `third_party/refloat/src/main.c:869-875`.
                    let centered_board = RefloatRealtimeRuntimeSetpoint::new(
                        AngleDegrees::from_degrees(centered_board_degrees),
                    );
                    setpoints = RefloatRealtimeRuntimeSetpoints::new(
                        centered_board,
                        setpoints.atr(),
                        setpoints.brake_tilt(),
                        setpoints.torque_tilt(),
                        setpoints.turn_tilt(),
                        setpoints.remote(),
                    );
                }
            }
            if matches!(
                ride_state.setpoint_adjustment(),
                RefloatSetpointAdjustment::ReverseStop
            ) {
                // Upstream `calculate_setpoint_target(d)` accumulates ERPM
                // while SAT_REVERSESTOP is active at `third_party/refloat/src/main.c:522-525`.
                self.reverse_total_erpm += motor_erpm;
            }
            let [_, gyro_pitch, gyro_yaw] = imu.angular_rate().xyz();
            // Upstream RUNNING executes this exact balance-current pipeline at
            // `third_party/refloat/src/main.c:918-956`; the helper keeps the
            // PID, booster, pitch-rate, soft-start, limit, darkride, and
            // traction branches unit-testable while this method preserves the
            // surrounding state-machine order.
            let balance_loop = refloat_balance_loop_step(
                RefloatBalanceLoopConfig {
                    kp: self.config_scaled_field(REFLOAT_CONFIG_KP_FIELD),
                    kp2: self.config_scaled_field(REFLOAT_CONFIG_KP2_FIELD),
                    ki: self.config_scaled_field(REFLOAT_CONFIG_KI_FIELD),
                    kp_brake: self.config_scaled_field(REFLOAT_CONFIG_KP_BRAKE_FIELD),
                    kp2_brake: self.config_scaled_field(REFLOAT_CONFIG_KP2_BRAKE_FIELD),
                    ki_limit: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_KI_LIMIT_OFFSET, 10.0),
                    )),
                    booster_angle: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET, 100.0),
                    ),
                    booster_ramp: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET, 100.0),
                    ),
                    booster_current: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET, 100.0),
                    )),
                    brkbooster_angle: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET, 100.0),
                    ),
                    brkbooster_ramp: AngleDegrees::from_degrees(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET, 100.0),
                    ),
                    brkbooster_current: MotorCurrent::new(Current::from_amps(
                        self.config_scaled_i16(REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET, 100.0),
                    )),
                    hertz: SampleRate::from_hertz(f32::from(
                        self.config_be_u16(REFLOAT_CONFIG_HERTZ_OFFSET),
                    )),
                },
                RefloatBalanceLoopInput {
                    setpoint: setpoints.board(),
                    brake_tilt_setpoint: setpoints.brake_tilt(),
                    balance_pitch,
                    raw_pitch: imu.pitch(),
                    roll: imu.roll(),
                    gyro_pitch,
                    gyro_yaw,
                    motor_erpm: base.motor().electrical_speed(),
                    motor_current: base.motor().motor_current(),
                    motor_current_max: self.motor_current_max,
                    motor_current_min: self.motor_current_min,
                    mode: ride_state.mode(),
                    darkride: ride_state.darkride(),
                    traction_control: self.traction_control,
                },
                RefloatBalanceLoopState {
                    balance_current: balance_current.current(),
                    booster_current: booster_current.current(),
                    pid_integral_current: MotorCurrent::new(Current::from_amps(
                        self.pid_integral_current,
                    )),
                    pid_kp_brake_scale: self.pid_kp_brake_scale,
                    pid_kp2_brake_scale: self.pid_kp2_brake_scale,
                    pid_kp_accel_scale: self.pid_kp_accel_scale,
                    pid_kp2_accel_scale: self.pid_kp2_accel_scale,
                    softstart_pid_limit: MotorCurrent::new(Current::from_amps(
                        self.softstart_pid_limit,
                    )),
                },
            );
            let balance_loop_state = balance_loop.state;
            self.pid_integral_current = balance_loop_state.pid_integral_current.current().as_amps();
            self.pid_kp_brake_scale = balance_loop_state.pid_kp_brake_scale;
            self.pid_kp2_brake_scale = balance_loop_state.pid_kp2_brake_scale;
            self.pid_kp_accel_scale = balance_loop_state.pid_kp_accel_scale;
            self.pid_kp2_accel_scale = balance_loop_state.pid_kp2_accel_scale;
            self.softstart_pid_limit = balance_loop_state.softstart_pid_limit.current().as_amps();
            booster_current =
                RefloatRealtimeBoosterCurrent::new(balance_loop_state.booster_current);
            balance_current =
                RefloatRealtimeBalanceCurrent::new(balance_loop_state.balance_current);
            self.request_motor_current(balance_loop.requested_current);
        } else if matches!(run_state, RefloatRunState::Ready) && !state_stop_fault {
            if self.rc_steps != 0 {
                self.rc_current =
                    self.rc_current * 0.95 + f32::from(self.rc_current_target_deciamps) * 0.005;
                if motor_erpm.abs() > 800.0 {
                    self.rc_current = 0.0;
                }
                self.rc_steps -= 1;
                self.rc_counter += 1;
                if self.rc_counter == 500 && self.rc_current_target_deciamps > 20 {
                    self.rc_current_target_deciamps /= 2;
                }
                // Upstream READY falls through to `do_rc_move(d)` at
                // `third_party/refloat/src/main.c:1069`, where active RC move steps filter/request
                // `rc_current` at `third_party/refloat/src/main.c:276-286`.
                self.request_motor_current(MotorCurrent::new(Current::from_amps(self.rc_current)));
            } else {
                let remote_throttle_current_max =
                    self.config_scaled_i16(REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET, 10.0);
                let remote_throttle_grace_period = self
                    .config_scaled_i16(REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET, 10.0);
                if remote_throttle_current_max > 0.0
                    && refloat_ticks_elapsed_f32(
                        system_time_ticks,
                        self.disengage_ticks,
                        remote_throttle_grace_period,
                    )
                    && self.remote_input.abs() > 0.02
                {
                    let servo_val =
                        if self.config_byte(REFLOAT_CONFIG_INPUTTILT_INVERT_THROTTLE_OFFSET) != 0 {
                            -self.remote_input
                        } else {
                            self.remote_input
                        };
                    self.rc_current =
                        self.rc_current * 0.95 + remote_throttle_current_max * servo_val * 0.05;
                    // Upstream READY falls through to `do_rc_move(d)` at
                    // `third_party/refloat/src/main.c:1069`, where the remote-throttle idle branch
                    // filters and requests `rc_current` at `third_party/refloat/src/main.c:291-298`.
                    self.request_motor_current(MotorCurrent::new(Current::from_amps(
                        self.rc_current,
                    )));
                } else {
                    self.rc_current = 0.0;
                }
            }
        }
        // C publishes the just-refreshed `imu.balance_pitch` through app-data;
        // normal mode comes from the balance filter at `third_party/refloat/src/imu.c:35-41`, while
        // FLYWHEEL mirrors raw pitch at `third_party/refloat/src/imu.c:56-58`.
        let base = RefloatAllDataBasePayload::new(
            balance_current,
            RefloatAllDataAttitude::new(balance_pitch, imu.roll(), imu.pitch()),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            setpoints,
            booster_current,
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn handle_handtest_packet(&mut self, bytes: &[u8]) -> bool {
        // QML sends `[101, COMMAND_HANDTEST, on]` from
        // `ui.qml.in:764-768`; Refloat C dispatches it at
        // `third_party/refloat/src/main.c:2226-2228` and applies READY/NORMAL/HANDTEST gates at
        // `third_party/refloat/src/main.c:1421-1430`.
        let [package_id, command_id, on, ..] = bytes else {
            return false;
        };
        if *package_id != REFLOAT_APP_DATA_PACKAGE_ID.get()
            || RefloatAppDataCommand::try_from_id(*command_id)
                != Ok(RefloatAppDataCommand::HandTest)
        {
            return false;
        }

        let ride_state = self.all_data_payloads.base().status().ride_state();
        if !matches!(
            (ride_state.run_state(), ride_state.mode()),
            (
                RefloatRunState::Ready,
                RefloatMode::Normal | RefloatMode::HandTest
            )
        ) {
            return true;
        }

        let mode = if *on == 0 {
            RefloatMode::Normal
        } else {
            RefloatMode::HandTest
        };
        self.set_ride_mode(mode);
        self.apply_handtest_config(matches!(mode, RefloatMode::HandTest));
        true
    }

    fn set_ride_mode(&mut self, mode: RefloatMode) {
        // HANDTEST changes only `state.mode` in C at `third_party/refloat/src/main.c:1430`;
        // preserve the rest of the packed Rust ride state while swapping mode.
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let ride_state = RefloatRideState::new(
            ride_state.run_state(),
            mode,
            ride_state.setpoint_adjustment(),
            ride_state.stop_condition(),
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(ride_state.wheelslip())
        .with_darkride(ride_state.darkride());
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn apply_handtest_config(&mut self, enabled: bool) {
        if !enabled {
            if let Some(config) = self.handtest_config_backup.take() {
                self.serialized_config = config;
            }
            return;
        }

        if self.handtest_config_backup.is_none() {
            self.handtest_config_backup = Some(self.serialized_config);
        }

        // Refloat C applies temporary HANDTEST safety config at
        // `third_party/refloat/src/main.c:1431-1446` and restores from EEPROM on off at
        // `third_party/refloat/src/main.c:1447-1449`.
        let mut config = self.serialized_config;
        let writes_ok = [
            (REFLOAT_CONFIG_KI_OFFSET, 0),
            (REFLOAT_CONFIG_KP_BRAKE_OFFSET, 100),
            (REFLOAT_CONFIG_KP2_BRAKE_OFFSET, 100),
            (REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET, 10_000),
            (REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET, 10_000),
            (REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET, 0),
            (REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET, 0),
            (REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET, 0),
            (REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET, 0),
            (REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET, 0),
            (REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET, 0),
            (REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET, 0),
            (REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET, 50),
            (REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET, 50),
        ]
        .into_iter()
        .all(|(offset, value)| Self::set_config_be_u16(&mut config, offset, value));
        if writes_ok {
            self.serialized_config = config;
        }
    }

    fn handle_charging_state_packet(&mut self, bytes: &[u8]) -> bool {
        // Refloat v1.2.1 routes COMMAND_CHARGING_STATE at `third_party/refloat/src/main.c:2267-2269`;
        // the command ID is defined in `third_party/refloat/src/charging.h:25`.
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

    fn send_response_bytes(&self, bytes: &[u8]) -> bool {
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

unsafe extern "C" fn stop_refloat_app_data(_arg: *mut core::ffi::c_void) {
    // C map: Refloat v1.2.1 `stop` starts at `third_party/refloat/src/main.c:2399`.
    // Upstream stop cleanup in `third_party/refloat/src/main.c:2398-2412` clears IMU/app-data/custom
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
        crate::runtime::request_refloat_runtime_thread_termination(unsafe { ptr.as_ref() });
        let _allocation =
            unsafe { vescpkg_rs::FirmwareAllocation::from_raw_parts(ptr, 1, &bindings) };
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::test_support::{
        RecordingAllocBindings, RecordingAppDataBindings, sample_all_data_payloads,
        sample_all_data_payloads_with_ride_state,
    };
    use super::{RefloatAppDataLifecycle, RefloatAppDataState, RefloatBalanceFilter};
    use super::{
        allocate_refloat_startup_app_data_with, handle_refloat_app_data_packet,
        install_refloat_startup_app_data_with,
    };
    use crate::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataMotorPayload,
        RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState,
        RefloatDarkRideState, RefloatFocIdCurrent, RefloatMode, RefloatRealtimeBalanceCurrent,
        RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent, RefloatRealtimeRuntimeSetpoint,
        RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState,
        RefloatSetpointAdjustment, RefloatStopCondition, RefloatWheelSlipState,
    };
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::{
        FakeImuBindings, FakeMotorControlBindings, FakeMotorTelemetryBindings,
    };
    use vescpkg_rs::{AppDataBindings, FirmwareAllocator, ffi};

    fn tick_refloat_state_and_handle_packet<B, M, I>(
        state: &mut RefloatAppDataState,
        lifecycle: &RefloatAppDataLifecycle<B>,
        telemetry: &MotorTelemetryApi<M>,
        imu: &ImuApi<I>,
        bytes: &[u8],
    ) -> bool
    where
        B: AppDataBindings,
        M: MotorTelemetryBindings,
        I: ImuBindings,
    {
        state.refresh_runtime_state(telemetry, imu, lifecycle.bindings().system_time_ticks());
        state.handle_packet_with_runtime(lifecycle, telemetry, imu, bytes)
    }

    fn balance_filter_with_pitch(pitch_radians: f32) -> RefloatBalanceFilter {
        // Refloat reads pitch from quaternion with
        // `balance_filter_get_pitch` at `third_party/refloat/src/balance_filter.c:145-154`.
        RefloatBalanceFilter::from_quaternions([
            libm::cosf(pitch_radians * 0.5),
            0.0,
            libm::sinf(pitch_radians * 0.5),
            0.0,
        ])
    }

    #[test]
    fn app_data_callback_dispatches_without_main_loop_refresh_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));

        assert!(state.handle_packet_with_runtime(
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Upstream `on_command_received` only dispatches app commands at
        // `third_party/refloat/src/main.c:2143-2225`; READY engage and
        // IMU/motor refresh stay in `refloat_thd` at `third_party/refloat/src/main.c:772-1080`.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            RefloatRunState::Ready
        );
    }

    #[test]
    fn default_config_decodes_pid_scales_like_refloat_settings() {
        let state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));

        // Refloat generated settings serialize `kp` with scale 10 at
        // `third_party/refloat/src/conf/settings.xml:28-54`, `kp2` with scale
        // 100 at `third_party/refloat/src/conf/settings.xml:55-84`, and
        // `kp2_brake` with scale 100 at
        // `third_party/refloat/src/conf/settings.xml:199-222`.
        assert_eq!(
            state.config_scaled_field(super::REFLOAT_CONFIG_KP_FIELD),
            20.0
        );
        assert_eq!(
            state.config_scaled_field(super::REFLOAT_CONFIG_KP2_FIELD),
            0.6
        );
        assert_eq!(
            state.config_scaled_field(super::REFLOAT_CONFIG_KP2_BRAKE_FIELD),
            1.0
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
        // Refloat v1.2.1 stop clears IMU/app-data/custom config callbacks at
        // `third_party/refloat/src/main.c:2401-2403`.
        assert_eq!(lifecycle.bindings().imu_read_callback_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_imu_read_callback.get(), 0);
        assert_eq!(lifecycle.bindings().custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn registers_imu_callback_like_refloat_startup() {
        let bindings = RecordingAppDataBindings::accepting();

        assert!(super::register_refloat_imu_callback_with(&bindings));

        // Refloat registers `imu_ref_callback` during startup at
        // `third_party/refloat/src/main.c:2455`.
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);
        assert_ne!(bindings.last_imu_read_callback.get(), 0);
    }

    #[test]
    fn lifecycle_sends_refloat_app_data_responses_through_bindings() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());

        assert!(lifecycle.send_response(
            &sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 4]);

        assert!(lifecycle.send_response(
            &sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::Info.id(),
                2,
                0,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 2);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 60);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 0, 2]);

        assert!(!lifecycle.send_response(
            &sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::PrintInfo.id(),
                4,
            ],
        ));
        assert_eq!(lifecycle.bindings().send_calls.get(), 2);
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
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
        );
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
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_temperatures(
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
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_ride_totals(
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
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_firmware_fault(FirmwareFaultCode::from_compat_code(5)),
        );
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
    fn app_data_handtest_command_toggles_ready_mode_like_refloat_qml() {
        // QML sends COMMAND_HANDTEST at `refloat/ui.qml.in:764-768`; C toggles
        // mode and temporary safety config at `third_party/refloat/src/main.c:1421-1450`.
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));
        let original_config = *state.serialized_config();

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::HandTest.id(),
                1,
            ],
        ));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            RefloatMode::HandTest
        );
        assert_eq!(state.config_be_u16(super::REFLOAT_CONFIG_KI_OFFSET), 0);
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_KP_BRAKE_OFFSET),
            100
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_KP2_BRAKE_OFFSET),
            100
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET),
            10_000
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET),
            10_000
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET),
            0
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET),
            50
        );
        assert_eq!(
            state.config_be_u16(super::REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET),
            50
        );

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::HandTest.id(),
                0,
            ],
        ));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            RefloatMode::Normal
        );
        assert_eq!(state.serialized_config(), &original_config);
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
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
        );
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
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_input_voltage_filtered(InputVoltage::new(Voltage::from_volts(84.2))),
        );
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
    fn app_data_state_refreshes_realtime_voltage_and_temperatures_from_motor_telemetry() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::with_input_voltage_and_temperatures(
                InputVoltage::new(Voltage::from_volts(84.2)),
                MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
                MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
            ),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        // Refloat writes realtime values as float16 at `third_party/refloat/src/main.c:1943-1954`
        // using `buffer_append_float16_auto` from `third_party/refloat/src/conf/buffer.c:143-145`.
        assert_eq!(lifecycle.bindings().send_calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_sent_len.get(), 53);
        assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 31, 4]);
        assert_eq!(
            lifecycle.bindings().last_sent_realtime_voltage_bytes.get(),
            [85, 67]
        );
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_realtime_temperature_bytes
                .get(),
            [80, 160, 82, 16]
        );
        assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
        assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
        assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
    }

    #[test]
    fn app_data_state_refreshes_realtime_timestamp_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(0x0102_0304),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Refloat v1.2.1 writes `d->time.now` into realtime packets at
        // `third_party/refloat/src/main.c:1931`; VESC system ticks are 100 us ticks.
        assert_eq!(
            lifecycle
                .bindings()
                .last_sent_realtime_timestamp_bytes
                .get(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn app_data_runtime_refreshes_startup_ready_gate_and_imu_attitude_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.25)),
                    ImuPitch::new(AngleRadians::from_radians(-0.125)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let payloads = state.all_data_payloads();
        assert_eq!(
            payloads.base().status().ride_state().run_state(),
            RefloatRunState::Ready
        );
        assert_eq!(payloads.base().attitude().roll().angle().as_radians(), 0.25);
        assert_eq!(
            payloads.base().attitude().pitch().angle().as_radians(),
            -0.125
        );
    }

    #[test]
    fn app_data_startup_ready_resets_runtime_vars_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.25)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Startup,
            RefloatMode::Normal,
        ));
        state.balance_filter = balance_filter_with_pitch(1.2);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        assert_eq!(
            base.status().ride_state().run_state(),
            RefloatRunState::Ready
        );
        // Refloat calls `reset_runtime_vars(d)` before READY at
        // `third_party/refloat/src/main.c:833-837`; reset clears
        // `balance_current` at `third_party/refloat/src/main.c:246`, resets
        // module setpoints at `third_party/refloat/src/main.c:239-244`, then
        // seeds only the board setpoint from balance pitch at
        // `third_party/refloat/src/main.c:249-252`.
        assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
        assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
        let expected_startup_setpoint = 1.2 * 180.0 / core::f32::consts::PI;
        assert!(
            (base.setpoints().board().angle().as_degrees() - expected_startup_setpoint).abs()
                < 0.0001
        );
        [
            base.setpoints().atr(),
            base.setpoints().brake_tilt(),
            base.setpoints().torque_tilt(),
            base.setpoints().turn_tilt(),
            base.setpoints().remote(),
        ]
        .into_iter()
        .for_each(|setpoint| assert_eq!(setpoint.angle().as_degrees(), 0.0));
    }

    #[test]
    fn app_data_ready_uses_configured_startup_tolerances_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET
            ..super::REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET + 2]
            .copy_from_slice(&400u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET
            ..super::REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET + 2]
            .copy_from_slice(&4500u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Upstream READY engages only inside configured startup pitch/roll
        // tolerances at `third_party/refloat/src/main.c:1033-1036`; default pitch tolerance is 4
        // degrees, not the broad 45 degree fallback used by earlier Rust code.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            RefloatRunState::Ready
        );
    }

    #[test]
    fn app_data_ready_pushstart_uses_wide_pitch_gate_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY push-start engages above 1000 ERPM when `can_engage`
        // passes and pitch/roll are within 45 degrees at `third_party/refloat/src/main.c:1056-1067`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
    }

    #[test]
    fn app_data_ready_pushstart_reverse_stop_blocks_negative_erpm_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-1200.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET] = 1;
        config[super::REFLOAT_CONFIG_FAULT_REVERSESTOP_ENABLED_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream ignores backwards push-start when reverse stop is enabled
        // at `third_party/refloat/src/main.c:1061-1064`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    }

    #[test]
    fn app_data_running_flywheel_both_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops RUNNING FLYWHEEL when both footpads
        // are engaged at `third_party/refloat/src/main.c:491-493`; `state_stop` moves to READY
        // and stores the stop condition at `third_party/refloat/src/state.c:29-33`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_running_flywheel_stop_clears_wheelslip_like_refloat_state_stop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads = sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
        );
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_wheelslip(RefloatWheelSlipState::Detected);
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `state_stop` clears wheelslip at `third_party/refloat/src/state.c:29-33`.
        assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::None);
    }

    #[test]
    fn app_data_running_reverse_stop_no_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` immediately stops reverse-stop mode when
        // the footpad is fully open at `third_party/refloat/src/main.c:418-422`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchFull
        );
    }

    #[test]
    fn app_data_running_quickstop_no_footpads_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(15.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let motor = RefloatAllDataMotorPayload::new(
            base.motor().battery_voltage(),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
            base.motor().vehicle_speed(),
            base.motor().motor_current(),
            base.motor().battery_current(),
            base.motor().duty_cycle(),
            base.motor().foc_id_current(),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            setpoints,
            base.booster_current(),
            motor,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET] = 1;
        state.store_serialized_config(&config);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` quick-stops no-footpad low-speed
        // pitch-runaway cases at `third_party/refloat/src/main.c:419-423`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::QuickStop);
    }

    #[test]
    fn app_data_running_full_switch_stopped_after_delay_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops a fully open switch after
        // `fault_delay_switch_full` at `third_party/refloat/src/main.c:397-404`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchFull
        );
    }

    #[test]
    fn app_data_running_half_switch_stopped_after_delay_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let motor = RefloatAllDataMotorPayload::new(
            base.motor().battery_voltage(),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
            base.motor().vehicle_speed(),
            base.motor().motor_current(),
            base.motor().battery_current(),
            base.motor().duty_cycle(),
            base.motor().foc_id_current(),
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            motor,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops a partially open switch below
        // `fault_adc_half_erpm` after `fault_delay_switch_half` at
        // `third_party/refloat/src/main.c:459-467`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_running_reverse_stop_high_pitch_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(19.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` immediately stops reverse-stop mode when
        // `fabsf(d->imu.pitch) > 18` at `third_party/refloat/src/main.c:423-426`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::ReverseStop
        );
    }

    #[test]
    fn app_data_running_reverse_stop_pitch_timer_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(11_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(11.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops reverse-stop mode when pitch stays
        // above 10 degrees for 1 second at `third_party/refloat/src/main.c:440-443`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::ReverseStop
        );
    }

    #[test]
    fn app_data_running_reverse_stop_total_erpm_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(201_000.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::ReverseStop,
            RefloatStopCondition::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        for _ in 0..2 {
            assert!(tick_refloat_state_and_handle_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::RealtimeData.id(),
                ],
            ));
        }

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream accumulates reverse-stop ERPM at `third_party/refloat/src/main.c:522-525`, then
        // stops once it exceeds `reverse_tolerance * 10` at `third_party/refloat/src/main.c:450-452`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::ReverseStop
        );
    }

    #[test]
    fn app_data_running_darkride_footpads_stop_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                base.balance_current(),
                base.attitude(),
                RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
                base.footpad(),
                base.setpoints(),
                base.booster_current(),
                base.motor(),
            ),
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream darkride `check_faults(d)` allows turning it off by
        // engaging foot sensors at `third_party/refloat/src/main.c:387-390`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_running_darkride_simple_start_single_footpad_stops_during_engage_grace() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(5_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                base.balance_current(),
                base.attitude(),
                RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
                single_footpad,
                base.setpoints(),
                base.booster_current(),
                base.motor(),
            ),
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *include_bytes!("conf/default_config.dat");
        config[super::REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream simple-start `can_engage(d)` accepts one sensor during the
        // first second after engage at `third_party/refloat/src/main.c:338-344`; darkride
        // `check_faults(d)` then stops at `third_party/refloat/src/main.c:387-390`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::SwitchHalf
        );
    }

    #[test]
    fn app_data_running_darkride_high_erpm_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2100.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                base.balance_current(),
                base.attitude(),
                RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
                no_footpads,
                base.setpoints(),
                base.booster_current(),
                base.motor(),
            ),
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream darkride `check_faults(d)` immediately reverse-stops above
        // 2000 ERPM at `third_party/refloat/src/main.c:363-373`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.stop_condition(),
            RefloatStopCondition::ReverseStop
        );
    }

    #[test]
    fn app_data_running_roll_stopped_after_delay_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(70.0_f32.to_radians())),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let mut state = RefloatAppDataState::new(payloads);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops roll above `fault_roll` after
        // `fault_delay_roll` at `third_party/refloat/src/main.c:474-482`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
    }

    #[test]
    fn app_data_running_pitch_stopped_after_delay_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(70.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let mut state = RefloatAppDataState::new(payloads);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `check_faults(d)` stops pitch above `fault_pitch` after
        // `fault_delay_pitch` when remote setpoint is below 30 degrees at
        // `third_party/refloat/src/main.c:497-503`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Pitch);
    }

    #[test]
    fn app_data_running_darkride_enabled_high_roll_stops_like_refloat_fault_check() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(110.0_f32.to_radians())),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            RefloatAllDataBasePayload::new(
                base.balance_current(),
                base.attitude(),
                base.status(),
                no_footpads,
                base.setpoints(),
                base.booster_current(),
                base.motor(),
            ),
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_FAULT_DARKRIDE_ENABLED_OFFSET] = 1;
        state.store_serialized_config(&config);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream non-darkride `check_faults(d)` stops immediately when
        // darkride faults are enabled and roll is 100-135 degrees at
        // `third_party/refloat/src/main.c:465-470`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
    }

    #[test]
    fn app_data_ready_darkride_first_second_engages_without_roll_gate_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(5_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(170.0_f32.to_radians())),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY darkride ignores roll during the first second after
        // disengage at `third_party/refloat/src/main.c:1038-1054`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
    }

    #[test]
    fn app_data_ready_normal_both_footpads_engages_like_refloat_start_conditions() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(0.05);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY engages when startup pitch/roll tolerances and
        // `can_engage(d)` pass at `third_party/refloat/src/main.c:1033-1036`; `state_engage`
        // moves to RUNNING and sets SAT_CENTERING at `third_party/refloat/src/state.c:36-39`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
        assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
    }

    #[test]
    fn app_data_ready_engage_resets_runtime_vars_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(0.05);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        let ride_state = base.status().ride_state();
        // Upstream `engage(d)` calls `reset_runtime_vars(d)` before
        // `state_engage(d)` at `third_party/refloat/src/main.c:263-270`, then
        // breaks out of the READY branch without running the RUNNING
        // balance-current loop.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
        assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
        let expected_engage_setpoint = 0.05 * 180.0 / core::f32::consts::PI;
        assert_eq!(
            base.setpoints().board().angle().as_degrees(),
            expected_engage_setpoint
        );
        assert_eq!(base.setpoints().remote().angle().as_degrees(), 0.0);
        assert!(!state.apply_requested_motor_current(&motor));
    }

    #[test]
    fn app_data_ready_normal_charging_does_not_engage_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let charging_state = base
            .status()
            .ride_state()
            .with_charging(RefloatChargingState::Charging);
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(charging_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` rejects charging state before checking
        // footpads at `third_party/refloat/src/main.c:328-331`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.charging(), RefloatChargingState::Charging);
    }

    #[test]
    fn app_data_ready_remote_throttle_requests_idle_current_like_refloat_do_rc_move() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(1),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET
            ..super::REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET + 2]
            .copy_from_slice(&100i16.to_be_bytes());
        config[super::REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET
            ..super::REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET + 2]
            .copy_from_slice(&0i16.to_be_bytes());
        assert!(state.store_serialized_config(&config));
        state.set_remote_input_for_test(0.5);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `do_rc_move(d)` uses default inverted throttle and filters
        // `rc_current = old * 0.95 + target * 0.05` before requesting current
        // at `third_party/refloat/src/main.c:291-298`; 10A max with 50% input requests -0.25A.
        assert_eq!(motor.bindings().current().current().as_amps(), -0.25);
    }

    #[test]
    fn app_data_rc_move_command_steps_idle_current_like_refloat_do_rc_move() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RcMove.id(),
                1,
                40,
                2,
                42,
            ],
        ));
        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `cmd_rc_move` sets `rc_steps = time * 100` and target
        // current/10 at `third_party/refloat/src/main.c:1747-1756`; `do_rc_move` filters the first
        // READY tick by 5% at `third_party/refloat/src/main.c:276-286`.
        assert!((motor.bindings().current().current().as_amps() - 0.2).abs() < 0.0001);
    }

    #[test]
    fn app_data_rc_move_halves_large_target_after_500_steps_like_refloat_do_rc_move() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(state.handle_packet_with_telemetry(
            &lifecycle,
            &telemetry,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RcMove.id(),
                1,
                60,
                6,
                66,
            ],
        ));
        for _ in 0..500 {
            assert!(tick_refloat_state_and_handle_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::RealtimeData.id(),
                ],
            ));
        }

        // Upstream `do_rc_move(d)` halves targets above 2A when `rc_counter`
        // reaches 500 at `third_party/refloat/src/main.c:281-284`, after decrementing steps.
        assert_eq!(state.rc_current_target_deciamps, 30);
        assert_eq!(state.rc_steps, 100);
    }

    #[test]
    fn app_data_ready_flywheel_without_footpads_engages_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Flywheel);
        let base = payloads.base();
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` keeps FLYWHEEL mode engaged after footpad
        // checks at `third_party/refloat/src/main.c:346-349`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_ready_flywheel_both_footpads_stops_flywheel_like_refloat_ready_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Flywheel,
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream READY handles FLYWHEEL abort/both-footpad before start
        // conditions at `third_party/refloat/src/main.c:957-963`; `flywheel_stop` returns to
        // NORMAL mode at `third_party/refloat/src/main.c:1869-1873`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(ride_state.mode(), RefloatMode::Normal);
    }

    #[test]
    fn app_data_ready_single_footpad_engages_when_dual_switch_config_is_set() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *include_bytes!("conf/default_config.dat");
        config[super::REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` allows a single footpad when
        // `fault_is_dual_switch` is enabled at `third_party/refloat/src/main.c:338-342`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_ready_single_footpad_default_config_does_not_engage_like_refloat_can_engage() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let upright_base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            upright_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` keeps a single footpad gated unless
        // `fault_is_dual_switch` or simple start is enabled at `third_party/refloat/src/main.c:338-342`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::None
        );
    }

    #[test]
    fn app_data_ready_simple_start_single_footpad_engages_after_disengage_grace_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(20_000),
        );
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.1)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let single_footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::Left,
        );
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *include_bytes!("conf/default_config.dat");
        config[super::REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET] = 1;
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream `can_engage(d)` allows simple-start single-sensor starts
        // two seconds after disengage at `third_party/refloat/src/main.c:338-344`.
        assert_eq!(ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(
            ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_runtime_applies_disabled_config_before_startup_ready_like_refloat() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = vescpkg_rs::ImuApi::new(
            vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
        );
        let mut incoming = *include_bytes!("conf/default_config.dat");
        incoming[super::REFLOAT_CONFIG_DISABLED_OFFSET] = 1;
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));
        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Upstream `configure(d)` applies `disabled` before the control-loop
        // startup gate at `third_party/refloat/src/main.c:184-190`; `state_set_disabled` forces
        // `STATE_DISABLED` at `third_party/refloat/src/state.c:41-47`, so `third_party/refloat/src/main.c:833-838`
        // cannot promote STARTUP to READY in this configuration.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            RefloatRunState::Disabled,
        );
    }

    #[test]
    fn app_data_configured_loop_time_uses_refloat_hertz_config() {
        let mut incoming = *include_bytes!("conf/default_config.dat");
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert_eq!(state.configured_loop_time_us(), 1201);

        incoming[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
            .copy_from_slice(&500u16.to_be_bytes());
        assert!(super::refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        // Upstream generated serialization places `hertz` after the first
        // seven float16 config fields; `configure(d)` then uses it as
        // `1e6 / d->float_conf.hertz` at `third_party/refloat/src/main.c:190-191`.
        assert_eq!(state.configured_loop_time_us(), 2000);
    }

    #[test]
    fn app_data_footpad_runtime_refresh_decodes_adc_like_refloat_sensor_update() {
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        state.refresh_footpad_runtime_state(2.5, -1.0);

        let footpad = state.all_data_payloads().base().footpad();
        // C map: Refloat v1.2.1 `footpad_sensor_update` reads ADCs, clamps
        // missing ADC2 to zero, and decodes the switch state at
        // `third_party/refloat/src/footpad_sensor.c:28-61`.
        assert_eq!(footpad.state(), FootpadSensorState::Left);
        assert_eq!(footpad.adc1_volts(), 2.5);
        assert_eq!(footpad.adc2_volts(), 0.0);
    }

    #[test]
    fn app_data_motor_control_applies_requested_current_like_refloat_motor_control() {
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        state.request_motor_current(MotorCurrent::new(Current::from_amps(6.25)));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `motor_control_apply` resets timeout, keeps current control
        // on for 50ms, sends the requested current, then clears the request at
        // `third_party/refloat/src/motor_control.c:92-99` and `third_party/refloat/src/motor_control.c:121-122`.
        assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
        assert_eq!(motor.bindings().set_current_off_delay_calls.get(), 1);
        assert_eq!(motor.bindings().current_off_delay_seconds(), 0.05);
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().current().current().as_amps(), 6.25);
        assert!(!state.apply_requested_motor_current(&motor));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
    }

    #[test]
    fn app_data_running_runtime_requests_balance_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(1.0_f32.to_radians());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING computes `d->balance_current` and then requests it
        // via `motor_control_request_current` at `third_party/refloat/src/main.c:949-956`.
        assert_eq!(motor.bindings().current().current().as_amps(), 3.8);
    }

    #[test]
    fn app_data_running_motor_apply_uses_current_branch_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.0_f32.to_radians())),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(1.0_f32.to_radians());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_motor_control(&motor, RefloatRunState::Running, 1));

        // Upstream RUNNING computes and requests balance current at
        // `third_party/refloat/src/main.c:918-956`, then `refloat_thd` calls
        // `motor_control_apply` at `third_party/refloat/src/main.c:1076`; a
        // current request takes the `mc_set_current` branch at
        // `third_party/refloat/src/motor_control.c:92-121`.
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().set_brake_current_calls.get(), 0);
        assert_eq!(motor.bindings().set_duty_calls.get(), 0);
        assert_eq!(motor.bindings().current().current().as_amps(), 3.8);
    }

    #[test]
    fn app_data_handtest_running_recenters_start_setpoint_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let payloads = sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::HandTest,
        );
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::HandTest,
            RefloatSetpointAdjustment::Centering,
            RefloatStopCondition::None,
        );
        let board = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0));
        let zero = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(board, zero, zero, zero, zero, zero);
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(0.0);
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET
            ..super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET + 2]
            .copy_from_slice(&5000u16.to_be_bytes());
        state.serialized_config = config;

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        // Refloat RUNNING `SAT_CENTERING` uses `startup_speed / hertz` from
        // `third_party/refloat/src/main.c:172` via
        // `get_setpoint_adjustment_step_size` at
        // `third_party/refloat/src/main.c:304-310`; `rate_limitf` applies that
        // step toward target zero at `third_party/refloat/src/utils.c:25-33`,
        // and the main loop publishes the new setpoint at
        // `third_party/refloat/src/main.c:869-875`.
        assert_eq!(base.setpoints().board().angle().as_degrees(), 1.5);
        assert_eq!(
            base.status().ride_state().setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }

    #[test]
    fn app_data_normal_algorithm_trace_matches_refloat_loop_order() {
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(1.5_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                )
                .with_angular_rate(ImuAngularRate::new([
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                ])),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
        let base = payloads.base();
        let stopped_base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.attitude(),
            base.status(),
            base.footpad(),
            base.setpoints(),
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataMotorPayload::new(
                BatteryVoltage::new(Voltage::from_volts(72.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
                RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(0.0))),
            ),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            stopped_base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.balance_filter = balance_filter_with_pitch(2.0_f32.to_radians());
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET
            ..super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET + 2]
            .copy_from_slice(&5000u16.to_be_bytes());
        state.serialized_config = config;

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &RefloatAppDataLifecycle::new(
                RecordingAppDataBindings::accepting().with_system_time_ticks(0),
            ),
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        let engaged_base = state.all_data_payloads().base();
        let engaged_ride_state = engaged_base.status().ride_state();
        // Upstream READY/NORMAL engages through `engage(d)` at
        // `third_party/refloat/src/main.c:263-270`; `reset_runtime_vars(d)`
        // seeds only the board setpoint from balance pitch at
        // `third_party/refloat/src/main.c:239-252`, and READY breaks before
        // RUNNING PID in `third_party/refloat/src/main.c:1018-1037`.
        assert_eq!(engaged_ride_state.run_state(), RefloatRunState::Running);
        assert_eq!(engaged_ride_state.mode(), RefloatMode::Normal);
        assert_eq!(
            engaged_ride_state.setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
        assert_eq!(engaged_base.setpoints().board().angle().as_degrees(), 2.0);
        assert_eq!(
            engaged_base.balance_current().current().current().as_amps(),
            0.0
        );

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &RefloatAppDataLifecycle::new(
                RecordingAppDataBindings::accepting().with_system_time_ticks(1),
            ),
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        let running_base = state.all_data_payloads().base();
        let kp = state.config_scaled_i16(super::REFLOAT_CONFIG_KP_OFFSET, 10.0);
        let ki = state.config_scaled_i16(super::REFLOAT_CONFIG_KI_OFFSET, 100_000.0);
        let ki_limit = state.config_scaled_i16(super::REFLOAT_CONFIG_KI_LIMIT_OFFSET, 10.0);
        let expected_board_setpoint = 1.5;
        let expected_setpoint_error = expected_board_setpoint - 2.0;
        let unclamped_i = expected_setpoint_error * ki;
        let expected_i = if ki_limit > 0.0 && unclamped_i.abs() > ki_limit {
            ki_limit * unclamped_i.signum()
        } else {
            unclamped_i
        };
        let current_limit = state.motor_current_max.current().as_amps();
        let expected_new_current =
            (expected_setpoint_error * kp + expected_i).clamp(-current_limit, current_limit);
        let expected_smoothed_current = expected_new_current * 0.2;
        // Upstream RUNNING centers with `startup_speed / hertz` at
        // `third_party/refloat/src/main.c:172`,
        // `third_party/refloat/src/main.c:304-310`, and
        // `third_party/refloat/src/main.c:869-875`;
        // then NORMAL PID and the regular motor-current limit run at
        // `third_party/refloat/src/main.c:918-956` before requesting motor
        // current. Raw pitch equals the centered board setpoint here, so the
        // booster proportional is zero by `third_party/refloat/src/main.c:921-922`.
        assert_eq!(
            running_base.setpoints().board().angle().as_degrees(),
            expected_board_setpoint
        );
        assert_eq!(
            running_base.booster_current().current().current().as_amps(),
            0.0
        );
        assert!(
            (running_base.balance_current().current().current().as_amps()
                - expected_smoothed_current)
                .abs()
                < 0.0001
        );

        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        assert!(state.apply_motor_control(
            &motor,
            running_base.status().ride_state().run_state(),
            1,
        ));
        // Upstream main loop calls `motor_control_apply` after the balance loop
        // at `third_party/refloat/src/main.c:1075-1079`; requested current
        // takes the current-control branch at
        // `third_party/refloat/src/motor_control.c:92-99`.
        assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
        assert_eq!(motor.bindings().set_current_off_delay_calls.get(), 1);
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert!(
            (motor.bindings().current().current().as_amps() - expected_smoothed_current).abs()
                < 0.0001
        );
        assert_eq!(motor.bindings().set_duty_calls.get(), 0);
        assert_eq!(motor.bindings().set_brake_current_calls.get(), 0);
    }

    #[test]
    fn app_data_running_computes_angle_p_balance_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `pid_update` computes angle P at `third_party/refloat/src/pid.c:40` and scales
        // it by `kp` at `third_party/refloat/src/pid.c:69`; RUNNING then smooths balance current
        // as `old * 0.8 + new_current * 0.2` at `third_party/refloat/src/main.c:932-954`.
        assert!((motor.bindings().current().current().as_amps() - 12.001).abs() < 0.0001);
        assert!(
            (state
                .all_data_payloads()
                .base()
                .balance_current()
                .current()
                .current()
                .as_amps()
                - 12.001)
                .abs()
                < 0.0001
        );
    }

    #[test]
    fn app_data_running_uses_balance_filter_pitch_like_refloat_pid() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_KP_OFFSET,
            100,
        ));
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_KP2_OFFSET,
            0,
        ));
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_KI_OFFSET,
            0,
        ));
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_KP_BRAKE_OFFSET,
            100,
        ));
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET,
            10_000,
        ));
        assert!(RefloatAppDataState::set_config_be_u16(
            &mut config,
            super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET,
            0,
        ));
        assert!(state.store_serialized_config(&config));
        state.balance_filter = balance_filter_with_pitch(5.0_f32.to_radians());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // C refreshes `imu.balance_pitch` from `balance_filter_get_pitch` at
        // `third_party/refloat/src/imu.c:35-41` before `pid_update` computes
        // `setpoint - imu->balance_pitch` at `third_party/refloat/src/pid.c:40`.
        assert!((motor.bindings().current().current().as_amps() + 10.0).abs() < 0.0001);
        assert!(
            (state
                .all_data_payloads()
                .base()
                .attitude()
                .balance_pitch()
                .angle()
                .as_radians()
                * 180.0
                / core::f32::consts::PI
                - 5.0)
                .abs()
                < 0.0001
        );
    }

    #[test]
    fn balance_filter_update_integrates_positive_pitch_like_refloat_callback() {
        let mut filter = RefloatBalanceFilter::source_startup();

        filter.update([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 0.1);

        // Refloat's `imu_ref_callback` forwards gyro/accel/dt at
        // `third_party/refloat/src/main.c:760-765`; `balance_filter_update` integrates the
        // quaternion at `third_party/refloat/src/balance_filter.c:73-134`, and
        // `balance_filter_get_pitch` reads it at `third_party/refloat/src/balance_filter.c:145-154`.
        assert!(filter.pitch_radians() > 0.0);
    }

    #[test]
    fn imu_callback_state_update_feeds_normal_balance_pitch_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let mut state = RefloatAppDataState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Normal,
        ));

        super::refloat_imu_callback_with_state(&mut state, [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], 0.1);
        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        // Upstream `imu_ref_callback` updates the balance filter at
        // `third_party/refloat/src/main.c:760-765`; the main loop copies that
        // filter into `imu.balance_pitch` at `third_party/refloat/src/imu.c:35-41`
        // before RUNNING PID reads it at `third_party/refloat/src/pid.c:40`.
        assert!(
            state
                .all_data_payloads()
                .base()
                .attitude()
                .balance_pitch()
                .angle()
                .as_radians()
                > 0.0
        );
    }

    #[test]
    fn app_data_running_computes_rate_p_balance_current_like_refloat_pid() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_angular_rate(ImuAngularRate::new([
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(10.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                ])),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&20u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `imu_update` derives pitch rate from gyro at
        // `third_party/refloat/src/imu.c:45-53`; `kp2` uses generated config
        // scale 100 at `third_party/refloat/src/conf/settings.xml:55-84`;
        // `pid_update` computes `rate_p` at `third_party/refloat/src/pid.c:71-72`,
        // then RUNNING smooths it into `balance_current` at
        // `third_party/refloat/src/main.c:921-954`.
        assert!((motor.bindings().current().current().as_amps() + 0.4).abs() < 0.0001);
    }

    #[test]
    fn app_data_running_softstarts_pitch_based_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_angular_rate(ImuAngularRate::new([
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(10.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                ])),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.softstart_pid_limit = 0.0;
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&20u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING soft-start limits only `rate_p + booster.current`
        // before adding Angle P/I at `third_party/refloat/src/main.c:926-930`; a zero first-tick
        // limit removes the -20A Rate-P contribution before smoothing.
        assert_eq!(motor.bindings().current().current().as_amps(), 0.0);
    }

    #[test]
    fn app_data_running_scales_forward_braking_angle_p_like_refloat_pid() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1000.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_BRAKE_OFFSET..super::REFLOAT_CONFIG_KP_BRAKE_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `pid_update` moves forward braking Angle-P scale toward
        // `kp_brake` by 1% per tick at `third_party/refloat/src/pid.c:56-69`; with kp_brake=0 the
        // first tick scales -40A to -39.6A before RUNNING smooths by 0.2.
        assert!(
            (motor.bindings().current().current().as_amps() + 7.92).abs() < 0.0001,
            "{:?}",
            motor.bindings().current()
        );
    }

    #[test]
    fn app_data_running_accumulates_angle_i_balance_current_like_refloat_pid() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));
        assert!(
            (motor.bindings().current().current().as_amps() - 4.001).abs() < 0.0001,
            "{:?}",
            motor.bindings().current()
        );

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `pid_update` accumulates `pid->i += pid->p * config->ki`
        // and clamps it at `third_party/refloat/src/pid.c:40-46`; RUNNING adds P + I before
        // smoothing balance current at `third_party/refloat/src/main.c:932-954`.
        assert!(
            (motor.bindings().current().current().as_amps() - 7.2028).abs() < 0.0001,
            "{:?}",
            motor.bindings().current()
        );
    }

    #[test]
    fn app_data_running_clamps_angle_i_at_default_ki_limit_like_refloat_pid() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10_000.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Refloat default `ki_limit` is 30A (`settings.xml:1679-1707`);
        // `pid_update` clamps the I term at `third_party/refloat/src/pid.c:40-46` before RUNNING
        // smooths it into `balance_current` at `third_party/refloat/src/main.c:932-954`.
        assert!((motor.bindings().current().current().as_amps() - 6.0).abs() < 0.0001);
    }

    #[test]
    fn app_data_running_limits_handtest_and_flywheel_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());

        for (mode, expected_current) in [
            (RefloatMode::HandTest, 1.4_f32),
            (RefloatMode::Flywheel, 8.0_f32),
        ] {
            let payloads = sample_all_data_payloads_with_ride_state(RefloatRunState::Running, mode);
            let base = payloads.base();
            let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10.0));
            let setpoints = RefloatRealtimeRuntimeSetpoints::new(
                setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
            );
            let footpad = if matches!(mode, RefloatMode::Flywheel) {
                FootpadSensorSample::new(
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                    FootpadSensorState::None,
                )
            } else {
                base.footpad()
            };
            let base = RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                    base.attitude().roll(),
                    base.attitude().pitch(),
                ),
                base.status(),
                footpad,
                setpoints,
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                base.motor(),
            );
            let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
                base,
                payloads.mode2(),
                payloads.mode3(),
                payloads.mode4(),
            ));

            assert!(tick_refloat_state_and_handle_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::RealtimeData.id(),
                ],
            ));
            assert!(state.apply_requested_motor_current(&motor));

            // Upstream RUNNING clamps `new_current` to 7A for HANDTEST and
            // 40A for FLYWHEEL at `third_party/refloat/src/main.c:932-942`, then smooths it into
            // `balance_current` at `third_party/refloat/src/main.c:949-954`.
            assert!(
                (motor.bindings().current().current().as_amps() - expected_current).abs() < 0.0001,
                "{mode:?}: {:?}",
                motor.bindings().current()
            );
        }
    }

    #[test]
    fn app_data_running_limits_normal_current_from_motor_config_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());

        for (motor_current, expected_current) in [(1.0_f32, 0.6_f32), (-1.0_f32, -0.4_f32)] {
            let telemetry = MotorTelemetryApi::new(
                FakeMotorTelemetryBindings::new()
                    .with_runtime_motor(
                        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                        MotorCurrent::new(Current::from_amps(motor_current)),
                        BatteryCurrent::new(Current::from_amps(0.0)),
                        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
                    )
                    .with_motor_current_limits(
                        MotorCurrent::new(Current::from_amps(3.0)),
                        MotorCurrent::new(Current::from_amps(2.0)),
                    ),
            );
            let payloads = sample_all_data_payloads_with_ride_state(
                RefloatRunState::Running,
                RefloatMode::Normal,
            );
            let base = payloads.base();
            let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
                10.0 * motor_current.signum(),
            ));
            let setpoints = RefloatRealtimeRuntimeSetpoints::new(
                setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
            );
            let base = RefloatAllDataBasePayload::new(
                RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                RefloatAllDataAttitude::new(
                    RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                    base.attitude().roll(),
                    base.attitude().pitch(),
                ),
                base.status(),
                base.footpad(),
                setpoints,
                RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
                base.motor(),
            );
            let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
                base,
                payloads.mode2(),
                payloads.mode3(),
                payloads.mode4(),
            ));
            let mut config = *state.serialized_config();
            config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
                .copy_from_slice(&0u16.to_be_bytes());
            config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
                .copy_from_slice(&0u16.to_be_bytes());
            assert!(state.store_serialized_config(&config));

            assert!(tick_refloat_state_and_handle_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::RealtimeData.id(),
                ],
            ));
            assert!(state.apply_requested_motor_current(&motor));

            // Upstream `motor_data_update` caches `l_current_max` and
            // `fabsf(l_current_min)` at `third_party/refloat/src/motor_data.c:90-91`; RUNNING uses
            // max while accelerating and min while braking at `third_party/refloat/src/main.c:932-942`.
            assert!(
                (motor.bindings().current().current().as_amps() - expected_current).abs() < 0.0001,
                "{motor_current}: {:?}",
                motor.bindings().current()
            );
        }
    }

    #[test]
    fn app_data_running_adds_booster_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0));
        let zero_setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
            zero_setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET
            ..super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET
            ..super::REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET
            ..super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET + 2]
            .copy_from_slice(&2000u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream subtracts brake tilt at `third_party/refloat/src/main.c:921`,
        // `booster_update` applies full configured booster current above
        // angle+ramp at `third_party/refloat/src/booster.c:35-46`, filters it
        // at `third_party/refloat/src/booster.c:74-75`, and RUNNING adds it to
        // rate-P before smoothing `balance_current` at
        // `third_party/refloat/src/main.c:921-954`.
        assert!(
            (state
                .all_data_payloads()
                .base()
                .booster_current()
                .current()
                .current()
                .as_amps()
                - 0.2)
                .abs()
                < 0.0001
        );
        assert!((motor.bindings().current().current().as_amps() - 0.04).abs() < 0.0001);
    }

    #[test]
    fn app_data_running_adds_braking_booster_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(-2.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            ));
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(3.0_f32.to_radians())),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET
            ..super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET
            ..super::REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET + 2]
            .copy_from_slice(&100u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET
            ..super::REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET + 2]
            .copy_from_slice(&2000u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `motor_data_update` marks braking from negative motor
        // current at `third_party/refloat/src/motor_data.c:121-123`; `booster_update` then uses
        // `brkbooster_*` config at `third_party/refloat/src/booster.c:35-41`, applies sign from
        // proportional at `third_party/refloat/src/booster.c:60-64`, filters at `third_party/refloat/src/booster.c:68`,
        // and RUNNING smooths balance current at `third_party/refloat/src/main.c:921-954`.
        assert!(
            (state
                .all_data_payloads()
                .base()
                .booster_current()
                .current()
                .current()
                .as_amps()
                + 0.2)
                .abs()
                < 0.0001
        );
        assert!((motor.bindings().current().current().as_amps() + 0.04).abs() < 0.0001);
    }

    #[test]
    fn app_data_running_inverts_darkride_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING negates `new_current` for darkride at
        // `third_party/refloat/src/main.c:944-946`, before smoothing/requesting motor current.
        assert!((motor.bindings().current().current().as_amps() + 4.001).abs() < 0.0001);
    }

    #[test]
    fn app_data_running_wheelslip_without_traction_control_smooths_current_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_wheelslip(RefloatWheelSlipState::Detected);
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING only sets `balance_current = 0` when
        // `traction_control` is set at `third_party/refloat/src/main.c:949-954`; wheelslip alone
        // remains a UI/state flag and the current path still smooths.
        assert_ne!(motor.bindings().current().current().as_amps(), 0.0);
        assert_ne!(
            state
                .all_data_payloads()
                .base()
                .balance_current()
                .current()
                .current()
                .as_amps(),
            0.0
        );
    }

    #[test]
    fn app_data_running_darkride_detects_traction_loss_like_refloat_loop() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-3_000.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                MotorCurrent::new(Current::from_amps(0.0)),
                BatteryCurrent::new(Current::from_amps(0.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.5)),
            ));
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.0)),
                    ImuPitch::new(AngleRadians::from_radians(0.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let ride_state = base
            .status()
            .ride_state()
            .with_darkride(RefloatDarkRideState::Active);
        let no_footpads = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
            FootpadSensorState::None,
        );
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            setpoints,
            base.booster_current(),
            base.motor(),
        );
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        let ride_state = state.all_data_payloads().base().status().ride_state();
        // Upstream detects traction loss from acceleration, ERPM, and duty at
        // `third_party/refloat/src/main.c:551-562`, then freewheels while traction control is set at
        // `third_party/refloat/src/main.c:949-954`.
        assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::Detected);
        assert_eq!(motor.bindings().current().current().as_amps(), 0.0);
    }

    #[test]
    fn app_data_runtime_refreshes_motor_payload_like_refloat_motor_data_update() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
                MotorCurrent::new(Current::from_amps(12.25)),
                BatteryCurrent::new(Current::from_amps(4.0)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
            ));
        let imu = ImuApi::new(FakeImuBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));

        let motor = state.all_data_payloads().base().motor();
        assert_eq!(
            motor.electrical_speed().rpm().as_revolutions_per_minute(),
            1234.0
        );
        assert_eq!(motor.vehicle_speed().speed().as_meters_per_second(), 5.5);
        assert_eq!(motor.motor_current().current().as_amps(), 12.25);
        assert_eq!(motor.battery_current().current().as_amps(), 0.04);
        assert_eq!(motor.duty_cycle().ratio().as_ratio(), 0.375);
    }

    #[test]
    fn app_data_runtime_refreshes_foc_id_current_like_refloat_all_data() {
        // Refloat v1.2.1 encodes `fabsf(VESC_IF->foc_get_id()) * 3` for
        // compact all-data at `third_party/refloat/src/main.c:1364-1368`.
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_foc_id_current(Some(MotorCurrent::new(Current::from_amps(-4.0)))),
        );
        let imu = ImuApi::new(FakeImuBindings::new());
        let mut state = RefloatAppDataState::new(RefloatAllDataPayloads::source_startup());

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                0,
            ],
        ));

        let motor = state.all_data_payloads().base().motor();
        assert_eq!(
            motor
                .foc_id_current()
                .as_measured()
                .expect("measured Id current")
                .current()
                .as_amps(),
            -4.0
        );
        assert_eq!(lifecycle.bindings().last_sent_base_foc_id_byte.get(), 12);
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
    fn lifecycle_installs_refloat_state_before_callbacks_like_refloat_startup() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert!(unsafe { lifecycle.install_refloat_state(&mut info, &mut state, handler) });
        // Upstream sets `info->stop_fun` and `info->arg` at `third_party/refloat/src/main.c:2431-2432`,
        // before registering custom config/app-data/extensions at `third_party/refloat/src/main.c:2455-2459`.
        assert_eq!(lifecycle.bindings().handler_calls.get(), 0);
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
        assert!(info.stop_fun.is_some());
        assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
    }

    #[test]
    fn raw_handler_boundary_rejects_null_and_sends_valid_packets() {
        let lifecycle = RefloatAppDataLifecycle::new(RecordingAppDataBindings::accepting());
        let mut state = RefloatAppDataState::new(sample_all_data_payloads());

        assert!(!unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            let imu = ImuApi::new(FakeImuBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
                core::ptr::null_mut(),
                0,
            )
        });

        let mut request = [101, 10, 0];
        assert!(unsafe {
            let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
            let imu = ImuApi::new(FakeImuBindings::new());
            handle_refloat_app_data_packet(
                &mut state,
                &lifecycle,
                &telemetry,
                &imu,
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
        let allocated_state =
            unsafe { RefloatAppDataState::from_info_arg(&mut info) }.expect("allocated state");
        assert_eq!(
            *allocated_state,
            RefloatAppDataState::new(RefloatAllDataPayloads::source_startup()),
        );
    }

    #[test]
    fn custom_config_xml_callback_returns_upstream_settings_blob() {
        let mut buffer = core::ptr::null_mut();

        let len = unsafe { super::refloat_get_cfg_xml(&mut buffer) };

        // Refloat v1.2.1 returns generated `data_refloatconfig_` at
        // `third_party/refloat/src/main.c:2388-2396`, produced from `third_party/refloat/src/conf/settings.xml` by
        // `third_party/refloat/src/Makefile:28-31`.
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
        // applies generated defaults, and serializes it at `third_party/refloat/src/main.c:2339-2350`.
        // The generated format comes from `third_party/refloat/src/Makefile:28-31`;
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
        // package state at `third_party/refloat/src/main.c:2347-2350`; `data_init` populates it
        // from EEPROM or generated defaults at `third_party/refloat/src/main.c:1160-1185`.
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
        // `third_party/refloat/src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
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
        // write at `third_party/refloat/src/main.c:2375-2377`; generated
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
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is serialized at
        // `third_party/refloat/src/conf/settings.xml:4064`.
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
        // `third_party/refloat/src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
        assert_eq!(len, 276);
        assert_eq!(current, *include_bytes!("conf/default_config.dat"));
    }
}
