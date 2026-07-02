use core::cell::Cell;
use core::ffi::{c_char, c_int, c_uchar};

use crate::test_support::{empty_table, with_table};
use crate::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi, VescPin, VescPinMode};

use super::{
    VescIf, io_read, io_set_mode, io_write, lbm_add_extension, lbm_add_extension_with_table_base,
    lbm_dec_as_i32, lbm_enc_i, lbm_enc_sym_eerror, lbm_is_number, mc_get_amp_hours,
    mc_get_amp_hours_charged, mc_get_battery_level, mc_get_distance_abs, mc_get_fault,
    mc_get_input_voltage_filtered, mc_get_odometer, mc_get_watt_hours, mc_get_watt_hours_charged,
    mc_temp_fet_filtered, mc_temp_motor_filtered, vesc_clear_app_data_handler, vesc_send_app_data,
    vesc_set_app_data_handler, vesc_system_time_ticks,
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
static LBM_ENC_I: SyncCounter = SyncCounter::new();
static LBM_IS_NUMBER: SyncCounter = SyncCounter::new();
static SET_APP_DATA_HANDLER: SyncCounter = SyncCounter::new();
static SEND_APP_DATA: SyncCounter = SyncCounter::new();
static SEND_APP_DATA_LEN: SyncU32 = SyncU32::new();
static SYSTEM_TIME_TICKS: SyncCounter = SyncCounter::new();
static IO_SET_MODE: SyncCounter = SyncCounter::new();
static IO_WRITE: SyncCounter = SyncCounter::new();
static IO_READ: SyncCounter = SyncCounter::new();
static MC_GET_DISTANCE_ABS: SyncCounter = SyncCounter::new();
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
static LAST_PIN: SyncI32 = SyncI32::new();
static LAST_MODE: SyncI32 = SyncI32::new();
static LAST_LEVEL: SyncI32 = SyncI32::new();
static LAST_LBM_VALUE: SyncU32 = SyncU32::new();
static LAST_HANDLER_INSTALLED: SyncBool = SyncBool::new();

fn reset_counters() {
    for counter in [
        &LBM_ADD_EXTENSION,
        &LBM_DEC_AS_I32,
        &LBM_ENC_I,
        &LBM_IS_NUMBER,
        &SET_APP_DATA_HANDLER,
        &SEND_APP_DATA,
        &SYSTEM_TIME_TICKS,
        &IO_SET_MODE,
        &IO_WRITE,
        &IO_READ,
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
    ] {
        counter.set(0);
    }
    SEND_APP_DATA_LEN.set(0);
    LAST_PIN.set(0);
    LAST_MODE.set(0);
    LAST_LEVEL.set(0);
    LAST_LBM_VALUE.set(0);
    LAST_HANDLER_INSTALLED.set(false);
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

extern "C" fn stub_lbm_enc_i(value: i32) -> u32 {
    LBM_ENC_I.inc();
    value as u32 + 1
}

extern "C" fn stub_lbm_is_number(value: u32) -> bool {
    LBM_IS_NUMBER.inc();
    LAST_LBM_VALUE.set(value);
    value == 7
}

extern "C" fn stub_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
    SET_APP_DATA_HANDLER.inc();
    LAST_HANDLER_INSTALLED.set(handler.is_some());
    true
}

extern "C" fn stub_send_app_data(_data: *mut c_uchar, len: u32) {
    SEND_APP_DATA.inc();
    SEND_APP_DATA_LEN.set(len);
}

extern "C" fn stub_system_time_ticks() -> u32 {
    SYSTEM_TIME_TICKS.inc();
    42
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

extern "C" fn stub_mc_get_distance_abs() -> f32 {
    MC_GET_DISTANCE_ABS.inc();
    12.5
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

fn populated_table() -> VescIf {
    let mut table = empty_table();
    table.lbm_add_extension = Some(stub_lbm_add_extension);
    table.lbm_dec_as_i32 = Some(stub_lbm_dec_as_i32);
    table.lbm_enc_i = Some(stub_lbm_enc_i);
    table.lbm_is_number = Some(stub_lbm_is_number);
    table.lbm_enc_sym_eerror = 0xAABB_CC00;
    table.set_app_data_handler = Some(stub_set_app_data_handler);
    table.send_app_data = Some(stub_send_app_data);
    table.system_time_ticks = Some(stub_system_time_ticks);
    table.io_set_mode = Some(stub_io_set_mode);
    table.io_write = Some(stub_io_write);
    table.io_read = Some(stub_io_read);
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

fn with_empty_table<R>(body: impl FnOnce() -> R) -> R {
    let table = empty_table();
    with_table(&table, || {
        reset_counters();
        body()
    })
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
fn lbm_value_helpers_forward_and_handle_missing_slots() {
    with_populated_table(|| unsafe {
        assert_eq!(lbm_dec_as_i32(LbmValue(9)), 9);
        assert_eq!(lbm_enc_i(4), LbmValue(5));
        assert!(lbm_is_number(LbmValue(7)));
        assert!(!lbm_is_number(LbmValue(8)));
        assert_eq!(lbm_enc_sym_eerror(), LbmValue(0xAABB_CC00));
    });

    with_empty_table(|| unsafe {
        assert_eq!(lbm_dec_as_i32(LbmValue(1)), 0);
        assert_eq!(lbm_enc_i(1), LbmValue(0));
        assert!(!lbm_is_number(LbmValue(1)));
        assert_eq!(lbm_enc_sym_eerror(), LbmValue(0));
    });
}

#[test]
fn app_data_helpers_forward_and_handle_missing_slots() {
    with_populated_table(|| unsafe {
        extern "C" fn handler(_: *mut u8, _: u32) {}

        assert!(vesc_set_app_data_handler(handler));
        assert!(LAST_HANDLER_INSTALLED.get());
        assert!(vesc_clear_app_data_handler());
        assert!(!LAST_HANDLER_INSTALLED.get());

        let payload = [1_u8, 2, 3];
        vesc_send_app_data(payload.as_ptr(), payload.len() as u32);
        assert_eq!(SEND_APP_DATA.get(), 1);
        assert_eq!(SEND_APP_DATA_LEN.get(), 3);
    });

    with_empty_table(|| unsafe {
        assert!(!vesc_clear_app_data_handler());
        vesc_send_app_data(core::ptr::null(), 0);
        assert_eq!(SEND_APP_DATA.get(), 0);
    });
}

#[test]
fn system_time_ticks_forwards_and_defaults_to_zero() {
    with_populated_table(|| unsafe {
        assert_eq!(vesc_system_time_ticks(), 42);
    });

    with_empty_table(|| unsafe {
        assert_eq!(vesc_system_time_ticks(), 0);
    });
}

#[test]
fn gpio_helpers_forward_and_handle_missing_slots() {
    with_populated_table(|| unsafe {
        let pin = VescPin(3);
        let mode = VescPinMode(2);

        assert!(io_set_mode(pin, mode));
        assert!(io_write(pin, 1));
        assert!(io_read(pin));
        assert_eq!(LAST_PIN.get(), 3);
        assert_eq!(LAST_MODE.get(), 2);
        assert_eq!(LAST_LEVEL.get(), 1);
    });

    with_empty_table(|| unsafe {
        let pin = VescPin(1);
        assert!(!io_set_mode(pin, VescPinMode(0)));
        assert!(!io_write(pin, 0));
        assert!(!io_read(pin));
    });
}

#[test]
fn motor_data_helpers_forward_and_handle_missing_slots() {
    with_populated_table(|| unsafe {
        assert_eq!(mc_get_distance_abs(), 12.5);
        assert_eq!(mc_temp_fet_filtered(), 44.0);
        assert_eq!(mc_temp_motor_filtered(), 51.5);
        assert_eq!(mc_get_amp_hours(false), 3.2);
        assert_eq!(mc_get_amp_hours(true), -1.0);
        assert_eq!(mc_get_amp_hours_charged(false), 0.8);
        assert_eq!(mc_get_amp_hours_charged(true), -1.0);
        assert_eq!(mc_get_watt_hours(false), 170.0);
        assert_eq!(mc_get_watt_hours(true), -1.0);
        assert_eq!(mc_get_watt_hours_charged(false), 18.5);
        assert_eq!(mc_get_watt_hours_charged(true), -1.0);
        assert_eq!(mc_get_battery_level(core::ptr::null_mut()), 0.72);
        let mut wh_left = 0.0_f32;
        assert_eq!(mc_get_battery_level(&raw mut wh_left), 0.72);
        assert_eq!(wh_left, 42.0);
        assert_eq!(mc_get_odometer(), 123_456);
        assert_eq!(mc_get_fault(), 5);
        assert_eq!(mc_get_input_voltage_filtered(), 84.2);
        assert_eq!(MC_GET_DISTANCE_ABS.get(), 1);
        assert_eq!(MC_TEMP_FET_FILTERED.get(), 1);
        assert_eq!(MC_TEMP_MOTOR_FILTERED.get(), 1);
        assert_eq!(MC_GET_AMP_HOURS.get(), 2);
        assert_eq!(MC_GET_AMP_HOURS_CHARGED.get(), 2);
        assert_eq!(MC_GET_WATT_HOURS.get(), 2);
        assert_eq!(MC_GET_WATT_HOURS_CHARGED.get(), 2);
        assert_eq!(MC_GET_BATTERY_LEVEL.get(), 2);
        assert_eq!(MC_GET_ODOMETER.get(), 1);
        assert_eq!(MC_GET_FAULT.get(), 1);
        assert_eq!(MC_GET_INPUT_VOLTAGE_FILTERED.get(), 1);
    });

    with_empty_table(|| unsafe {
        assert_eq!(mc_get_distance_abs(), 0.0);
        assert_eq!(mc_temp_fet_filtered(), 0.0);
        assert_eq!(mc_temp_motor_filtered(), 0.0);
        assert_eq!(mc_get_amp_hours(false), 0.0);
        assert_eq!(mc_get_amp_hours_charged(false), 0.0);
        assert_eq!(mc_get_watt_hours(false), 0.0);
        assert_eq!(mc_get_watt_hours_charged(false), 0.0);
        assert_eq!(mc_get_battery_level(core::ptr::null_mut()), 0.0);
        let mut wh_left = 7.0_f32;
        assert_eq!(mc_get_battery_level(&raw mut wh_left), 0.0);
        assert_eq!(wh_left, 7.0);
        assert_eq!(mc_get_odometer(), 0);
        assert_eq!(mc_get_fault(), 0);
        assert_eq!(mc_get_input_voltage_filtered(), 0.0);
        assert_eq!(MC_GET_DISTANCE_ABS.get(), 0);
        assert_eq!(MC_TEMP_FET_FILTERED.get(), 0);
        assert_eq!(MC_TEMP_MOTOR_FILTERED.get(), 0);
        assert_eq!(MC_GET_AMP_HOURS.get(), 0);
        assert_eq!(MC_GET_AMP_HOURS_CHARGED.get(), 0);
        assert_eq!(MC_GET_WATT_HOURS.get(), 0);
        assert_eq!(MC_GET_WATT_HOURS_CHARGED.get(), 0);
        assert_eq!(MC_GET_BATTERY_LEVEL.get(), 0);
        assert_eq!(MC_GET_ODOMETER.get(), 0);
        assert_eq!(MC_GET_FAULT.get(), 0);
        assert_eq!(MC_GET_INPUT_VOLTAGE_FILTERED.get(), 0);
    });
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
