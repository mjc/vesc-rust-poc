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

/// Raw remote-control state mirrored from the firmware ABI.
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
/// Refloat/VESC Tool custom-config serializer callback.
///
/// Refloat `v1.2.1` passes `get_cfg` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:549-550`.
pub type CustomConfigGet = unsafe extern "C" fn(*mut u8, bool) -> c_int;
/// Refloat/VESC Tool custom-config deserializer callback.
///
/// Refloat `v1.2.1` passes `set_cfg` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:551`.
pub type CustomConfigSet = unsafe extern "C" fn(*mut u8) -> bool;
/// Refloat/VESC Tool custom-config XML callback.
///
/// Refloat `v1.2.1` passes `get_cfg_xml` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:552`.
pub type CustomConfigXml = unsafe extern "C" fn(*mut *mut u8) -> c_int;
/// Refloat/VESC IMU read callback.
///
/// Refloat `v1.2.1` registers `imu_ref_callback` with this slot at
/// `src/main.c:2455`; the callback itself updates the balance filter at
/// `src/main.c:760-764`.
pub type ImuReadCallback = unsafe extern "C" fn(*mut f32, *mut f32, *mut f32, f32);
type EncoderReadCallback = unsafe extern "C" fn() -> f32;
type EncoderFaultCallback = unsafe extern "C" fn() -> bool;
type EncoderInfoCallback = unsafe extern "C" fn() -> *mut c_char;

/// Raw firmware function table mirrored from the VESC native ABI.
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

    // Custom config. Refloat v1.2.1 (0ef6e99d8701) registers these callbacks in
    // `src/main.c:2456` and clears them in `src/main.c:2403`.
    conf_custom_add_config:
        Option<unsafe extern "C" fn(CustomConfigGet, CustomConfigSet, CustomConfigXml)>,
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

#[cfg(not(all(target_arch = "arm", not(test))))]
#[inline(always)]
unsafe fn vesc_if() -> *const VescIf {
    #[cfg(test)]
    if let Some(table) = crate::test_support::current_table() {
        return table;
    }
    VescIfAbi::BASE_ADDR.0 as *const VescIf
}

#[cfg(all(target_arch = "arm", not(test)))]
#[inline(always)]
unsafe fn load_vesc_if_word_from<const OFFSET: usize>(vesc_if: usize) -> usize {
    let word: usize;
    unsafe {
        core::arch::asm!(
            "ldr {word}, [{vesc_if}, #{offset}]",
            vesc_if = in(reg) vesc_if,
            word = out(reg) word,
            offset = const OFFSET,
            options(nostack, preserves_flags),
        );
    }
    word
}

#[cfg(all(target_arch = "arm", not(test)))]
macro_rules! vesc_slot_word_from {
    ($vesc_if:expr, $name:ident) => {
        crate::raw::load_vesc_if_word_from::<{ crate::c_vesc_if::$name::VESC32_BYTE_OFFSET }>(
            $vesc_if as usize,
        )
    };
}

mod slots {
    use super::{
        AppDataHandler, CustomConfigGet, CustomConfigSet, CustomConfigXml, ExtensionHandler,
        ImuReadCallback, LibMutex, LibThread, VescIfAbi, c_char, c_int, c_uchar, c_void,
    };
    #[cfg(not(all(target_arch = "arm", not(test))))]
    use super::{VescIf, vesc_if};

    macro_rules! word_slot {
        ($name:ident) => {
            #[inline(always)]
            pub(super) unsafe fn $name() -> usize {
                #[cfg(all(target_arch = "arm", not(test)))]
                unsafe {
                    vesc_slot_word_from!(VescIfAbi::BASE_ADDR.0, $name)
                }

                #[cfg(not(all(target_arch = "arm", not(test))))]
                unsafe {
                    (*vesc_if()).$name
                }
            }
        };
    }

    macro_rules! fn_slot {
        ($name:ident as $fn_ty:ty) => {
            #[inline(always)]
            pub(super) unsafe fn $name() -> $fn_ty {
                #[cfg(all(target_arch = "arm", not(test)))]
                unsafe {
                    core::mem::transmute::<usize, $fn_ty>(vesc_slot_word_from!(
                        VescIfAbi::BASE_ADDR.0,
                        $name
                    ))
                }

                #[cfg(not(all(target_arch = "arm", not(test))))]
                unsafe {
                    (*vesc_if())
                        .$name
                        .expect("mock VESC_IF table must populate required slot")
                }
            }
        };
    }

    macro_rules! optional_fn_slot {
        ($name:ident as $fn_ty:ty) => {
            #[inline(always)]
            pub(super) unsafe fn $name() -> Option<$fn_ty> {
                #[cfg(all(target_arch = "arm", not(test)))]
                unsafe {
                    let address = vesc_slot_word_from!(VescIfAbi::BASE_ADDR.0, $name);
                    if address == 0 {
                        None
                    } else {
                        Some(core::mem::transmute::<usize, $fn_ty>(address))
                    }
                }

                #[cfg(not(all(target_arch = "arm", not(test))))]
                unsafe {
                    (*vesc_if()).$name
                }
            }
        };
    }

    #[cfg(all(target_arch = "arm", not(test)))]
    #[inline(always)]
    pub(super) unsafe fn lbm_add_extension_from(
        vesc_if_base: usize,
    ) -> unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool {
        unsafe {
            core::mem::transmute::<usize, unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool>(
                vesc_slot_word_from!(vesc_if_base, lbm_add_extension),
            )
        }
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    #[inline(always)]
    pub(super) unsafe fn lbm_add_extension_from(
        vesc_if_base: usize,
    ) -> unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool {
        let table = if vesc_if_base == VescIfAbi::BASE_ADDR.0 {
            unsafe { vesc_if() }
        } else {
            vesc_if_base as *const VescIf
        };
        unsafe {
            (*table)
                .lbm_add_extension
                .expect("mock VESC_IF table must populate lbm_add_extension")
        }
    }

    fn_slot!(lbm_dec_as_i32 as unsafe extern "C" fn(u32) -> i32);
    fn_slot!(lbm_enc_i as unsafe extern "C" fn(i32) -> u32);
    fn_slot!(lbm_is_number as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(set_app_data_handler as unsafe extern "C" fn(Option<AppDataHandler>) -> bool);
    fn_slot!(imu_set_read_callback as unsafe extern "C" fn(Option<ImuReadCallback>));

    word_slot!(lbm_enc_sym_nil);
    word_slot!(lbm_enc_sym_true);
    word_slot!(lbm_enc_sym_eerror);

    fn_slot!(
        conf_custom_add_config
            as unsafe extern "C" fn(CustomConfigGet, CustomConfigSet, CustomConfigXml)
    );
    fn_slot!(conf_custom_clear_configs as unsafe extern "C" fn());
    fn_slot!(mutex_create as unsafe extern "C" fn() -> LibMutex);
    fn_slot!(mutex_lock as unsafe extern "C" fn(LibMutex));
    fn_slot!(mutex_unlock as unsafe extern "C" fn(LibMutex));
    fn_slot!(malloc as unsafe extern "C" fn(usize) -> *mut c_void);
    fn_slot!(free as unsafe extern "C" fn(*mut c_void));
    fn_slot!(sleep_us as unsafe extern "C" fn(u32));
    fn_slot!(
        spawn
            as unsafe extern "C" fn(
                unsafe extern "C" fn(*mut c_void),
                usize,
                *const c_char,
                *mut c_void,
            ) -> LibThread
    );
    fn_slot!(request_terminate as unsafe extern "C" fn(LibThread));
    fn_slot!(should_terminate as unsafe extern "C" fn() -> bool);
    fn_slot!(get_arg as unsafe extern "C" fn(u32) -> *mut *mut c_void);
    fn_slot!(mc_get_fault as unsafe extern "C" fn() -> c_int);
    fn_slot!(mc_get_rpm as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_speed as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_directional_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_in_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_duty_cycle_now as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_input_voltage_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_amp_hours as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_amp_hours_charged as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_watt_hours as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_watt_hours_charged as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_battery_level as unsafe extern "C" fn(*mut f32) -> f32);
    fn_slot!(mc_get_distance_abs as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_odometer as unsafe extern "C" fn() -> u64);
    fn_slot!(get_cfg_float as unsafe extern "C" fn(c_int) -> f32);
    fn_slot!(get_cfg_int as unsafe extern "C" fn(c_int) -> c_int);
    fn_slot!(mc_set_duty as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_current as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_current_off_delay as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_brake_current as unsafe extern "C" fn(f32));
    fn_slot!(timeout_reset as unsafe extern "C" fn());
    // Refloat capability-probes this pre-6.05 slot because not every motor
    // implementation populates the FOC-specific function.
    optional_fn_slot!(foc_get_id as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_temp_fet_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_temp_motor_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_startup_done as unsafe extern "C" fn() -> bool);
    fn_slot!(imu_get_roll as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_pitch as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_yaw as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_gyro as unsafe extern "C" fn(*mut f32));
    fn_slot!(imu_get_quaternions as unsafe extern "C" fn(*mut f32));
    fn_slot!(send_app_data as unsafe extern "C" fn(*mut c_uchar, u32));
    fn_slot!(system_time as unsafe extern "C" fn() -> f32);
    // Appended in firmware 6.05; older tables fall back to `system_time`.
    optional_fn_slot!(system_time_ticks as unsafe extern "C" fn() -> u32);
    // Appended in firmware 6.06; callers treat absence as an unsupported hint.
    optional_fn_slot!(thread_set_priority as unsafe extern "C" fn(c_int));
    fn_slot!(io_set_mode as unsafe extern "C" fn(c_int, c_int) -> bool);
    fn_slot!(io_write as unsafe extern "C" fn(c_int, c_int) -> bool);
    fn_slot!(io_read as unsafe extern "C" fn(c_int) -> bool);
    fn_slot!(io_read_analog as unsafe extern "C" fn(c_int) -> f32);
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
        slots::lbm_add_extension_from(VescIfAbi::BASE_ADDR.0)(name as *mut c_char, handler)
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
        let lbm_add_extension = slots::lbm_add_extension_from(vesc_if_base as usize);
        lbm_add_extension(name as *mut c_char, handler)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        slots::lbm_add_extension_from(vesc_if_base as usize)(name as *mut c_char, handler)
    }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        slots::lbm_dec_as_i32()(value.0)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        slots::lbm_dec_as_i32()(value.0)
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_i(value: i32) -> LbmValue {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        LbmValue(slots::lbm_enc_i()(value))
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        LbmValue(slots::lbm_enc_i()(value))
    }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_number(value: LbmValue) -> bool {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        slots::lbm_is_number()(value.0)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        slots::lbm_is_number()(value.0)
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_nil() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_nil() as u32) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_true() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_true() as u32) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_eerror() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_eerror() as u32) }
}

/// Register the firmware app-data callback using the refloat/C ABI.
///
/// # Safety
///
/// `handler` must remain valid until replaced or cleared by a later firmware call.
pub unsafe fn vesc_set_app_data_handler(handler: AppDataHandler) -> bool {
    unsafe { vesc_set_app_data_handler_slot(Some(handler)) }
}

unsafe fn vesc_set_app_data_handler_slot(handler: Option<AppDataHandler>) -> bool {
    #[cfg(all(target_arch = "arm", not(test)))]
    unsafe {
        slots::set_app_data_handler()(handler)
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    unsafe {
        slots::set_app_data_handler()(handler)
    }
}

/// Clear the firmware app-data callback.
///
/// # Safety
///
/// Must only be called when the firmware `VESC_IF` table is valid, same as
/// [`vesc_set_app_data_handler`].
pub unsafe fn vesc_clear_app_data_handler() {
    let _ = unsafe { vesc_set_app_data_handler_slot(None) };
}

/// Register the firmware IMU read callback.
///
/// Refloat registers `imu_ref_callback` at `src/main.c:2455`; that callback
/// updates the balance filter at `src/main.c:760-764`. The VESC slot is
/// declared in `lispBM/c_libs/vesc_c_if.h:586`.
///
/// # Safety
///
/// `handler` must remain valid until replaced or cleared by a later firmware call.
pub unsafe fn vesc_set_imu_read_callback(handler: ImuReadCallback) {
    unsafe { vesc_set_imu_read_callback_slot(Some(handler)) }
}

unsafe fn vesc_set_imu_read_callback_slot(handler: Option<ImuReadCallback>) {
    unsafe { slots::imu_set_read_callback()(handler) }
}

/// Clear the firmware IMU read callback.
///
/// Refloat clears package callbacks during stop at `src/main.c:2401-2403`;
/// the VESC callback slot is declared in `lispBM/c_libs/vesc_c_if.h:586`.
///
/// # Safety
///
/// Must only be called when the firmware `VESC_IF` table is valid, same as
/// [`vesc_set_imu_read_callback`].
pub unsafe fn vesc_clear_imu_read_callback() {
    unsafe { vesc_set_imu_read_callback_slot(None) }
}

/// Read firmware IMU quaternions.
///
/// Refloat initializes its balance filter from firmware quaternions at
/// `src/balance_filter.c:53-61`; the VESC slot is declared in
/// `lispBM/c_libs/vesc_c_if.h:521`.
///
/// # Safety
///
/// `quaternions` must point to four writable `f32` values.
pub unsafe fn vesc_imu_get_quaternions(quaternions: *mut f32) {
    unsafe { slots::imu_get_quaternions()(quaternions) }
}

/// Register firmware custom-config callbacks using the Refloat/VESC ABI.
///
/// Refloat `v1.2.1` registers `get_cfg`, `set_cfg`, and `get_cfg_xml` through
/// this slot in `src/main.c:2456`. The VESC function-table slot is declared in
/// `vesc_pkg_lib/vesc_c_if.h:549-552`.
///
/// # Safety
///
/// The callbacks must remain valid until package stop clears them or the
/// firmware replaces them.
pub unsafe fn conf_custom_add_config(
    get_cfg: CustomConfigGet,
    set_cfg: CustomConfigSet,
    get_cfg_xml: CustomConfigXml,
) {
    unsafe { slots::conf_custom_add_config()(get_cfg, set_cfg, get_cfg_xml) }
}

/// Clear firmware custom-config callbacks.
///
/// Refloat `v1.2.1` calls this during stop in `src/main.c:2403`. The VESC
/// function-table slot is declared in `vesc_pkg_lib/vesc_c_if.h:553`.
///
/// # Safety
///
/// Must only be called while the firmware `VESC_IF` table is valid.
pub unsafe fn conf_custom_clear_configs() {
    unsafe { slots::conf_custom_clear_configs()() }
}

/// Allocate and initialize a firmware mutex.
///
/// The returned mutex belongs to the firmware reserve heap and must eventually
/// be released with [`vesc_free`].
///
/// # Safety
///
/// The firmware `VESC_IF` table must be valid.
pub unsafe fn vesc_mutex_create() -> LibMutex {
    unsafe { slots::mutex_create()() }
}

/// Lock a firmware mutex, blocking the current firmware thread.
///
/// # Safety
///
/// `mutex` must be a live handle returned by [`vesc_mutex_create`]. The mutex
/// is non-recursive and must not already be owned by the current thread.
pub unsafe fn vesc_mutex_lock(mutex: LibMutex) {
    unsafe { slots::mutex_lock()(mutex) };
}

/// Unlock a firmware mutex owned by the current firmware thread.
///
/// # Safety
///
/// `mutex` must be a live handle returned by [`vesc_mutex_create`] and locked
/// by the current thread.
pub unsafe fn vesc_mutex_unlock(mutex: LibMutex) {
    unsafe { slots::mutex_unlock()(mutex) };
}

/// Allocate memory from the firmware LispBM reserve heap.
///
/// # Safety
///
/// The caller must check for null. A non-null returned pointer belongs to the
/// firmware/LispBM reserve heap and must be freed with [`vesc_free`] when no
/// longer used.
pub unsafe fn vesc_malloc(bytes: usize) -> *mut c_void {
    unsafe { slots::malloc()(bytes) }
}

/// Free memory previously allocated by [`vesc_malloc`].
///
/// # Safety
///
/// `ptr` must be null or a pointer returned by the firmware allocator, and it
/// must not already have been freed.
pub unsafe fn vesc_free(ptr: *mut c_void) {
    unsafe {
        slots::free()(ptr);
    }
}

/// Spawn a firmware package thread.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:382` and starts its main/auxiliary threads at
/// `src/main.c:2438-2448`.
///
/// # Safety
///
/// `entry` and `name` must remain valid for the firmware call, and `arg` must
/// point to state that lives until the spawned thread terminates.
pub unsafe fn vesc_spawn(
    entry: unsafe extern "C" fn(*mut c_void),
    stack_bytes: usize,
    name: *const c_char,
    arg: *mut c_void,
) -> LibThread {
    unsafe { slots::spawn()(entry, stack_bytes, name, arg) }
}

/// Sleep the current firmware package thread for a number of microseconds.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:376` and sleeps the main loop at
/// `src/main.c:1080`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_sleep_us(micros: u32) {
    unsafe { slots::sleep_us()(micros) };
}

/// Set the current firmware package thread priority when the slot is present.
///
/// Refloat v1.2.1 checks optional `thread_set_priority` before lowering
/// `aux_thd` priority at `src/main.c:1133-1135`; the VESC ABI slot is declared
/// at `vesc_pkg_lib/vesc_c_if.h:670`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_thread_set_priority(priority: c_int) -> bool {
    unsafe { slots::thread_set_priority() }
        .map(|func| unsafe { func(priority) })
        .is_some()
}
/// Ask a firmware package thread to terminate.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:383` and requests thread termination during stop
/// at `src/main.c:2404-2408`.
///
/// # Safety
///
/// `thread` must be null or a thread handle returned by [`vesc_spawn`].
pub unsafe fn vesc_request_terminate(thread: LibThread) {
    unsafe {
        slots::request_terminate()(thread);
    }
}

/// Return whether the current firmware package thread should terminate.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:384` and loops on it in `refloat_thd` and
/// `aux_thd` at `src/main.c:771` and `src/main.c:1138`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_should_terminate() -> bool {
    unsafe { slots::should_terminate()() }
}

/// Return the firmware-owned mutable `lib_info.arg` slot for a loaded native library.
///
/// # Safety
///
/// `prog_addr` must be the native library base address passed by the VESC loader.
pub unsafe fn vesc_get_arg(prog_addr: u32) -> *mut *mut c_void {
    unsafe { slots::get_arg()(prog_addr) }
}

/// Return the active motor fault code, or zero for no fault.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_fault() -> c_int {
    unsafe { slots::mc_get_fault()() }
}

/// Return the current motor electrical RPM.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:108`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:450`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_rpm() -> f32 {
    unsafe { slots::mc_get_rpm()() }
}

/// Return firmware-calculated vehicle speed in meters per second.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:118`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:470`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_speed() -> f32 {
    unsafe { slots::mc_get_speed()() }
}

/// Return filtered total motor current.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:120`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:456`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_filtered() -> f32 {
    unsafe { slots::mc_get_tot_current_filtered()() }
}

/// Return direction-adjusted filtered motor current.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:121`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:458`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_directional_filtered() -> f32 {
    unsafe { slots::mc_get_tot_current_directional_filtered()() }
}

/// Return filtered input/battery current.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:140`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:460`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_in_filtered() -> f32 {
    unsafe { slots::mc_get_tot_current_in_filtered()() }
}

/// Read a firmware motor configuration float by `CFG_PARAM_*` id.
///
/// Refloat v1.2.1 reads `CFG_PARAM_l_current_max` and
/// `CFG_PARAM_l_current_min` in `src/motor_data.c:90-91`; the VESC ABI slot is
/// declared at `vesc_pkg_lib/vesc_c_if.h:588`.
///
/// # Safety
///
/// The firmware VESC function table must be valid and `param` must be a valid
/// firmware configuration parameter id for a float-valued setting.
pub unsafe fn get_cfg_float(param: c_int) -> f32 {
    unsafe { slots::get_cfg_float()(param) }
}

/// Read a firmware motor configuration integer by `CFG_PARAM_*` id.
///
/// Refloat v1.2.1 reads `CFG_PARAM_si_battery_cells` in
/// `src/motor_data.c:76`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:590`.
///
/// # Safety
///
/// The firmware VESC function table must be valid and `param` must be a valid
/// firmware configuration parameter id for an integer-valued setting.
pub unsafe fn get_cfg_int(param: c_int) -> c_int {
    unsafe { slots::get_cfg_int()(param) }
}

/// Reset the firmware motor-command safety timeout.
///
/// Refloat v1.2.1 calls this before every motor-control apply branch in
/// `third_party/refloat/src/motor_control.c:92-93`; the VESC ABI slot is declared at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:538`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn timeout_reset() {
    unsafe { slots::timeout_reset()() }
}

/// Keep current control enabled after a current command.
///
/// Refloat v1.2.1 calls this before `mc_set_current` in
/// `third_party/refloat/src/motor_control.c:96-99`; the VESC ABI slot is declared at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:476`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_current_off_delay(seconds: f32) {
    unsafe { slots::mc_set_current_off_delay()(seconds) }
}

/// Set the motor current command in amps.
///
/// Refloat v1.2.1 sends requested current in
/// `third_party/refloat/src/motor_control.c:96-99`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:440`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_current(amps: f32) {
    unsafe { slots::mc_set_current()(amps) }
}

/// Set the motor duty-cycle command.
///
/// Refloat v1.2.1 applies parking brake duty zero in
/// `third_party/refloat/src/motor_control.c:112-114`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_duty(duty_cycle: f32) {
    unsafe { slots::mc_set_duty()(duty_cycle) }
}

/// Set the motor brake current command in amps.
///
/// Refloat v1.2.1 applies idle brake current in
/// `third_party/refloat/src/motor_control.c:115-117`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_brake_current(amps: f32) {
    unsafe { slots::mc_set_brake_current()(amps) }
}

/// Return the current duty cycle.
///
/// Refloat v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:124`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:448`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_duty_cycle_now() -> f32 {
    unsafe { slots::mc_get_duty_cycle_now()() }
}

/// Return FOC d-axis Id current when the firmware slot is present.
///
/// Refloat v1.2.1 reads optional `foc_get_id` while encoding compact all-data
/// at `src/main.c:1364-1368`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:616`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn foc_get_id() -> Option<f32> {
    unsafe { slots::foc_get_id() }.map(|func| unsafe { func() })
}
/// Return the filtered input/battery voltage.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_input_voltage_filtered() -> f32 {
    unsafe { slots::mc_get_input_voltage_filtered()() }
}

/// Return the discharged amp-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_amp_hours(reset: bool) -> f32 {
    unsafe { slots::mc_get_amp_hours()(reset) }
}

/// Return the charged amp-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_amp_hours_charged(reset: bool) -> f32 {
    unsafe { slots::mc_get_amp_hours_charged()(reset) }
}

/// Return the discharged watt-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_watt_hours(reset: bool) -> f32 {
    unsafe { slots::mc_get_watt_hours()(reset) }
}

/// Return the charged watt-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_watt_hours_charged(reset: bool) -> f32 {
    unsafe { slots::mc_get_watt_hours_charged()(reset) }
}

/// Return the estimated battery level as a ratio.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid. If
/// `wh_left` is not null, it must be valid for firmware to write one `f32`.
pub unsafe fn mc_get_battery_level(wh_left: *mut f32) -> f32 {
    unsafe { slots::mc_get_battery_level()(wh_left) }
}

/// Return the absolute motor distance in meters.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_distance_abs() -> f32 {
    unsafe { slots::mc_get_distance_abs()() }
}

/// Return the odometer distance in meters.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_odometer() -> u64 {
    unsafe { slots::mc_get_odometer()() }
}

/// Return the filtered MOSFET/FET temperature in degrees Celsius.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_temp_fet_filtered() -> f32 {
    unsafe { slots::mc_temp_fet_filtered()() }
}

/// Return the filtered motor temperature in degrees Celsius.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_temp_motor_filtered() -> f32 {
    unsafe { slots::mc_temp_motor_filtered()() }
}

/// Return whether firmware IMU startup has completed.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:510` and gates startup readiness at
/// `src/main.c:834-838`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_startup_done() -> bool {
    unsafe { slots::imu_startup_done()() }
}

/// Return firmware IMU roll in radians.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:511` and reads it in `src/imu.c:35-40`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_roll() -> f32 {
    unsafe { slots::imu_get_roll()() }
}

/// Return firmware IMU pitch in radians.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:512` and reads it in `src/imu.c:37-38`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_pitch() -> f32 {
    unsafe { slots::imu_get_pitch()() }
}

/// Return firmware IMU yaw in radians.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:513` and reads it in `src/imu.c:39-40`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_yaw() -> f32 {
    unsafe { slots::imu_get_yaw()() }
}

/// Write firmware IMU gyro axes in degrees/sec into `xyz`.
///
/// Refloat v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:516` and reads it in `src/imu.c:45-53`.
///
/// # Safety
///
/// `xyz` must point to three writable `f32` values, and the VESC function
/// table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_gyro(xyz: *mut f32) {
    unsafe { slots::imu_get_gyro()(xyz) }
}

/// # Safety
///
/// `data` must point to at least `len` bytes that remain valid for the
/// duration of the firmware call.
pub unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
    unsafe {
        slots::send_app_data()(data as *mut c_uchar, len);
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_system_time_ticks() -> u32 {
    unsafe {
        if let Some(system_time_ticks) = slots::system_time_ticks() {
            system_time_ticks()
        } else {
            // Legacy VESC tables expose seconds only. The firmware system tick
            // is 100 microseconds (10 kHz), matching chVTGetSystemTimeX().
            (slots::system_time()() * 10_000.0) as u32
        }
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_set_mode(pin: crate::VescPin, mode: crate::VescPinMode) -> bool {
    unsafe { slots::io_set_mode()(pin.0, mode.0) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_write(pin: crate::VescPin, level: i32) -> bool {
    unsafe { slots::io_write()(pin.0, level) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_read(pin: crate::VescPin) -> bool {
    unsafe { slots::io_read()(pin.0) }
}

/// # Safety
///
/// The VESC slot is declared in `lispBM/c_libs/vesc_c_if.h:396`.
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
#[inline(always)]
pub unsafe fn io_read_analog(pin: crate::VescPin) -> f32 {
    unsafe { slots::io_read_analog()(pin.0) }
}

/// # Safety
///
/// The VESC slot is declared in `lispBM/c_libs/vesc_c_if.h:396`.
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
#[inline(always)]
pub unsafe fn io_read_analog_pair(first: crate::VescPin, second: crate::VescPin) -> (f32, f32) {
    let read = unsafe { slots::io_read_analog() };
    (unsafe { read(first.0) }, unsafe { read(second.0) })
}

/// Returns selected `VescIf` field offsets for ABI layout tests.
#[cfg(test)]
pub fn vesc_if_offsets_for_tests() -> [usize; VescIfAbi::USED_SLOT_COUNT] {
    macro_rules! offsets {
        ($($const_name:ident => $slot_name:ident),+ $(,)?) => {
            [$(core::mem::offset_of!(VescIf, $slot_name)),+]
        };
    }

    vesc_if_used_slots!(offsets)
}
#[cfg(test)]
mod abi_audit;
#[cfg(test)]
mod dispatch_tests;

/// Returns `VescIf` size, alignment, and final-slot offset for ABI layout tests.
#[cfg(test)]
pub fn vesc_if_full_layout_for_tests() -> (usize, usize, usize) {
    (
        core::mem::size_of::<VescIf>(),
        core::mem::align_of::<VescIf>(),
        core::mem::offset_of!(VescIf, shutdown_disable),
    )
}

/// Returns host mock function-pointer slot size and alignment for ABI layout tests.
#[cfg(test)]
pub fn mock_fn_slot_layout_for_tests() -> (usize, usize) {
    (
        core::mem::size_of::<Option<unsafe extern "C" fn()>>(),
        core::mem::align_of::<Option<unsafe extern "C" fn()>>(),
    )
}
