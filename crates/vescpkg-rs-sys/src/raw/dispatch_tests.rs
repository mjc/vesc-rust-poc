use core::cell::Cell;
use core::ffi::{c_char, c_int, c_uchar, c_uint, c_void};

use crate::c_vesc_if;
use crate::test_support::{empty_table, with_table};
use crate::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi, VescPin, VescPinMode};

use super::{
    CanStatusMsg, CustomConfigGet, CustomConfigSet, CustomConfigXml, GnssData, RemoteState, VescIf,
    can_status_msg_index, conf_custom_add_config, conf_custom_clear_configs, foc_get_id,
    gnss_snapshot, io_read, io_read_analog, io_set_mode, io_write, lbm_add_extension,
    lbm_add_extension_with_table_base, lbm_dec_as_float, lbm_dec_as_i32, lbm_enc_i,
    lbm_enc_sym_eerror, lbm_enc_sym_nil, lbm_enc_sym_true, lbm_is_number, mc_get_amp_hours,
    mc_get_amp_hours_charged, mc_get_battery_level, mc_get_distance_abs, mc_get_duty_cycle_now,
    mc_get_fault, mc_get_input_voltage_filtered, mc_get_odometer, mc_get_rpm, mc_get_speed,
    mc_get_tot_current_directional_filtered, mc_get_tot_current_filtered,
    mc_get_tot_current_in_filtered, mc_get_watt_hours, mc_get_watt_hours_charged,
    mc_temp_fet_filtered, mc_temp_motor_filtered, read_eeprom_word, read_nvm, remote_state,
    store_eeprom_word, vesc_clear_app_data_handler, vesc_mutex_create, vesc_mutex_lock,
    vesc_mutex_unlock, vesc_send_app_data, vesc_set_app_data_handler, vesc_sleep_us,
    vesc_system_time_ticks, vesc_thread_set_priority, wipe_nvm, write_nvm,
};

struct SyncCounter(Cell<usize>);

unsafe impl Sync for SyncCounter {}

impl SyncCounter {
    const fn new() -> Self {
        Self(Cell::new(0))
    }

    fn get(&self) -> usize {
        self.0.get()
    }

    fn inc(&self) {
        self.0.set(self.get() + 1);
    }

    fn set(&self, value: usize) {
        self.0.set(value);
    }
}

struct SyncI32(Cell<i32>);

unsafe impl Sync for SyncI32 {}

impl SyncI32 {
    const fn new() -> Self {
        Self(Cell::new(0))
    }

    fn get(&self) -> i32 {
        self.0.get()
    }

    fn set(&self, value: i32) {
        self.0.set(value);
    }
}

struct SyncU32(Cell<u32>);

unsafe impl Sync for SyncU32 {}

impl SyncU32 {
    const fn new() -> Self {
        Self(Cell::new(0))
    }

    fn get(&self) -> u32 {
        self.0.get()
    }

    fn set(&self, value: u32) {
        self.0.set(value);
    }
}

struct SyncF32(Cell<f32>);

unsafe impl Sync for SyncF32 {}

impl SyncF32 {
    const fn new() -> Self {
        Self(Cell::new(0.0))
    }

    fn get(&self) -> f32 {
        self.0.get()
    }

    fn set(&self, value: f32) {
        self.0.set(value);
    }
}

struct SyncBool(Cell<bool>);

unsafe impl Sync for SyncBool {}

impl SyncBool {
    const fn new() -> Self {
        Self(Cell::new(false))
    }

    fn get(&self) -> bool {
        self.0.get()
    }

    fn set(&self, value: bool) {
        self.0.set(value);
    }
}

static LBM_ADD_EXTENSION: SyncCounter = SyncCounter::new();
static LBM_DEC_AS_I32: SyncCounter = SyncCounter::new();
static LBM_DEC_AS_FLOAT: SyncCounter = SyncCounter::new();
static LBM_ENC_I: SyncCounter = SyncCounter::new();
static LBM_IS_NUMBER: SyncCounter = SyncCounter::new();
static SET_APP_DATA_HANDLER: SyncCounter = SyncCounter::new();
static SEND_APP_DATA: SyncCounter = SyncCounter::new();
static SEND_APP_DATA_LEN: SyncU32 = SyncU32::new();
static CONF_CUSTOM_ADD_CONFIG: SyncCounter = SyncCounter::new();
static CONF_CUSTOM_CLEAR_CONFIGS: SyncCounter = SyncCounter::new();
static CUSTOM_CONFIG_GET: SyncCounter = SyncCounter::new();
static CUSTOM_CONFIG_SET: SyncCounter = SyncCounter::new();
static CUSTOM_CONFIG_XML: SyncCounter = SyncCounter::new();
static SLEEP_US: SyncCounter = SyncCounter::new();
static THREAD_SET_PRIORITY: SyncCounter = SyncCounter::new();
static MUTEX_CREATE: SyncCounter = SyncCounter::new();
static MUTEX_LOCK: SyncCounter = SyncCounter::new();
static MUTEX_UNLOCK: SyncCounter = SyncCounter::new();
static SYSTEM_TIME: SyncCounter = SyncCounter::new();
static SYSTEM_TIME_TICKS: SyncCounter = SyncCounter::new();
static IO_SET_MODE: SyncCounter = SyncCounter::new();
static IO_WRITE: SyncCounter = SyncCounter::new();
static IO_READ: SyncCounter = SyncCounter::new();
static IO_READ_ANALOG: SyncCounter = SyncCounter::new();
static MC_GET_DISTANCE_ABS: SyncCounter = SyncCounter::new();
static MC_GET_RPM: SyncCounter = SyncCounter::new();
static MC_GET_SPEED: SyncCounter = SyncCounter::new();
static MC_GET_TOT_CURRENT_FILTERED: SyncCounter = SyncCounter::new();
static MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED: SyncCounter = SyncCounter::new();
static MC_GET_TOT_CURRENT_IN_FILTERED: SyncCounter = SyncCounter::new();
static MC_GET_DUTY_CYCLE_NOW: SyncCounter = SyncCounter::new();
static FOC_GET_ID: SyncCounter = SyncCounter::new();
static MC_TEMP_FET_FILTERED: SyncCounter = SyncCounter::new();
static MC_TEMP_MOTOR_FILTERED: SyncCounter = SyncCounter::new();
static MC_GET_AMP_HOURS: SyncCounter = SyncCounter::new();
static MC_GET_AMP_HOURS_CHARGED: SyncCounter = SyncCounter::new();
static MC_GET_WATT_HOURS: SyncCounter = SyncCounter::new();
static MC_GET_WATT_HOURS_CHARGED: SyncCounter = SyncCounter::new();
static MC_GET_BATTERY_LEVEL: SyncCounter = SyncCounter::new();
static MC_GET_ODOMETER: SyncCounter = SyncCounter::new();
static MC_GET_FAULT: SyncCounter = SyncCounter::new();
static MC_GET_INPUT_VOLTAGE_FILTERED: SyncCounter = SyncCounter::new();
static READ_EEPROM_VAR: SyncCounter = SyncCounter::new();
static STORE_EEPROM_VAR: SyncCounter = SyncCounter::new();
static LAST_PIN: SyncI32 = SyncI32::new();
static LAST_MODE: SyncI32 = SyncI32::new();
static LAST_LEVEL: SyncI32 = SyncI32::new();
static LAST_LBM_VALUE: SyncU32 = SyncU32::new();
static LAST_SLEEP_US: SyncU32 = SyncU32::new();
static LAST_THREAD_PRIORITY: SyncI32 = SyncI32::new();
static LAST_FOC_ID: SyncF32 = SyncF32::new();
static LAST_HANDLER_INSTALLED: SyncBool = SyncBool::new();
static LAST_CUSTOM_CONFIG_DEFAULT: SyncBool = SyncBool::new();
static LAST_EEPROM_ADDRESS: SyncI32 = SyncI32::new();
static LAST_EEPROM_WORD: SyncU32 = SyncU32::new();
static CAN_STATUS: CanStatusMsg = CanStatusMsg {
    id: 17,
    rx_time: 1234,
    rpm: 1500.0,
    current: 4.5,
    duty: 0.25,
};
static GNSS: GnssData = GnssData {
    lat: 40.015,
    lon: -105.2705,
    height: 1624.0,
    speed: 3.5,
    hdop: 0.9,
    ms_today: 12_345,
    yy: 26,
    mo: 7,
    dd: 21,
    last_update: 9876,
};

fn reset_counters() {
    for counter in [
        &LBM_ADD_EXTENSION,
        &LBM_DEC_AS_I32,
        &LBM_DEC_AS_FLOAT,
        &LBM_ENC_I,
        &LBM_IS_NUMBER,
        &SET_APP_DATA_HANDLER,
        &SEND_APP_DATA,
        &CONF_CUSTOM_ADD_CONFIG,
        &CONF_CUSTOM_CLEAR_CONFIGS,
        &CUSTOM_CONFIG_GET,
        &CUSTOM_CONFIG_SET,
        &CUSTOM_CONFIG_XML,
        &SLEEP_US,
        &THREAD_SET_PRIORITY,
        &MUTEX_CREATE,
        &MUTEX_LOCK,
        &MUTEX_UNLOCK,
        &SYSTEM_TIME,
        &SYSTEM_TIME_TICKS,
        &IO_SET_MODE,
        &IO_WRITE,
        &IO_READ,
        &IO_READ_ANALOG,
        &MC_GET_RPM,
        &MC_GET_SPEED,
        &MC_GET_TOT_CURRENT_FILTERED,
        &MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED,
        &MC_GET_TOT_CURRENT_IN_FILTERED,
        &MC_GET_DUTY_CYCLE_NOW,
        &FOC_GET_ID,
        &MC_GET_DISTANCE_ABS,
        &MC_TEMP_FET_FILTERED,
        &MC_TEMP_MOTOR_FILTERED,
        &MC_GET_AMP_HOURS,
        &MC_GET_AMP_HOURS_CHARGED,
        &MC_GET_WATT_HOURS,
        &MC_GET_WATT_HOURS_CHARGED,
        &MC_GET_BATTERY_LEVEL,
        &MC_GET_ODOMETER,
        &MC_GET_FAULT,
        &MC_GET_INPUT_VOLTAGE_FILTERED,
        &READ_EEPROM_VAR,
        &STORE_EEPROM_VAR,
    ] {
        counter.set(0);
    }
    SEND_APP_DATA_LEN.set(0);
    LAST_PIN.set(0);
    LAST_MODE.set(0);
    LAST_LEVEL.set(0);
    LAST_LBM_VALUE.set(0);
    LAST_SLEEP_US.set(0);
    LAST_THREAD_PRIORITY.set(0);
    LAST_FOC_ID.set(0.0);
    LAST_HANDLER_INSTALLED.set(false);
    LAST_CUSTOM_CONFIG_DEFAULT.set(false);
    LAST_EEPROM_ADDRESS.set(0);
    LAST_EEPROM_WORD.set(0);
}

extern "C" fn stub_lbm_add_extension(_name: *mut c_char, _handler: ExtensionHandler) -> bool {
    LBM_ADD_EXTENSION.inc();
    true
}

extern "C" fn stub_lbm_dec_as_i32(value: u32) -> i32 {
    LBM_DEC_AS_I32.inc();
    LAST_LBM_VALUE.set(value);
    value as i32
}

extern "C" fn stub_lbm_dec_as_float(value: LbmValue) -> f32 {
    LBM_DEC_AS_FLOAT.inc();
    LAST_LBM_VALUE.set(value.0);
    4.25
}

extern "C" fn stub_lbm_enc_i(value: i32) -> u32 {
    LBM_ENC_I.inc();
    value as u32 + 1
}

extern "C" fn stub_lbm_is_number(value: u32) -> bool {
    LBM_IS_NUMBER.inc();
    LAST_LBM_VALUE.set(value);
    value == 7
}

extern "C" fn stub_read_eeprom_var(word: *mut super::EepromVar, address: c_int) -> bool {
    READ_EEPROM_VAR.inc();
    LAST_EEPROM_ADDRESS.set(address);
    let Some(word) = (unsafe { word.cast::<u32>().as_mut() }) else {
        return false;
    };
    *word = 0x1234_5678;
    true
}

extern "C" fn stub_store_eeprom_var(word: *mut super::EepromVar, address: c_int) -> bool {
    STORE_EEPROM_VAR.inc();
    LAST_EEPROM_ADDRESS.set(address);
    let Some(word) = (unsafe { word.cast::<u32>().as_ref() }) else {
        return false;
    };
    LAST_EEPROM_WORD.set(*word);
    true
}

extern "C" fn stub_can_get_status_msg_index(_index: c_int) -> *mut CanStatusMsg {
    &CAN_STATUS as *const CanStatusMsg as *mut CanStatusMsg
}

extern "C" fn stub_mc_gnss() -> *mut GnssData {
    &GNSS as *const GnssData as *mut GnssData
}

extern "C" fn stub_get_remote_state() -> RemoteState {
    RemoteState {
        js_x: -0.25,
        js_y: 0.75,
        bt_c: true,
        bt_z: false,
        is_rev: true,
        age_s: 0.5,
    }
}

extern "C" fn stub_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
    SET_APP_DATA_HANDLER.inc();
    LAST_HANDLER_INSTALLED.set(handler.is_some());
    handler.is_some()
}

extern "C" fn stub_send_app_data(_data: *mut c_uchar, len: u32) {
    SEND_APP_DATA.inc();
    SEND_APP_DATA_LEN.set(len);
}

extern "C" fn stub_conf_custom_add_config(
    get_cfg: CustomConfigGet,
    set_cfg: CustomConfigSet,
    get_cfg_xml: CustomConfigXml,
) {
    CONF_CUSTOM_ADD_CONFIG.inc();
    unsafe {
        let _ = get_cfg(core::ptr::null_mut(), true);
        let _ = set_cfg(core::ptr::null_mut());
        let _ = get_cfg_xml(core::ptr::null_mut());
    }
}

extern "C" fn stub_conf_custom_clear_configs() {
    CONF_CUSTOM_CLEAR_CONFIGS.inc();
}

unsafe extern "C" fn custom_config_get(_buffer: *mut u8, is_default: bool) -> c_int {
    CUSTOM_CONFIG_GET.inc();
    LAST_CUSTOM_CONFIG_DEFAULT.set(is_default);
    11
}

unsafe extern "C" fn custom_config_set(_buffer: *mut u8) -> bool {
    CUSTOM_CONFIG_SET.inc();
    true
}

unsafe extern "C" fn custom_config_xml(_buffer: *mut *mut u8) -> c_int {
    CUSTOM_CONFIG_XML.inc();
    12
}

extern "C" fn stub_system_time_ticks() -> u32 {
    SYSTEM_TIME_TICKS.inc();
    42
}

extern "C" fn stub_system_time() -> f32 {
    SYSTEM_TIME.inc();
    1.25
}

extern "C" fn stub_sleep_us(micros: u32) {
    SLEEP_US.inc();
    LAST_SLEEP_US.set(micros);
}

extern "C" fn stub_thread_set_priority(priority: c_int) {
    THREAD_SET_PRIORITY.inc();
    LAST_THREAD_PRIORITY.set(priority);
}

extern "C" fn stub_mutex_create() -> *mut c_void {
    MUTEX_CREATE.inc();
    0xCAFEusize as *mut c_void
}

extern "C" fn stub_mutex_lock(mutex: *mut c_void) {
    assert_eq!(mutex as usize, 0xCAFE);
    MUTEX_LOCK.inc();
}

extern "C" fn stub_mutex_unlock(mutex: *mut c_void) {
    assert_eq!(mutex as usize, 0xCAFE);
    MUTEX_UNLOCK.inc();
}

extern "C" fn stub_io_set_mode(pin: c_int, mode: c_int) -> bool {
    IO_SET_MODE.inc();
    LAST_PIN.set(pin);
    LAST_MODE.set(mode);
    true
}

extern "C" fn stub_io_write(pin: c_int, level: c_int) -> bool {
    IO_WRITE.inc();
    LAST_PIN.set(pin);
    LAST_LEVEL.set(level);
    true
}

extern "C" fn stub_io_read(pin: c_int) -> bool {
    IO_READ.inc();
    LAST_PIN.set(pin);
    pin == 3
}

extern "C" fn stub_io_read_analog(pin: c_int) -> f32 {
    IO_READ_ANALOG.inc();
    LAST_PIN.set(pin);
    0.25 * pin as f32
}

extern "C" fn stub_mc_get_distance_abs() -> f32 {
    MC_GET_DISTANCE_ABS.inc();
    12.5
}

extern "C" fn stub_mc_get_rpm() -> f32 {
    MC_GET_RPM.inc();
    3210.0
}

extern "C" fn stub_mc_get_speed() -> f32 {
    MC_GET_SPEED.inc();
    12.25
}

extern "C" fn stub_mc_get_tot_current_filtered() -> f32 {
    MC_GET_TOT_CURRENT_FILTERED.inc();
    33.5
}

extern "C" fn stub_mc_get_tot_current_directional_filtered() -> f32 {
    MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED.inc();
    -21.25
}

extern "C" fn stub_mc_get_tot_current_in_filtered() -> f32 {
    MC_GET_TOT_CURRENT_IN_FILTERED.inc();
    -8.25
}

extern "C" fn stub_mc_get_duty_cycle_now() -> f32 {
    MC_GET_DUTY_CYCLE_NOW.inc();
    -0.42
}

extern "C" fn stub_foc_get_id() -> f32 {
    FOC_GET_ID.inc();
    LAST_FOC_ID.set(1.5);
    1.5
}

extern "C" fn stub_mc_temp_fet_filtered() -> f32 {
    MC_TEMP_FET_FILTERED.inc();
    44.0
}

extern "C" fn stub_mc_temp_motor_filtered() -> f32 {
    MC_TEMP_MOTOR_FILTERED.inc();
    51.5
}

extern "C" fn stub_mc_get_amp_hours(reset: bool) -> f32 {
    MC_GET_AMP_HOURS.inc();
    if reset { -1.0 } else { 3.2 }
}

extern "C" fn stub_mc_get_amp_hours_charged(reset: bool) -> f32 {
    MC_GET_AMP_HOURS_CHARGED.inc();
    if reset { -1.0 } else { 0.8 }
}

extern "C" fn stub_mc_get_watt_hours(reset: bool) -> f32 {
    MC_GET_WATT_HOURS.inc();
    if reset { -1.0 } else { 170.0 }
}

extern "C" fn stub_mc_get_watt_hours_charged(reset: bool) -> f32 {
    MC_GET_WATT_HOURS_CHARGED.inc();
    if reset { -1.0 } else { 18.5 }
}

extern "C" fn stub_mc_get_battery_level(wh_left: *mut f32) -> f32 {
    MC_GET_BATTERY_LEVEL.inc();
    if let Some(wh_left) = unsafe { wh_left.as_mut() } {
        *wh_left = 42.0;
    }
    0.72
}

extern "C" fn stub_mc_get_odometer() -> u64 {
    MC_GET_ODOMETER.inc();
    123_456
}

extern "C" fn stub_mc_get_fault() -> c_int {
    MC_GET_FAULT.inc();
    5
}

extern "C" fn stub_mc_get_input_voltage_filtered() -> f32 {
    MC_GET_INPUT_VOLTAGE_FILTERED.inc();
    84.2
}

unsafe extern "C" fn stub_read_nvm(buffer: *mut u8, offset: c_uint, len: c_uint) -> bool {
    let Some(buffer) = (unsafe { buffer.cast::<[u8; 4]>().as_mut() }) else {
        return false;
    };
    if len != buffer.len() as c_uint {
        return false;
    }
    buffer.copy_from_slice(&[
        offset as u8,
        offset.wrapping_add(1) as u8,
        offset.wrapping_add(2) as u8,
        offset.wrapping_add(3) as u8,
    ]);
    true
}

unsafe extern "C" fn stub_write_nvm(_buffer: *mut u8, _offset: c_uint, len: c_uint) -> bool {
    len != 0
}

unsafe extern "C" fn stub_wipe_nvm() -> bool {
    true
}

fn populated_table() -> VescIf {
    let mut table = empty_table();
    table.lbm_add_extension = Some(stub_lbm_add_extension);
    table.lbm_dec_as_float = Some(stub_lbm_dec_as_float);
    table.lbm_dec_as_i32 = Some(stub_lbm_dec_as_i32);
    table.lbm_enc_i = Some(stub_lbm_enc_i);
    table.lbm_is_number = Some(stub_lbm_is_number);
    table.lbm_enc_sym_nil = 0xAABB_0000;
    table.lbm_enc_sym_true = 0xAABB_1100;
    table.lbm_enc_sym_eerror = 0xAABB_CC00;
    table.read_eeprom_var = Some(stub_read_eeprom_var);
    table.store_eeprom_var = Some(stub_store_eeprom_var);
    table.can_get_status_msg_index = Some(stub_can_get_status_msg_index);
    table.mc_gnss = Some(stub_mc_gnss);
    table.get_remote_state = Some(stub_get_remote_state);
    table.set_app_data_handler = Some(stub_set_app_data_handler);
    table.send_app_data = Some(stub_send_app_data);
    table.conf_custom_add_config = Some(stub_conf_custom_add_config);
    table.conf_custom_clear_configs = Some(stub_conf_custom_clear_configs);
    table.sleep_us = Some(stub_sleep_us);
    table.thread_set_priority = Some(stub_thread_set_priority);
    table.mutex_create = Some(stub_mutex_create);
    table.mutex_lock = Some(stub_mutex_lock);
    table.mutex_unlock = Some(stub_mutex_unlock);
    table.system_time = Some(stub_system_time);
    table.system_time_ticks = Some(stub_system_time_ticks);
    table.io_set_mode = Some(stub_io_set_mode);
    table.io_write = Some(stub_io_write);
    table.io_read = Some(stub_io_read);
    table.io_read_analog = Some(stub_io_read_analog);
    table.mc_get_rpm = Some(stub_mc_get_rpm);
    table.mc_get_speed = Some(stub_mc_get_speed);
    table.mc_get_tot_current_filtered = Some(stub_mc_get_tot_current_filtered);
    table.mc_get_tot_current_directional_filtered =
        Some(stub_mc_get_tot_current_directional_filtered);
    table.mc_get_tot_current_in_filtered = Some(stub_mc_get_tot_current_in_filtered);
    table.mc_get_duty_cycle_now = Some(stub_mc_get_duty_cycle_now);
    table.foc_get_id = Some(stub_foc_get_id);
    table.mc_get_distance_abs = Some(stub_mc_get_distance_abs);
    table.mc_temp_fet_filtered = Some(stub_mc_temp_fet_filtered);
    table.mc_temp_motor_filtered = Some(stub_mc_temp_motor_filtered);
    table.mc_get_amp_hours = Some(stub_mc_get_amp_hours);
    table.mc_get_amp_hours_charged = Some(stub_mc_get_amp_hours_charged);
    table.mc_get_watt_hours = Some(stub_mc_get_watt_hours);
    table.mc_get_watt_hours_charged = Some(stub_mc_get_watt_hours_charged);
    table.mc_get_battery_level = Some(stub_mc_get_battery_level);
    table.mc_get_odometer = Some(stub_mc_get_odometer);
    table.mc_get_fault = Some(stub_mc_get_fault);
    table.mc_get_input_voltage_filtered = Some(stub_mc_get_input_voltage_filtered);
    table
}

fn with_populated_table<R>(body: impl FnOnce() -> R) -> R {
    let table = populated_table();
    with_table(&table, || {
        reset_counters();
        body()
    })
}

#[test]
fn nvm_dispatch_reports_firmware_results_and_absence() {
    let mut table = populated_table();
    table.read_nvm = Some(stub_read_nvm);
    table.write_nvm = Some(stub_write_nvm);
    table.wipe_nvm = Some(stub_wipe_nvm);

    with_table(&table, || unsafe {
        let mut bytes = [0; 4];
        assert_eq!(
            read_nvm(bytes.as_mut_ptr(), 7, bytes.len() as c_uint),
            Some(true)
        );
        assert_eq!(bytes, [7, 8, 9, 10]);
        assert_eq!(
            write_nvm(bytes.as_mut_ptr(), 7, bytes.len() as c_uint),
            Some(true)
        );
        assert_eq!(wipe_nvm(), Some(true));
    });

    let table = populated_table();
    with_table(&table, || unsafe {
        let mut bytes = [0; 4];
        assert_eq!(read_nvm(bytes.as_mut_ptr(), 0, bytes.len() as c_uint), None);
        assert_eq!(
            write_nvm(bytes.as_mut_ptr(), 0, bytes.len() as c_uint),
            None
        );
        assert_eq!(wipe_nvm(), None);
    });
}

#[test]
fn lbm_add_extension_forwards_through_mock_table() {
    with_populated_table(|| unsafe {
        extern "C" fn handler(_: *mut u32, _: u32) -> u32 {
            0
        }

        assert!(lbm_add_extension(c"ext-test".as_ptr(), handler));
        assert_eq!(LBM_ADD_EXTENSION.get(), 1);
    });
}

#[test]
fn lbm_add_extension_with_table_base_uses_mock_when_base_matches_firmware_addr() {
    with_populated_table(|| unsafe {
        extern "C" fn handler(_: *mut u32, _: u32) -> u32 {
            0
        }

        assert!(lbm_add_extension_with_table_base(
            VescIfAbi::BASE_ADDR.0 as u32,
            c"ext-test".as_ptr(),
            handler
        ));
        assert_eq!(LBM_ADD_EXTENSION.get(), 1);
    });
}

#[test]
fn lbm_value_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        assert_eq!(lbm_dec_as_float(LbmValue(9)), 4.25);
        assert_eq!(LAST_LBM_VALUE.get(), 9);
        assert_eq!(LBM_DEC_AS_FLOAT.get(), 1);
        assert_eq!(lbm_dec_as_i32(LbmValue(9)), 9);
        assert_eq!(lbm_enc_i(4), LbmValue(5));
        assert!(lbm_is_number(LbmValue(7)));
        assert!(!lbm_is_number(LbmValue(8)));
        assert_eq!(lbm_enc_sym_nil(), LbmValue(0xAABB_0000));
        assert_eq!(lbm_enc_sym_true(), LbmValue(0xAABB_1100));
        assert_eq!(lbm_enc_sym_eerror(), LbmValue(0xAABB_CC00));
    });
}

#[test]
fn eeprom_helpers_forward_word_pointers_and_addresses() {
    with_populated_table(|| unsafe {
        let mut read_word = 0;
        assert!(read_eeprom_word(&mut read_word, 7));
        assert_eq!(read_word, 0x1234_5678);
        assert_eq!(READ_EEPROM_VAR.get(), 1);
        assert_eq!(LAST_EEPROM_ADDRESS.get(), 7);

        let mut stored_word = 0xAABB_CCDD;
        assert!(store_eeprom_word(&mut stored_word, 9));
        assert_eq!(STORE_EEPROM_VAR.get(), 1);
        assert_eq!(LAST_EEPROM_ADDRESS.get(), 9);
        assert_eq!(LAST_EEPROM_WORD.get(), stored_word);
    });
}

#[test]
fn can_status_loader_copies_firmware_owned_records() {
    with_populated_table(|| unsafe {
        let status = can_status_msg_index(3).expect("mock CAN status record");
        assert_eq!(status.id, 17);
        assert_eq!(status.rx_time, 1234);
        assert_eq!(status.rpm, 1500.0);
        assert_eq!(status.current, 4.5);
        assert_eq!(status.duty, 0.25);
    });
}

#[test]
fn absent_can_status_loader_returns_none_without_calling_a_null_slot() {
    let table = empty_table();
    with_table(&table, || unsafe {
        assert!(can_status_msg_index(0).is_none());
    });
}

#[test]
fn gnss_and_remote_loaders_copy_firmware_owned_records() {
    with_populated_table(|| unsafe {
        let gnss = gnss_snapshot().expect("mock GNSS record");
        assert_eq!(gnss.lat, 40.015);
        assert_eq!(gnss.lon, -105.2705);
        assert_eq!(gnss.last_update, 9876);

        let remote = remote_state();
        assert_eq!(remote.js_x, -0.25);
        assert_eq!(remote.js_y, 0.75);
        assert!(remote.bt_c);
        assert!(remote.is_rev);
    });
}

#[test]
fn absent_gnss_loader_returns_none_without_dereferencing_a_null_record() {
    let table = empty_table();
    with_table(&table, || unsafe {
        assert!(gnss_snapshot().is_none());
    });
}

#[test]
fn app_data_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        extern "C" fn handler(_: *mut u8, _: u32) {}

        assert!(vesc_set_app_data_handler(handler));
        assert!(LAST_HANDLER_INSTALLED.get());
        let _: () = vesc_clear_app_data_handler();
        assert!(!LAST_HANDLER_INSTALLED.get());

        let payload = [1_u8, 2, 3];
        vesc_send_app_data(payload.as_ptr(), payload.len() as u32);
        assert_eq!(SEND_APP_DATA.get(), 1);
        assert_eq!(SEND_APP_DATA_LEN.get(), 3);
    });
}

#[test]
fn custom_config_helpers_forward_through_mock_table() {
    // Refloat v1.2.1 registers these callbacks in `src/main.c:2456`, clears them
    // in `src/main.c:2403`, and gets the ABI slots from `vesc_pkg_lib/vesc_c_if.h:549-553`.
    with_populated_table(|| unsafe {
        conf_custom_add_config(custom_config_get, custom_config_set, custom_config_xml);
        assert_eq!(CONF_CUSTOM_ADD_CONFIG.get(), 1);
        assert_eq!(CUSTOM_CONFIG_GET.get(), 1);
        assert_eq!(CUSTOM_CONFIG_SET.get(), 1);
        assert_eq!(CUSTOM_CONFIG_XML.get(), 1);
        assert!(LAST_CUSTOM_CONFIG_DEFAULT.get());

        conf_custom_clear_configs();
        assert_eq!(CONF_CUSTOM_CLEAR_CONFIGS.get(), 1);
    });
}

#[test]
fn system_time_ticks_forwards_through_mock_table() {
    with_populated_table(|| unsafe {
        assert_eq!(vesc_system_time_ticks(), 42);
        assert_eq!(SYSTEM_TIME_TICKS.get(), 1);
        assert_eq!(SYSTEM_TIME.get(), 0);
    });
}

#[test]
fn system_time_ticks_falls_back_to_seconds_when_tick_slot_is_absent_like_refloat() {
    let mut table = populated_table();
    table.system_time_ticks = None;

    with_table(&table, || {
        reset_counters();
        let ticks = unsafe { vesc_system_time_ticks() };

        assert_eq!(ticks, 12_500);
        assert_eq!(SYSTEM_TIME_TICKS.get(), 0);
        assert_eq!(SYSTEM_TIME.get(), 1);
    });
}

#[test]
fn sleep_us_forwards_through_mock_table() {
    with_populated_table(|| unsafe {
        vesc_sleep_us(1201);

        assert_eq!(SLEEP_US.get(), 1);
        assert_eq!(LAST_SLEEP_US.get(), 1201);
    });
}

#[test]
#[should_panic(expected = "mock VESC_IF table must populate required slot")]
fn missing_required_slot_fails_loudly() {
    let mut table = populated_table();
    table.sleep_us = None;

    with_table(&table, || unsafe {
        vesc_sleep_us(1);
    });
}

#[test]
fn thread_set_priority_forwards_through_mock_table() {
    with_populated_table(|| unsafe {
        assert!(vesc_thread_set_priority(-1));

        assert_eq!(THREAD_SET_PRIORITY.get(), 1);
        assert_eq!(LAST_THREAD_PRIORITY.get(), -1);
    });
}

#[test]
fn thread_set_priority_reports_absence_on_pre_6_06_tables() {
    let mut table = populated_table();
    table.thread_set_priority = None;

    with_table(&table, || unsafe {
        assert!(!vesc_thread_set_priority(-1));
        assert_eq!(THREAD_SET_PRIORITY.get(), 0);
    });
}

#[test]
fn mutex_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        let mutex = vesc_mutex_create();
        vesc_mutex_lock(mutex);
        vesc_mutex_unlock(mutex);

        assert_eq!(MUTEX_CREATE.get(), 1);
        assert_eq!(MUTEX_LOCK.get(), 1);
        assert_eq!(MUTEX_UNLOCK.get(), 1);
    });
}

#[test]
fn runtime_motor_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        assert_eq!(mc_get_rpm(), 3210.0);
        assert_eq!(mc_get_speed(), 12.25);
        assert_eq!(mc_get_tot_current_filtered(), 33.5);
        assert_eq!(mc_get_tot_current_directional_filtered(), -21.25);
        assert_eq!(mc_get_tot_current_in_filtered(), -8.25);
        assert_eq!(mc_get_duty_cycle_now(), -0.42);
        assert_eq!(foc_get_id(), Some(1.5));

        assert_eq!(MC_GET_RPM.get(), 1);
        assert_eq!(MC_GET_SPEED.get(), 1);
        assert_eq!(MC_GET_TOT_CURRENT_FILTERED.get(), 1);
        assert_eq!(MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED.get(), 1);
        assert_eq!(MC_GET_TOT_CURRENT_IN_FILTERED.get(), 1);
        assert_eq!(MC_GET_DUTY_CYCLE_NOW.get(), 1);
        assert_eq!(FOC_GET_ID.get(), 1);
        assert_eq!(LAST_FOC_ID.get(), 1.5);
    });
}

#[test]
fn foc_get_id_reports_absence_when_the_motor_does_not_expose_it() {
    let mut table = populated_table();
    table.foc_get_id = None;

    with_table(&table, || unsafe {
        assert_eq!(foc_get_id(), None);
        assert_eq!(FOC_GET_ID.get(), 0);
    });
}

#[test]
fn gpio_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        let pin = VescPin(3);
        let mode = VescPinMode(2);

        assert!(io_set_mode(pin, mode));
        assert!(io_write(pin, 1));
        assert!(io_read(pin));
        assert_eq!(io_read_analog(pin), 0.75);
        assert_eq!(LAST_PIN.get(), 3);
        assert_eq!(LAST_MODE.get(), 2);
        assert_eq!(LAST_LEVEL.get(), 1);
    });
}

#[test]
fn motor_data_helpers_forward_through_mock_table() {
    with_populated_table(|| unsafe {
        assert_eq!(mc_get_distance_abs(), 12.5);
        assert_eq!(mc_temp_fet_filtered(), 44.0);
        assert_eq!(mc_temp_motor_filtered(), 51.5);
        assert_eq!(mc_get_amp_hours(false), 3.2);
        assert_eq!(mc_get_amp_hours_charged(false), 0.8);
        assert_eq!(mc_get_watt_hours(false), 170.0);
        assert_eq!(mc_get_watt_hours_charged(false), 18.5);
        assert_eq!(mc_get_battery_level(core::ptr::null_mut()), 0.72);
        assert_eq!(mc_get_odometer(), 123_456);
        assert_eq!(mc_get_fault(), 5);
        assert_eq!(mc_get_input_voltage_filtered(), 84.2);
        assert_eq!(MC_GET_DISTANCE_ABS.get(), 1);
        assert_eq!(MC_TEMP_FET_FILTERED.get(), 1);
        assert_eq!(MC_TEMP_MOTOR_FILTERED.get(), 1);
        assert_eq!(MC_GET_AMP_HOURS.get(), 1);
        assert_eq!(MC_GET_AMP_HOURS_CHARGED.get(), 1);
        assert_eq!(MC_GET_WATT_HOURS.get(), 1);
        assert_eq!(MC_GET_WATT_HOURS_CHARGED.get(), 1);
        assert_eq!(MC_GET_BATTERY_LEVEL.get(), 1);
        assert_eq!(MC_GET_ODOMETER.get(), 1);
        assert_eq!(MC_GET_FAULT.get(), 1);
        assert_eq!(MC_GET_INPUT_VOLTAGE_FILTERED.get(), 1);
    });
}

#[test]
fn generated_vesc_if_inventory_matches_pinned_upstream_header() {
    assert_eq!(
        VescIfAbi::SOURCE_REPOSITORY,
        "https://github.com/lukash/vesc_pkg_lib"
    );
    assert_eq!(
        VescIfAbi::SOURCE_COMMIT,
        "e8bdc8296b90a266713da3762868f0d18ec027fe"
    );
    assert_eq!(
        VescIfAbi::SOURCE_HEADER,
        "third_party/vesc_pkg_lib/vesc_c_if.h"
    );
    assert_eq!(c_vesc_if::FIELD_COUNT, 253);
    assert_eq!(VescIfAbi::FIELD_COUNT, c_vesc_if::FIELD_COUNT);

    assert_eq!(c_vesc_if::lbm_add_extension::INDEX, 0);
    assert_eq!(c_vesc_if::lbm_add_extension::VESC32_BYTE_OFFSET, 0);
    assert_eq!(c_vesc_if::lbm_add_extension::HEADER_LINE, 325);
    assert_eq!(c_vesc_if::send_app_data::INDEX, 148);
    assert_eq!(c_vesc_if::set_app_data_handler::INDEX, 149);
    assert_eq!(c_vesc_if::mc_get_fault::INDEX, 92);
    assert_eq!(c_vesc_if::system_time_ticks::INDEX, 238);
    assert_eq!(c_vesc_if::shutdown_disable::INDEX, 252);
    assert_eq!(c_vesc_if::shutdown_disable::HEADER_LINE, 672);

    assert_eq!(c_vesc_if::SLOTS[0].name, c_vesc_if::lbm_add_extension::NAME);
    assert_eq!(
        c_vesc_if::SLOTS[c_vesc_if::FIELD_COUNT - 1].name,
        c_vesc_if::shutdown_disable::NAME
    );
    assert_eq!(
        c_vesc_if::SLOTS[c_vesc_if::FIELD_COUNT - 1].vesc32_byte_offset,
        c_vesc_if::shutdown_disable::VESC32_BYTE_OFFSET
    );
}

#[test]
fn public_vesc_if_slots_are_projected_from_generated_inventory() {
    for slot in VescIfAbi::USED_SLOTS {
        let generated = c_vesc_if::SLOTS
            .iter()
            .find(|generated| generated.name == slot.name())
            .expect("used slot must exist in generated upstream inventory");

        assert_eq!(generated.index, slot.slot_index());
        assert_eq!(generated.vesc32_byte_offset, slot.vesc32_byte_offset());
    }
}

#[test]
fn vesc_if_abi_gpio_offsets_match_struct_layout() {
    let pointer_size = core::mem::size_of::<usize>();
    let vesc32 = |field_offset: usize| (field_offset / pointer_size) * 4;

    assert_eq!(
        VescIfAbi::IO_SET_MODE.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, io_set_mode))
    );
    assert_eq!(
        VescIfAbi::IO_WRITE.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, io_write))
    );
    assert_eq!(
        VescIfAbi::IO_READ.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, io_read))
    );
    assert_eq!(
        VescIfAbi::IO_READ_ANALOG.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, io_read_analog))
    );
}

#[test]
fn vesc_if_abi_custom_config_offsets_match_struct_layout() {
    // Refloat v1.2.1 uses `src/main.c:2456` and `src/main.c:2403`; the matching
    // ABI slots are declared in `vesc_pkg_lib/vesc_c_if.h:549-553`.
    let pointer_size = core::mem::size_of::<usize>();
    let vesc32 = |field_offset: usize| (field_offset / pointer_size) * 4;

    assert_eq!(
        VescIfAbi::CONF_CUSTOM_ADD_CONFIG.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, conf_custom_add_config))
    );
    assert_eq!(
        VescIfAbi::CONF_CUSTOM_CLEAR_CONFIGS.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, conf_custom_clear_configs))
    );
}

#[test]
fn vesc_if_abi_motor_data_offsets_match_struct_layout() {
    let pointer_size = core::mem::size_of::<usize>();
    let vesc32 = |field_offset: usize| (field_offset / pointer_size) * 4;

    assert_eq!(
        VescIfAbi::MC_GET_AMP_HOURS.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_amp_hours))
    );
    assert_eq!(
        VescIfAbi::MC_GET_AMP_HOURS_CHARGED.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_amp_hours_charged))
    );
    assert_eq!(
        VescIfAbi::MC_GET_WATT_HOURS.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_watt_hours))
    );
    assert_eq!(
        VescIfAbi::MC_GET_WATT_HOURS_CHARGED.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_watt_hours_charged))
    );
    assert_eq!(
        VescIfAbi::MC_GET_DISTANCE_ABS.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_distance_abs))
    );
    assert_eq!(
        VescIfAbi::MC_TEMP_FET_FILTERED.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_temp_fet_filtered))
    );
    assert_eq!(
        VescIfAbi::MC_TEMP_MOTOR_FILTERED.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_temp_motor_filtered))
    );
    assert_eq!(
        VescIfAbi::MC_GET_BATTERY_LEVEL.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_battery_level))
    );
    assert_eq!(
        VescIfAbi::MC_GET_ODOMETER.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_odometer))
    );
    assert_eq!(
        VescIfAbi::MC_GET_FAULT.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_fault))
    );
    assert_eq!(
        VescIfAbi::MC_GET_INPUT_VOLTAGE_FILTERED.vesc32_byte_offset(),
        vesc32(core::mem::offset_of!(VescIf, mc_get_input_voltage_filtered))
    );
}
