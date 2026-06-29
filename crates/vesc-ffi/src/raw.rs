#![cfg_attr(test, allow(dead_code))]

use crate::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi};
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
    set_app_data_handler: Option<unsafe extern "C" fn(AppDataHandler) -> bool>,

    // UART
    uart_start: Option<unsafe extern "C" fn(u32, bool) -> bool>,
    uart_write: Option<unsafe extern "C" fn(*const u8, u32) -> bool>,
    uart_read: Option<unsafe extern "C" fn() -> i32>,

    // Packets
    packet_init:
        Option<unsafe extern "C" fn(PacketSendCallback, PacketProcessCallback, *mut PacketState)>,
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
    terminal_register_command_callback:
        Option<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char, TerminalCallback)>,
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

#[inline(always)]
unsafe fn vesc_if() -> *const VescIf {
    #[cfg(test)]
    if let Some(table) = crate::test_support::current_table() {
        return table;
    }
    VescIfAbi::BASE_ADDR.0 as *const VescIf
}

/// # Safety
///
/// `name` must point to a valid, NUL-terminated extension name and
/// `handler` must use the firmware LispBM extension ABI.
pub unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> bool {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        lbm_add_extension_with_table_base(VescIfAbi::BASE_ADDR.0 as u32, name, handler)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        match (*vesc_if()).lbm_add_extension {
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
    unsafe {
        let table = if vesc_if_base == VescIfAbi::BASE_ADDR.0 as u32 {
            vesc_if()
        } else {
            vesc_if_base as *const VescIf
        };
        match (*table).lbm_add_extension {
            Some(lbm_add_extension) => lbm_add_extension(name as *mut c_char, handler),
            None => false,
        }
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
        let lbm_dec_as_i32: unsafe extern "C" fn(u32) -> i32 = core::mem::transmute(lbm_dec_as_i32);
        lbm_dec_as_i32(value.0)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        match (*vesc_if()).lbm_dec_as_i32 {
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
        LbmValue(lbm_enc_i(value))
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        match (*vesc_if()).lbm_enc_i {
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
        let lbm_is_number: unsafe extern "C" fn(u32) -> bool = core::mem::transmute(lbm_is_number);
        lbm_is_number(value.0)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        match (*vesc_if()).lbm_is_number {
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
        LbmValue(lbm_enc_sym_eerror as u32)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        LbmValue((*vesc_if()).lbm_enc_sym_eerror as u32)
    }
}

/// Register or clear the firmware app-data callback using the refloat/C ABI.
///
/// Pass a null function pointer to clear the handler.
///
/// # Safety
///
/// `handler` must remain valid until replaced or cleared by a later firmware call.
pub unsafe fn vesc_set_app_data_handler(handler: AppDataHandler) -> bool {
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
        let set_app_data_handler: unsafe extern "C" fn(AppDataHandler) -> bool =
            core::mem::transmute(set_app_data_handler);
        set_app_data_handler(handler)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        let Some(set_app_data_handler) = (*vesc_if()).set_app_data_handler else {
            return false;
        };

        set_app_data_handler(handler)
    }
}

/// Clear the firmware app-data callback.
///
/// # Safety
///
/// Must only be called when the firmware `VESC_IF` table is valid, same as
/// [`vesc_set_app_data_handler`].
pub unsafe fn vesc_clear_app_data_handler() -> bool {
    unsafe {
        let handler: AppDataHandler =
            core::mem::transmute::<*mut u8, AppDataHandler>(core::ptr::null_mut());
        vesc_set_app_data_handler(handler)
    }
}

/// # Safety
///
/// `data` must point to at least `len` bytes that remain valid for the
/// duration of the firmware call.
pub unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        let vesc_if = VescIfAbi::BASE_ADDR.0;
        let send_app_data: usize;
        core::arch::asm!(
            "ldr {send_app_data}, [{vesc_if}, #592]",
            vesc_if = in(reg) vesc_if,
            send_app_data = out(reg) send_app_data,
            options(nostack, preserves_flags),
        );
        if send_app_data != 0 {
            let send_app_data: unsafe extern "C" fn(*mut c_uchar, u32) =
                core::mem::transmute(send_app_data);
            send_app_data(data as *mut c_uchar, len);
        }
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        if let Some(send_app_data) = (*vesc_if()).send_app_data {
            send_app_data(data as *mut c_uchar, len);
        }
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_system_time_ticks() -> u32 {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        let vesc_if = VescIfAbi::BASE_ADDR.0;
        let system_time_ticks: usize;
        core::arch::asm!(
            "ldr {system_time_ticks}, [{vesc_if}, #952]",
            vesc_if = in(reg) vesc_if,
            system_time_ticks = out(reg) system_time_ticks,
            options(nostack, preserves_flags),
        );
        if system_time_ticks == 0 {
            return 0;
        }
        let system_time_ticks: unsafe extern "C" fn() -> u32 =
            core::mem::transmute(system_time_ticks);
        system_time_ticks()
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        match (*vesc_if()).system_time_ticks {
            Some(system_time_ticks) => system_time_ticks(),
            None => 0,
        }
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_set_mode(pin: crate::VescPin, mode: crate::VescPinMode) -> bool {
    unsafe {
        match (*vesc_if()).io_set_mode {
            Some(io_set_mode) => io_set_mode(pin.0, mode.0),
            None => false,
        }
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_write(pin: crate::VescPin, level: i32) -> bool {
    unsafe {
        match (*vesc_if()).io_write {
            Some(io_write) => io_write(pin.0, level),
            None => false,
        }
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_read(pin: crate::VescPin) -> bool {
    unsafe {
        match (*vesc_if()).io_read {
            Some(io_read) => io_read(pin.0),
            None => false,
        }
    }
}

#[cfg(test)]
pub fn vesc_if_offsets_for_tests() -> [usize; 11] {
    [
        core::mem::offset_of!(VescIf, lbm_add_extension),
        core::mem::offset_of!(VescIf, lbm_enc_i),
        core::mem::offset_of!(VescIf, lbm_dec_as_i32),
        core::mem::offset_of!(VescIf, lbm_is_number),
        core::mem::offset_of!(VescIf, lbm_enc_sym_eerror),
        core::mem::offset_of!(VescIf, send_app_data),
        core::mem::offset_of!(VescIf, set_app_data_handler),
        core::mem::offset_of!(VescIf, system_time_ticks),
        core::mem::offset_of!(VescIf, io_set_mode),
        core::mem::offset_of!(VescIf, io_write),
        core::mem::offset_of!(VescIf, io_read),
    ]
}

#[cfg(test)]
mod dispatch_tests;

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
