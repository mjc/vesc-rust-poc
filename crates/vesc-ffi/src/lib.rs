//! Minimal VESC ABI crate.
//!
//! This crate mirrors the firmware C ABI and keeps semantic Rust domain types
//! out of the raw boundary. It exposes raw scalar wrappers, view wrappers, and
//! firmware-facing helper APIs, but it does not define the later ergonomic
//! `vesc-types` / `vesc-units` surface.
#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

use core::ffi::{CStr, c_char, c_void};

mod types;
pub use types::*;

pub mod views;

pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ThreadEntry(pub core::ptr::NonNull<c_void>);

pub type ExtensionHandler = unsafe extern "C" fn(*mut u32, u32) -> u32;
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct LibInfo {
    pub stop_fun: Option<StopHandler>,
    pub arg: *mut c_void,
    pub base_addr: u32,
}

pub struct LibInfoAbi;

impl LibInfoAbi {
    pub const STOP_FUN_OFFSET: usize = 0;
    pub const ARG_OFFSET: usize = 4;
    pub const BASE_ADDR_OFFSET: usize = 8;
    pub const SIZE: usize = 12;
    pub const ALIGN: usize = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
}

impl VescIfSlot {
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self { name, offset }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn vesc32_byte_offset(self) -> usize {
        self.offset
    }

    pub const fn slot_index(self) -> usize {
        self.offset / 4
    }

    pub const fn host_byte_offset(self, pointer_size: usize) -> usize {
        self.slot_index() * pointer_size
    }
}

pub struct VescIfAbi;

impl VescIfAbi {
    pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
    // These offsets are pinned to the 32-bit VESC native header/table layout.
    pub const LBM_ADD_EXTENSION: VescIfSlot = VescIfSlot::new("lbm_add_extension", 0);
    pub const LBM_ENC_I: VescIfSlot = VescIfSlot::new("lbm_enc_i", 64);
    pub const LBM_DEC_AS_I32: VescIfSlot = VescIfSlot::new("lbm_dec_as_i32", 100);
    pub const LBM_IS_NUMBER: VescIfSlot = VescIfSlot::new("lbm_is_number", 124);
    pub const LBM_ENC_SYM_EERROR: VescIfSlot = VescIfSlot::new("lbm_enc_sym_eerror", 148);
    pub const SEND_APP_DATA: VescIfSlot = VescIfSlot::new("send_app_data", 592);
    pub const SET_APP_DATA_HANDLER: VescIfSlot = VescIfSlot::new("set_app_data_handler", 596);
    pub const SYSTEM_TIME_TICKS: VescIfSlot = VescIfSlot::new("system_time_ticks", 952);

    pub const USED_SLOTS: [VescIfSlot; 8] = [
        Self::LBM_ADD_EXTENSION,
        Self::LBM_ENC_I,
        Self::LBM_DEC_AS_I32,
        Self::LBM_IS_NUMBER,
        Self::LBM_ENC_SYM_EERROR,
        Self::SEND_APP_DATA,
        Self::SET_APP_DATA_HANDLER,
        Self::SYSTEM_TIME_TICKS,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageOffset(usize);

impl ImageOffset {
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeImage {
    base_addr: NativeAddress,
}

impl NativeImage {
    pub const fn new(base_addr: u32) -> Self {
        Self {
            base_addr: NativeAddress(base_addr as usize),
        }
    }

    pub fn from_info(info: &LibInfo) -> Self {
        Self::new(info.base_addr)
    }

    pub const fn base_addr(self) -> NativeAddress {
        self.base_addr
    }

    pub fn rebase_offset(self, offset: ImageOffset) -> NativeAddress {
        NativeAddress(self.base_addr.0 + offset.0)
    }

    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.rebase_offset(ImageOffset::new(image_addr)).0
    }

    pub fn rebase_ptr<T>(self, ptr: *const T) -> *const T {
        self.rebase_addr(ptr as usize) as *const T
    }
}

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

pub trait AppDataBindings {
    /// # Safety
    /// `handler` must be either `None` or a callback with the firmware app-data ABI
    /// that remains valid until it is replaced or cleared.
    unsafe fn set_app_data_handler(&self, handler: Option<AppDataHandler>) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionNameError {
    MissingExtPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
    InvalidExtensionName,
    FirmwareRejected,
}

#[derive(Clone, Copy)]
pub struct ExtensionDescriptor {
    name: &'static CStr,
    handler: ExtensionHandler,
}

impl ExtensionDescriptor {
    pub const fn new(name: &'static CStr, handler: ExtensionHandler) -> Self {
        Self { name, handler }
    }

    pub const fn name(self) -> &'static CStr {
        self.name
    }

    pub const fn handler(self) -> ExtensionHandler {
        self.handler
    }

    pub fn validate(self) -> Result<Self, ExtensionNameError> {
        if self.name.to_bytes().starts_with(b"ext-") {
            Ok(self)
        } else {
            Err(ExtensionNameError::MissingExtPrefix)
        }
    }
}

pub struct RealBindings;

impl LbmBindings for RealBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        unsafe { raw::lbm_add_extension(name, handler) }
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { raw::lbm_dec_as_i32(value) }
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { raw::lbm_enc_i(value) }
    }

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        unsafe { raw::lbm_is_number(value) }
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        unsafe { raw::lbm_enc_sym_eerror() }
    }
}

impl AppDataBindings for RealBindings {
    unsafe fn set_app_data_handler(&self, handler: Option<AppDataHandler>) -> bool {
        unsafe { raw::vesc_set_app_data_handler(handler) }
    }
}

pub struct LbmApi<B = RealBindings> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    pub fn register_extension(&self, name: &CStr, handler: ExtensionHandler) -> bool {
        unsafe { self.bindings.add_extension(name.as_ptr(), handler) }
    }

    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    pub fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { self.bindings.encode_i32(value) }
    }

    pub fn is_number(&self, value: LbmValue) -> bool {
        unsafe { self.bindings.is_number(value) }
    }

    pub fn encode_eval_error(&self) -> LbmValue {
        unsafe { self.bindings.encode_eval_error() }
    }
}

pub struct PackageLifecycle<B = RealBindings> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn register_extension(&self, descriptor: ExtensionDescriptor) -> Result<(), RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;

        if self
            .api
            .register_extension(descriptor.name(), descriptor.handler())
        {
            Ok(())
        } else {
            Err(RegisterError::FirmwareRejected)
        }
    }
}

pub struct LoopbackLifecycle<B = RealBindings> {
    bindings: B,
}

impl<B: AppDataBindings> LoopbackLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// # Safety
    ///
    /// `info` must either be null or point to live loader metadata.
    /// `stop_handler` and `app_data_handler` must remain valid for as long as
    /// the firmware may call them. The native image is built as PIC, matching
    /// refloat's VESC package model, so these callback pointers are already
    /// runtime addresses when this code executes.
    pub unsafe fn install(
        &self,
        info: *mut LibInfo,
        stop_handler: StopHandler,
        _app_data_handler: AppDataHandler,
    ) -> bool {
        if let Some(info) = unsafe { info.as_mut() } {
            info.stop_fun = Some(stop_handler);
        }

        true
    }

    pub fn clear_app_data_handler(&self) -> bool {
        unsafe { self.bindings.set_app_data_handler(None) }
    }

    pub fn register_app_data_handler(&self, handler: AppDataHandler) -> bool {
        unsafe { self.bindings.set_app_data_handler(Some(handler)) }
    }
}

#[cfg_attr(test, allow(dead_code))]
pub mod raw {
    use super::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi};
    use core::ffi::{c_char, c_int, c_uchar, c_uint, c_void};

    type LbmFlatValue = c_void;
    type LibThread = *mut c_void;
    type LibMutex = *mut c_void;
    type LibSemaphore = *mut c_void;
    type GpioPort = c_void;
    type HwType = c_int;
    type CanStatusMsg = c_void;
    type CanStatusMsg2 = c_void;
    type CanStatusMsg3 = c_void;
    type CanStatusMsg4 = c_void;
    type CanStatusMsg5 = c_void;
    type CanStatusMsg6 = c_void;
    type PacketState = c_void;
    type EepromVar = c_void;
    type GnssData = c_void;
    type AttitudeInfo = c_void;

    #[repr(C)]
    pub struct RemoteState {
        js_x: f32,
        js_y: f32,
        bt_c: bool,
        bt_z: bool,
        is_rev: bool,
        age_s: f32,
    }

    type CanRxCallback = unsafe extern "C" fn(u32, *mut u8, u8) -> bool;
    type ReplyCallback = unsafe extern "C" fn(*mut c_uchar, c_uint);
    type PwmCallback = unsafe extern "C" fn();
    type PacketSendCallback = unsafe extern "C" fn(*mut c_uchar, c_uint);
    type PacketProcessCallback = unsafe extern "C" fn(*mut c_uchar, c_uint);
    type TerminalCallback = unsafe extern "C" fn(c_int, *const *const c_char);
    type ImuReadCallback = unsafe extern "C" fn(*mut f32, *mut f32, *mut f32, f32);
    type EncoderReadCallback = unsafe extern "C" fn() -> f32;
    type EncoderFaultCallback = unsafe extern "C" fn() -> bool;
    type EncoderInfoCallback = unsafe extern "C" fn() -> *mut c_char;

    #[repr(C)]
    pub struct VescIf {
        // LBM
        lbm_add_extension: Option<unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool>,
        lbm_block_ctx_from_extension: Option<unsafe extern "C" fn()>,
        lbm_unblock_ctx: Option<unsafe extern "C" fn(u32, *mut LbmFlatValue) -> bool>,
        lbm_get_current_cid: Option<unsafe extern "C" fn() -> u32>,
        lbm_set_error_reason: Option<unsafe extern "C" fn(*mut c_char) -> c_int>,
        lbm_pause_eval_with_gc: Option<unsafe extern "C" fn(u32)>,
        lbm_continue_eval: Option<unsafe extern "C" fn()>,
        lbm_send_message: Option<unsafe extern "C" fn(u32, LbmValue) -> c_int>,
        lbm_eval_is_paused: Option<unsafe extern "C" fn() -> bool>,
        lbm_cons: Option<unsafe extern "C" fn(LbmValue, LbmValue) -> LbmValue>,
        lbm_car: Option<unsafe extern "C" fn(LbmValue) -> LbmValue>,
        lbm_cdr: Option<unsafe extern "C" fn(LbmValue) -> LbmValue>,
        lbm_list_destructive_reverse: Option<unsafe extern "C" fn(LbmValue) -> LbmValue>,
        lbm_create_byte_array: Option<unsafe extern "C" fn(*mut LbmValue, u32) -> bool>,
        lbm_add_symbol_const: Option<unsafe extern "C" fn(*mut c_char, *mut u32) -> c_int>,
        lbm_get_symbol_by_name: Option<unsafe extern "C" fn(*mut c_char, *mut u32) -> c_int>,
        lbm_enc_i: Option<unsafe extern "C" fn(i32) -> u32>,
        lbm_enc_u: Option<unsafe extern "C" fn(u32) -> LbmValue>,
        lbm_enc_char: Option<unsafe extern "C" fn(u8) -> LbmValue>,
        lbm_enc_float: Option<unsafe extern "C" fn(f32) -> LbmValue>,
        lbm_enc_u32: Option<unsafe extern "C" fn(u32) -> LbmValue>,
        lbm_enc_i32: Option<unsafe extern "C" fn(i32) -> LbmValue>,
        lbm_enc_sym: Option<unsafe extern "C" fn(u32) -> LbmValue>,
        lbm_dec_as_float: Option<unsafe extern "C" fn(LbmValue) -> f32>,
        lbm_dec_as_u32: Option<unsafe extern "C" fn(LbmValue) -> u32>,
        lbm_dec_as_i32: Option<unsafe extern "C" fn(u32) -> i32>,
        lbm_dec_char: Option<unsafe extern "C" fn(LbmValue) -> u8>,
        lbm_dec_str: Option<unsafe extern "C" fn(LbmValue) -> *mut c_char>,
        lbm_dec_sym: Option<unsafe extern "C" fn(LbmValue) -> u32>,
        lbm_is_byte_array: Option<unsafe extern "C" fn(LbmValue) -> bool>,
        lbm_is_cons: Option<unsafe extern "C" fn(LbmValue) -> bool>,
        lbm_is_number: Option<unsafe extern "C" fn(u32) -> bool>,
        lbm_is_char: Option<unsafe extern "C" fn(LbmValue) -> bool>,
        lbm_is_symbol: Option<unsafe extern "C" fn(LbmValue) -> bool>,
        lbm_enc_sym_nil: usize,
        lbm_enc_sym_true: usize,
        lbm_enc_sym_terror: usize,
        lbm_enc_sym_eerror: usize,
        lbm_enc_sym_merror: usize,
        lbm_is_symbol_nil: Option<unsafe extern "C" fn(u32) -> bool>,
        lbm_is_symbol_true: Option<unsafe extern "C" fn(u32) -> bool>,

        // OS
        sleep_ms: Option<unsafe extern "C" fn(u32)>,
        sleep_us: Option<unsafe extern "C" fn(u32)>,
        system_time: Option<unsafe extern "C" fn() -> f32>,
        ts_to_age_s: Option<unsafe extern "C" fn(u32) -> f32>,
        printf: Option<unsafe extern "C" fn(*const c_char, ...) -> c_int>,
        malloc: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        free: Option<unsafe extern "C" fn(*mut c_void)>,
        spawn: Option<
            unsafe extern "C" fn(
                unsafe extern "C" fn(*mut c_void),
                usize,
                *const c_char,
                *mut c_void,
            ) -> LibThread,
        >,
        request_terminate: Option<unsafe extern "C" fn(LibThread)>,
        should_terminate: Option<unsafe extern "C" fn() -> bool>,
        get_arg: Option<unsafe extern "C" fn(u32) -> *mut *mut c_void>,

        // ST IO
        set_pad_mode: Option<unsafe extern "C" fn(*mut GpioPort, u32, u32)>,
        set_pad: Option<unsafe extern "C" fn(*mut GpioPort, u32)>,
        clear_pad: Option<unsafe extern "C" fn(*mut GpioPort, u32)>,

        // Abstract IO
        io_set_mode: Option<unsafe extern "C" fn(c_int, c_int) -> bool>,
        io_write: Option<unsafe extern "C" fn(c_int, c_int) -> bool>,
        io_read: Option<unsafe extern "C" fn(c_int) -> bool>,
        io_read_analog: Option<unsafe extern "C" fn(c_int) -> f32>,
        io_get_st_pin: Option<unsafe extern "C" fn(c_int, *mut *mut GpioPort, *mut u32) -> bool>,

        // CAN
        can_set_sid_cb: Option<unsafe extern "C" fn(Option<CanRxCallback>)>,
        can_set_eid_cb: Option<unsafe extern "C" fn(Option<CanRxCallback>)>,
        can_transmit_sid: Option<unsafe extern "C" fn(u32, *const u8, u8)>,
        can_transmit_eid: Option<unsafe extern "C" fn(u32, *const u8, u8)>,
        can_send_buffer: Option<unsafe extern "C" fn(u8, *mut u8, c_uint, u8)>,
        can_set_duty: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_current: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_current_off_delay: Option<unsafe extern "C" fn(u8, f32, f32)>,
        can_set_current_brake: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_rpm: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_pos: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_current_rel: Option<unsafe extern "C" fn(u8, f32)>,
        can_set_current_rel_off_delay: Option<unsafe extern "C" fn(u8, f32, f32)>,
        can_set_current_brake_rel: Option<unsafe extern "C" fn(u8, f32)>,
        can_ping: Option<unsafe extern "C" fn(u8, *mut HwType) -> bool>,
        can_get_status_msg_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg>,
        can_get_status_msg_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg>,
        can_get_status_msg_2_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg2>,
        can_get_status_msg_2_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg2>,
        can_get_status_msg_3_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg3>,
        can_get_status_msg_3_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg3>,
        can_get_status_msg_4_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg4>,
        can_get_status_msg_4_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg4>,
        can_get_status_msg_5_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg5>,
        can_get_status_msg_5_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg5>,
        can_get_status_msg_6_index: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg6>,
        can_get_status_msg_6_id: Option<unsafe extern "C" fn(c_int) -> *mut CanStatusMsg6>,

        // Motor Control
        mc_motor_now: Option<unsafe extern "C" fn() -> c_int>,
        mc_select_motor_thread: Option<unsafe extern "C" fn(c_int)>,
        mc_get_motor_thread: Option<unsafe extern "C" fn() -> c_int>,
        mc_dccal_done: Option<unsafe extern "C" fn() -> bool>,
        mc_set_pwm_callback: Option<unsafe extern "C" fn(Option<PwmCallback>)>,
        mc_get_fault: Option<unsafe extern "C" fn() -> c_int>,
        mc_fault_to_string: Option<unsafe extern "C" fn(c_int) -> *const c_char>,
        mc_set_duty: Option<unsafe extern "C" fn(f32)>,
        mc_set_duty_noramp: Option<unsafe extern "C" fn(f32)>,
        mc_set_pid_speed: Option<unsafe extern "C" fn(f32)>,
        mc_set_pid_pos: Option<unsafe extern "C" fn(f32)>,
        mc_set_current: Option<unsafe extern "C" fn(f32)>,
        mc_set_brake_current: Option<unsafe extern "C" fn(f32)>,
        mc_set_current_rel: Option<unsafe extern "C" fn(f32)>,
        mc_set_brake_current_rel: Option<unsafe extern "C" fn(f32)>,
        mc_set_handbrake: Option<unsafe extern "C" fn(f32)>,
        mc_set_handbrake_rel: Option<unsafe extern "C" fn(f32)>,
        mc_set_tachometer_value: Option<unsafe extern "C" fn(c_int) -> c_int>,
        mc_release_motor: Option<unsafe extern "C" fn()>,
        mc_wait_for_motor_release: Option<unsafe extern "C" fn(f32) -> bool>,
        mc_get_duty_cycle_now: Option<unsafe extern "C" fn() -> f32>,
        mc_get_sampling_frequency_now: Option<unsafe extern "C" fn() -> f32>,
        mc_get_rpm: Option<unsafe extern "C" fn() -> f32>,
        mc_get_amp_hours: Option<unsafe extern "C" fn(bool) -> f32>,
        mc_get_amp_hours_charged: Option<unsafe extern "C" fn(bool) -> f32>,
        mc_get_watt_hours: Option<unsafe extern "C" fn(bool) -> f32>,
        mc_get_watt_hours_charged: Option<unsafe extern "C" fn(bool) -> f32>,
        mc_get_tot_current: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tot_current_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tot_current_directional: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tot_current_directional_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tot_current_in: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tot_current_in_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_get_input_voltage_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_get_tachometer_value: Option<unsafe extern "C" fn(bool) -> c_int>,
        mc_get_tachometer_abs_value: Option<unsafe extern "C" fn(bool) -> c_int>,
        mc_get_pid_pos_set: Option<unsafe extern "C" fn() -> f32>,
        mc_get_pid_pos_now: Option<unsafe extern "C" fn() -> f32>,
        mc_update_pid_pos_offset: Option<unsafe extern "C" fn(f32, bool)>,
        mc_temp_fet_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_temp_motor_filtered: Option<unsafe extern "C" fn() -> f32>,
        mc_get_battery_level: Option<unsafe extern "C" fn(*mut f32) -> f32>,
        mc_get_speed: Option<unsafe extern "C" fn() -> f32>,
        mc_get_distance: Option<unsafe extern "C" fn() -> f32>,
        mc_get_distance_abs: Option<unsafe extern "C" fn() -> f32>,
        mc_get_odometer: Option<unsafe extern "C" fn() -> u64>,
        mc_set_odometer: Option<unsafe extern "C" fn(u64)>,
        mc_set_current_off_delay: Option<unsafe extern "C" fn(f32)>,
        mc_stat_speed_avg: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_speed_max: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_power_avg: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_power_max: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_current_avg: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_current_max: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_temp_mosfet_avg: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_temp_mosfet_max: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_temp_motor_avg: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_temp_motor_max: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_count_time: Option<unsafe extern "C" fn() -> f32>,
        mc_stat_reset: Option<unsafe extern "C" fn()>,

        // Comm
        commands_process_packet: Option<unsafe extern "C" fn(*mut c_uchar, c_uint, ReplyCallback)>,
        send_app_data: Option<unsafe extern "C" fn(*mut c_uchar, u32)>,
        set_app_data_handler: Option<unsafe extern "C" fn(Option<AppDataHandler>) -> bool>,

        // UART
        uart_start: Option<unsafe extern "C" fn(u32, bool) -> bool>,
        uart_write: Option<unsafe extern "C" fn(*const u8, u32) -> bool>,
        uart_read: Option<unsafe extern "C" fn() -> i32>,

        // Packets
        packet_init: Option<
            unsafe extern "C" fn(PacketSendCallback, PacketProcessCallback, *mut PacketState),
        >,
        packet_reset: Option<unsafe extern "C" fn(*mut PacketState)>,
        packet_process_byte: Option<unsafe extern "C" fn(u8, *mut PacketState)>,
        packet_send_packet: Option<unsafe extern "C" fn(*mut c_uchar, c_uint, *mut PacketState)>,

        // IMU
        imu_startup_done: Option<unsafe extern "C" fn() -> bool>,
        imu_get_roll: Option<unsafe extern "C" fn() -> f32>,
        imu_get_pitch: Option<unsafe extern "C" fn() -> f32>,
        imu_get_yaw: Option<unsafe extern "C" fn() -> f32>,
        imu_get_rpy: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_accel: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_gyro: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_mag: Option<unsafe extern "C" fn(*mut f32)>,
        imu_derotate: Option<unsafe extern "C" fn(*const f32, *mut f32)>,
        imu_get_accel_derotated: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_gyro_derotated: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_quaternions: Option<unsafe extern "C" fn(*mut f32)>,
        imu_get_calibration: Option<unsafe extern "C" fn(f32, *mut f32)>,
        imu_set_yaw: Option<unsafe extern "C" fn(f32)>,

        // Terminal
        terminal_register_command_callback: Option<
            unsafe extern "C" fn(*const c_char, *const c_char, *const c_char, TerminalCallback),
        >,
        terminal_unregister_callback: Option<unsafe extern "C" fn(TerminalCallback)>,

        // EEPROM
        read_eeprom_var: Option<unsafe extern "C" fn(*mut EepromVar, c_int) -> bool>,
        store_eeprom_var: Option<unsafe extern "C" fn(*mut EepromVar, c_int) -> bool>,

        // Timeout
        timeout_reset: Option<unsafe extern "C" fn()>,
        timeout_has_timeout: Option<unsafe extern "C" fn() -> bool>,
        timeout_secs_since_update: Option<unsafe extern "C" fn() -> f32>,

        // Plot
        plot_init: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
        plot_add_graph: Option<unsafe extern "C" fn(*const c_char)>,
        plot_set_graph: Option<unsafe extern "C" fn(c_int)>,
        plot_send_points: Option<unsafe extern "C" fn(f32, f32)>,

        // Custom config
        conf_custom_add_config: Option<
            unsafe extern "C" fn(
                unsafe extern "C" fn(*mut u8, bool) -> c_int,
                unsafe extern "C" fn(*mut u8) -> bool,
                unsafe extern "C" fn(*mut *mut u8) -> c_int,
            ),
        >,
        conf_custom_clear_configs: Option<unsafe extern "C" fn()>,

        // Settings
        get_cfg_float: Option<unsafe extern "C" fn(c_int) -> f32>,
        get_cfg_int: Option<unsafe extern "C" fn(c_int) -> c_int>,
        set_cfg_float: Option<unsafe extern "C" fn(c_int, f32) -> bool>,
        set_cfg_int: Option<unsafe extern "C" fn(c_int, c_int) -> bool>,
        store_cfg: Option<unsafe extern "C" fn() -> bool>,

        // GNSS
        mc_gnss: Option<unsafe extern "C" fn() -> *mut GnssData>,

        // Mutex
        mutex_create: Option<unsafe extern "C" fn() -> LibMutex>,
        mutex_lock: Option<unsafe extern "C" fn(LibMutex)>,
        mutex_unlock: Option<unsafe extern "C" fn(LibMutex)>,

        // Get ST io-pin from lbm symbol
        lbm_symbol_to_io: Option<unsafe extern "C" fn(u32, *mut *mut GpioPort, *mut u32) -> bool>,

        // High resolution timer
        timer_time_now: Option<unsafe extern "C" fn() -> u32>,
        timer_seconds_elapsed_since: Option<unsafe extern "C" fn(u32) -> f32>,
        timer_sleep: Option<unsafe extern "C" fn(f32)>,

        // System lock
        sys_lock: Option<unsafe extern "C" fn()>,
        sys_unlock: Option<unsafe extern "C" fn()>,

        // Comm reply cleanup
        commands_unregister_reply_func: Option<unsafe extern "C" fn(ReplyCallback)>,

        // IMU AHRS functions and read callback
        imu_set_read_callback: Option<unsafe extern "C" fn(Option<ImuReadCallback>)>,
        ahrs_init_attitude_info: Option<unsafe extern "C" fn(*mut AttitudeInfo)>,
        ahrs_update_initial_orientation:
            Option<unsafe extern "C" fn(*const f32, *const f32, *mut AttitudeInfo)>,
        ahrs_update_mahony_imu:
            Option<unsafe extern "C" fn(*const f32, *const f32, f32, *mut AttitudeInfo)>,
        ahrs_update_madgwick_imu:
            Option<unsafe extern "C" fn(*const f32, *const f32, f32, *mut AttitudeInfo)>,
        ahrs_get_roll: Option<unsafe extern "C" fn(*const AttitudeInfo) -> f32>,
        ahrs_get_pitch: Option<unsafe extern "C" fn(*const AttitudeInfo) -> f32>,
        ahrs_get_yaw: Option<unsafe extern "C" fn(*const AttitudeInfo) -> f32>,

        // Encoder callbacks
        encoder_set_custom_callbacks: Option<
            unsafe extern "C" fn(EncoderReadCallback, EncoderFaultCallback, EncoderInfoCallback),
        >,

        // Store backup data
        store_backup_data: Option<unsafe extern "C" fn() -> bool>,

        // Input Devices
        get_remote_state: Option<unsafe extern "C" fn() -> RemoteState>,
        get_ppm: Option<unsafe extern "C" fn() -> f32>,
        get_ppm_age: Option<unsafe extern "C" fn() -> f32>,
        app_is_output_disabled: Option<unsafe extern "C" fn() -> bool>,

        // Firmware 6.2 NVM
        read_nvm: Option<unsafe extern "C" fn(*mut u8, c_uint, c_uint) -> bool>,
        write_nvm: Option<unsafe extern "C" fn(*mut u8, c_uint, c_uint) -> bool>,
        wipe_nvm: Option<unsafe extern "C" fn() -> bool>,

        // Firmware 6.2 FOC
        foc_get_id: Option<unsafe extern "C" fn() -> f32>,
        foc_get_iq: Option<unsafe extern "C" fn() -> f32>,
        foc_get_vd: Option<unsafe extern "C" fn() -> f32>,
        foc_get_vq: Option<unsafe extern "C" fn() -> f32>,
        foc_set_openloop_current: Option<unsafe extern "C" fn(f32, f32)>,
        foc_set_openloop_phase: Option<unsafe extern "C" fn(f32, f32)>,
        foc_set_openloop_duty: Option<unsafe extern "C" fn(f32, f32)>,
        foc_set_openloop_duty_phase: Option<unsafe extern "C" fn(f32, f32)>,

        // Firmware 6.05 flat values
        lbm_start_flatten: Option<unsafe extern "C" fn(*mut LbmFlatValue, usize) -> bool>,
        lbm_finish_flatten: Option<unsafe extern "C" fn(*mut LbmFlatValue) -> bool>,
        f_cons: Option<unsafe extern "C" fn(*mut LbmFlatValue) -> bool>,
        f_sym: Option<unsafe extern "C" fn(*mut LbmFlatValue, u32) -> bool>,
        f_i: Option<unsafe extern "C" fn(*mut LbmFlatValue, i32) -> bool>,
        f_b: Option<unsafe extern "C" fn(*mut LbmFlatValue, u8) -> bool>,
        f_i32: Option<unsafe extern "C" fn(*mut LbmFlatValue, i32) -> bool>,
        f_u32: Option<unsafe extern "C" fn(*mut LbmFlatValue, u32) -> bool>,
        f_float: Option<unsafe extern "C" fn(*mut LbmFlatValue, f32) -> bool>,
        f_i64: Option<unsafe extern "C" fn(*mut LbmFlatValue, i64) -> bool>,
        f_u64: Option<unsafe extern "C" fn(*mut LbmFlatValue, u64) -> bool>,
        f_lbm_array: Option<unsafe extern "C" fn(*mut LbmFlatValue, u32, *mut u8) -> bool>,

        // Unblock unboxed
        lbm_unblock_ctx_unboxed: Option<unsafe extern "C" fn(u32, LbmValue) -> bool>,

        // Time since boot in system ticks
        system_time_ticks: Option<unsafe extern "C" fn() -> u32>,
        sleep_ticks: Option<unsafe extern "C" fn(u32)>,

        // FOC Audio
        foc_beep: Option<unsafe extern "C" fn(f32, f32, f32) -> bool>,
        foc_play_tone: Option<unsafe extern "C" fn(c_int, f32, f32) -> bool>,
        foc_stop_audio: Option<unsafe extern "C" fn(bool)>,
        foc_set_audio_sample_table: Option<unsafe extern "C" fn(c_int, *const f32, c_int) -> bool>,
        foc_get_audio_sample_table: Option<unsafe extern "C" fn(c_int) -> *const f32>,
        foc_play_audio_samples: Option<unsafe extern "C" fn(*const i8, c_int, f32, f32) -> bool>,

        // Semaphores
        sem_create: Option<unsafe extern "C" fn() -> LibSemaphore>,
        sem_wait: Option<unsafe extern "C" fn(LibSemaphore)>,
        sem_signal: Option<unsafe extern "C" fn(LibSemaphore)>,
        sem_wait_to: Option<unsafe extern "C" fn(LibSemaphore, u32) -> bool>,
        sem_reset: Option<unsafe extern "C" fn(LibSemaphore)>,

        // Firmware 6.06. Keep this table pinned to Refloat's
        // `vesc_pkg_lib/vesc_c_if.h`; add newer firmware slots only after
        // intentionally updating that baseline.
        thread_set_priority: Option<unsafe extern "C" fn(c_int)>,
        shutdown_disable: Option<unsafe extern "C" fn(bool)>,
    }

    const VESC_IF: *const VescIf = VescIfAbi::BASE_ADDR.0 as *const VescIf;

    /// # Safety
    ///
    /// `name` must point to a valid, NUL-terminated extension name and
    /// `handler` must use the firmware LispBM extension ABI.
    pub unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> bool {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            return lbm_add_extension_with_table_base(VescIfAbi::BASE_ADDR.0 as u32, name, handler);
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            match (*VESC_IF).lbm_add_extension {
                Some(lbm_add_extension) => lbm_add_extension(name as *mut c_char, handler),
                None => false,
            }
        }
    }

    /// # Safety
    ///
    /// `vesc_if_base` must be the firmware VESC function table address and
    /// `name`/`handler` must satisfy the same requirements as
    /// [`lbm_add_extension`].
    #[inline(always)]
    pub unsafe fn lbm_add_extension_with_table_base(
        vesc_if_base: u32,
        name: *const c_char,
        handler: ExtensionHandler,
    ) -> bool {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = vesc_if_base as usize;
            let lbm_add_extension: usize;
            core::arch::asm!(
                "ldr {lbm_add_extension}, [{vesc_if}, #0]",
                vesc_if = in(reg) vesc_if,
                lbm_add_extension = out(reg) lbm_add_extension,
                options(nostack, preserves_flags),
            );
            let lbm_add_extension: unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool =
                core::mem::transmute(lbm_add_extension);
            lbm_add_extension(name as *mut c_char, handler)
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        {
            let _ = (vesc_if_base, name, handler);
            false
        }
    }

    /// # Safety
    ///
    /// `value` must be a LispBM value supplied by the firmware.
    pub unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = VescIfAbi::BASE_ADDR.0;
            let lbm_dec_as_i32: usize;
            core::arch::asm!(
                "ldr {lbm_dec_as_i32}, [{vesc_if}, #100]",
                vesc_if = in(reg) vesc_if,
                lbm_dec_as_i32 = out(reg) lbm_dec_as_i32,
                options(nostack, preserves_flags),
            );
            let lbm_dec_as_i32: unsafe extern "C" fn(u32) -> i32 =
                core::mem::transmute(lbm_dec_as_i32);
            return lbm_dec_as_i32(value.0);
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            match (*VESC_IF).lbm_dec_as_i32 {
                Some(lbm_dec_as_i32) => lbm_dec_as_i32(value.0),
                None => 0,
            }
        }
    }

    /// # Safety
    ///
    /// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
    pub unsafe fn lbm_enc_i(value: i32) -> LbmValue {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = VescIfAbi::BASE_ADDR.0;
            let lbm_enc_i: usize;
            core::arch::asm!(
                "ldr {lbm_enc_i}, [{vesc_if}, #64]",
                vesc_if = in(reg) vesc_if,
                lbm_enc_i = out(reg) lbm_enc_i,
                options(nostack, preserves_flags),
            );
            let lbm_enc_i: unsafe extern "C" fn(i32) -> u32 = core::mem::transmute(lbm_enc_i);
            return LbmValue(lbm_enc_i(value));
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            match (*VESC_IF).lbm_enc_i {
                Some(lbm_enc_i) => LbmValue(lbm_enc_i(value)),
                None => LbmValue(0),
            }
        }
    }

    /// # Safety
    ///
    /// `value` must be a LispBM value supplied by the firmware.
    pub unsafe fn lbm_is_number(value: LbmValue) -> bool {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = VescIfAbi::BASE_ADDR.0;
            let lbm_is_number: usize;
            core::arch::asm!(
                "ldr {lbm_is_number}, [{vesc_if}, #124]",
                vesc_if = in(reg) vesc_if,
                lbm_is_number = out(reg) lbm_is_number,
                options(nostack, preserves_flags),
            );
            let lbm_is_number: unsafe extern "C" fn(u32) -> bool =
                core::mem::transmute(lbm_is_number);
            return lbm_is_number(value.0);
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            match (*VESC_IF).lbm_is_number {
                Some(lbm_is_number) => lbm_is_number(value.0),
                None => false,
            }
        }
    }

    /// # Safety
    ///
    /// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
    pub unsafe fn lbm_enc_sym_eerror() -> LbmValue {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = VescIfAbi::BASE_ADDR.0;
            let lbm_enc_sym_eerror: usize;
            core::arch::asm!(
                "ldr {lbm_enc_sym_eerror}, [{vesc_if}, #148]",
                vesc_if = in(reg) vesc_if,
                lbm_enc_sym_eerror = out(reg) lbm_enc_sym_eerror,
                options(nostack, preserves_flags),
            );
            return LbmValue(lbm_enc_sym_eerror as u32);
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            LbmValue((*VESC_IF).lbm_enc_sym_eerror as u32)
        }
    }

    /// # Safety
    ///
    /// `handler` must either be `None` or remain valid until replaced or
    /// cleared by a later firmware call.
    pub unsafe fn vesc_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
        #[cfg(all(target_arch = "arm", not(test)))]
        unsafe {
            let vesc_if = VescIfAbi::BASE_ADDR.0;
            let set_app_data_handler: usize;
            core::arch::asm!(
                "ldr {set_app_data_handler}, [{vesc_if}, #596]",
                vesc_if = in(reg) vesc_if,
                set_app_data_handler = out(reg) set_app_data_handler,
                options(nostack, preserves_flags),
            );
            let set_app_data_handler: unsafe extern "C" fn(Option<AppDataHandler>) -> bool =
                core::mem::transmute(set_app_data_handler);
            return set_app_data_handler(handler);
        }

        #[cfg(not(all(target_arch = "arm", not(test))))]
        unsafe {
            let Some(set_app_data_handler) = (*VESC_IF).set_app_data_handler else {
                return false;
            };

            set_app_data_handler(handler)
        }
    }

    /// # Safety
    ///
    /// `data` must point to at least `len` bytes that remain valid for the
    /// duration of the firmware call.
    pub unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
        unsafe {
            if let Some(send_app_data) = (*VESC_IF).send_app_data {
                send_app_data(data as *mut c_uchar, len);
            }
        }
    }

    /// # Safety
    ///
    /// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
    pub unsafe fn vesc_system_time_ticks() -> u32 {
        unsafe {
            match (*VESC_IF).system_time_ticks {
                Some(system_time_ticks) => system_time_ticks(),
                None => 0,
            }
        }
    }

    #[cfg(test)]
    pub fn vesc_if_offsets_for_tests() -> [usize; 8] {
        [
            core::mem::offset_of!(VescIf, lbm_add_extension),
            core::mem::offset_of!(VescIf, lbm_enc_i),
            core::mem::offset_of!(VescIf, lbm_dec_as_i32),
            core::mem::offset_of!(VescIf, lbm_is_number),
            core::mem::offset_of!(VescIf, lbm_enc_sym_eerror),
            core::mem::offset_of!(VescIf, send_app_data),
            core::mem::offset_of!(VescIf, set_app_data_handler),
            core::mem::offset_of!(VescIf, system_time_ticks),
        ]
    }

    #[cfg(test)]
    pub fn vesc_if_full_layout_for_tests() -> (usize, usize, usize) {
        (
            core::mem::size_of::<VescIf>(),
            core::mem::align_of::<VescIf>(),
            core::mem::offset_of!(VescIf, shutdown_disable),
        )
    }

    #[cfg(test)]
    pub fn nullable_slot_layout_for_tests() -> (usize, usize) {
        (
            core::mem::size_of::<Option<unsafe extern "C" fn()>>(),
            core::mem::align_of::<Option<unsafe extern "C" fn()>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::{
        AppDataLen, AppDataPacket, CanControllerId, CanFrameLen, CanPayload, CanStatusIndex,
        CfgFloat, CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult, ConfigXmlBytes,
        EepromAddress, EepromVar, ExtensionDescriptor, ExtensionHandler, FirmwareNonNull,
        FirmwarePtr, GpioPin, GpioPortPtr, HalfDuplex, HardwareType, ImageOffset, LbmApi,
        LbmBindings, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt,
        LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo, LibInfoAbi,
        LoaderBaseAddress, MallocLen, MotorIndex, MutablePacket, MutexHandle, NativeAddress,
        NativeImage, NvmAddress, NvmBytes, NvmLen, OwnedFirmwareAllocation, PackageLifecycle,
        PlotAxisName, PlotGraphIndex, PlotGraphName, PlotPoint, ProgramAddress, RegisterError,
        ReplyPacket, SemaphoreHandle, StackSizeBytes, SystemTicks, ThreadHandle, ThreadName,
        UartBaudRate, UartWriteLen, VescIfAbi, VescPin, VescPinMode,
    };
    use core::cell::Cell;
    use core::ffi::{CStr, c_char};

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        encode_calls: Cell<usize>,
        last_name: Cell<usize>,
        last_handler: Cell<usize>,
        add_results: Cell<[bool; 2]>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self::with_add_results([true, true])
        }

        fn with_add_results(add_results: [bool; 2]) -> Self {
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
    }

    unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
        0
    }

    #[test]
    fn wrapper_delegates_through_the_binding_trait() {
        let bindings = FakeBindings::new();
        let api = LbmApi::new(bindings);
        let name = c"ext-rust-add";

        assert!(api.register_extension(name, stub_handler));
        assert_eq!(api.decode_i32(LbmValue(3)), 3);
        assert_eq!(api.encode_i32(9), LbmValue(9));
        assert!(api.is_number(LbmValue(9)));
        assert_eq!(api.encode_eval_error(), LbmValue(0xffff_ffff));
    }

    #[test]
    fn native_image_rebases_image_data_offsets() {
        let image = NativeImage::new(0x2000);

        assert_eq!(image.rebase_addr(0x61), 0x2061);
        assert_eq!(image.base_addr(), NativeAddress(0x2000));
        assert_eq!(
            image.rebase_offset(ImageOffset::new(0x61)),
            NativeAddress(0x2061)
        );
        assert_eq!(image.rebase_ptr(0x1df as *const c_char) as usize, 0x21df);
    }

    #[test]
    fn package_registration_reports_name_validation_and_firmware_rejection() {
        let bindings = FakeBindings::with_add_results([false, true]);
        let lifecycle = PackageLifecycle::new(bindings);

        let invalid = ExtensionDescriptor::new(c"bad-name", stub_handler);
        assert_eq!(
            lifecycle.register_extension(invalid),
            Err(RegisterError::InvalidExtensionName)
        );

        let rejected = ExtensionDescriptor::new(c"ext-rust-reject", stub_handler);
        assert_eq!(
            lifecycle.register_extension(rejected),
            Err(RegisterError::FirmwareRejected)
        );
    }

    #[test]
    fn repeated_package_registration_reports_each_firmware_result() {
        let bindings = FakeBindings::with_add_results([false, true]);
        let lifecycle = PackageLifecycle::new(bindings);

        let first = ExtensionDescriptor::new(c"ext-rust-a", stub_handler);
        let second = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);
        assert_eq!(
            lifecycle.register_extension(first),
            Err(RegisterError::FirmwareRejected)
        );
        assert_eq!(lifecycle.api.bindings.add_calls.get(), 1);
        assert_eq!(lifecycle.register_extension(second), Ok(()));
        assert_eq!(lifecycle.api.bindings.add_calls.get(), 2);
    }

    #[test]
    fn lib_info_abi_constants_match_the_vesc_native_loader_layout() {
        assert_eq!(LibInfoAbi::STOP_FUN_OFFSET, 0);
        assert_eq!(LibInfoAbi::ARG_OFFSET, 4);
        assert_eq!(LibInfoAbi::BASE_ADDR_OFFSET, 8);
        assert_eq!(LibInfoAbi::SIZE, 12);
        assert_eq!(LibInfoAbi::ALIGN, 4);
    }

    #[test]
    fn lib_info_repr_c_layout_scales_with_the_compilation_pointer_width() {
        let pointer_size = core::mem::size_of::<usize>();

        assert_eq!(core::mem::size_of::<LibInfo>(), pointer_size * 3);
        assert_eq!(core::mem::align_of::<LibInfo>(), pointer_size);
        assert_eq!(core::mem::offset_of!(LibInfo, stop_fun), 0);
        assert_eq!(core::mem::offset_of!(LibInfo, arg), pointer_size);
        assert_eq!(core::mem::offset_of!(LibInfo, base_addr), pointer_size * 2);
    }

    #[test]
    fn raw_vesc_if_offsets_match_the_documented_32_bit_package_header_slots() {
        let expected =
            VescIfAbi::USED_SLOTS.map(|slot| slot.host_byte_offset(core::mem::size_of::<usize>()));

        assert_eq!(super::raw::vesc_if_offsets_for_tests(), expected);
    }

    #[test]
    fn raw_vesc_if_table_covers_the_current_vesc_firmware_header() {
        let pointer_size = core::mem::size_of::<usize>();

        assert_eq!(
            super::raw::vesc_if_full_layout_for_tests(),
            (253 * pointer_size, pointer_size, 252 * pointer_size)
        );
    }

    #[test]
    fn raw_vesc_if_callable_slots_are_nullable_c_function_pointers() {
        let pointer_size = core::mem::size_of::<usize>();

        assert_eq!(
            super::raw::nullable_slot_layout_for_tests(),
            (pointer_size, pointer_size)
        );
    }

    #[test]
    fn vesc_if_slot_constants_name_the_package_header_offsets() {
        let slots = VescIfAbi::USED_SLOTS;

        assert_eq!(VescIfAbi::BASE_ADDR, NativeAddress(0x1000_f800));
        assert_eq!(
            slots.map(|slot| slot.name()),
            [
                "lbm_add_extension",
                "lbm_enc_i",
                "lbm_dec_as_i32",
                "lbm_is_number",
                "lbm_enc_sym_eerror",
                "send_app_data",
                "set_app_data_handler",
                "system_time_ticks",
            ]
        );
        assert_eq!(
            slots.map(|slot| slot.vesc32_byte_offset()),
            [0, 64, 100, 124, 148, 592, 596, 952]
        );
        assert_eq!(
            slots.map(|slot| slot.slot_index()),
            [0, 16, 25, 31, 37, 148, 149, 238]
        );
    }

    #[test]
    fn newtypes_wrap_the_expected_scalar_shapes() {
        assert_eq!(core::mem::size_of::<LbmInt>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<LbmUint>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmType>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmCid>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<LbmFloat>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmErrorSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmBoolSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmNilSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<ProgramAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LoaderBaseAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<AppDataLen>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<UartBaudRate>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<UartWriteLen>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<MotorIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<CanControllerId>(),
            core::mem::size_of::<u8>()
        );
        assert_eq!(
            core::mem::size_of::<CanFrameLen>(),
            core::mem::size_of::<u8>()
        );
        assert_eq!(
            core::mem::size_of::<AppDataPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<MutablePacket<'_>>(),
            core::mem::size_of::<&mut [u8]>()
        );
        assert_eq!(
            core::mem::size_of::<CommandPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ReplyPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<HalfDuplex>(),
            core::mem::size_of::<bool>()
        );
        assert_eq!(
            core::mem::size_of::<SystemTicks>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<CfgParam>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<CfgFloat>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<CfgInt>(), core::mem::size_of::<i32>());
        assert_eq!(
            core::mem::size_of::<ConfigSetResult>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<ConfigXmlBytes<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ConfigPayload<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ThreadName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<StackSizeBytes>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<ThreadHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<MutexHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<SemaphoreHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<FirmwarePtr::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<FirmwareNonNull::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<MallocLen>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<OwnedFirmwareAllocation::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<CanPayload<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<CanStatusIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<HardwareType>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<PlotAxisName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<PlotGraphName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<PlotGraphIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<PlotPoint>(),
            core::mem::size_of::<f32>() * 2
        );
        assert_eq!(core::mem::size_of::<VescPin>(), core::mem::size_of::<i32>());
        assert_eq!(
            core::mem::size_of::<VescPinMode>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<GpioPortPtr>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(core::mem::size_of::<GpioPin>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<LbmIoSymbol>(),
            core::mem::size_of::<LbmSymbol>()
        );
        assert_eq!(
            core::mem::size_of::<NvmAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(core::mem::size_of::<NvmLen>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<NvmBytes<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<EepromAddress>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<EepromVar>(),
            core::mem::size_of::<i32>()
        );
    }

    #[test]
    fn transparent_wrappers_expose_raw_tuple_fields() {
        let raw = [1_u8, 2, 3];
        let mut mut_raw = [4_u8, 5, 6];
        let name = c"axis";

        assert_eq!(LbmInt(-7).0, -7);
        assert_eq!(LbmFloat(3.5).0, 3.5);
        assert!(HalfDuplex(true).0);
        assert_eq!(ConfigXmlBytes(&raw).0, &raw);
        assert_eq!(ConfigPayload(&raw).0, &raw);
        assert_eq!(ThreadName(name).0, name);
        assert_eq!(CanPayload(&raw).0, &raw);
        assert_eq!(PlotAxisName(name).0, name);
        assert_eq!(PlotGraphName(name).0, name);
        assert_eq!(NvmBytes(&raw).0, &raw);
        {
            let packet = MutablePacket(&mut mut_raw);
            packet.0[0] = 9;
        }
        assert_eq!(mut_raw[0], 9);
        let point = PlotPoint { x: 1.5, y: 2.5 };
        assert_eq!(point.x, 1.5);
        assert_eq!(point.y, 2.5);
    }
}
