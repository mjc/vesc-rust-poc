//! Documented VESC firmware function-table slots used by Rust packages.

use crate::{c_vesc_if, image::NativeAddress};

/// One entry in the VESC firmware function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
}

impl VescIfSlot {
    /// Create a named slot at the given 32-bit byte offset.
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self { name, offset }
    }

    /// Return the firmware symbol name for this slot.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Return the 32-bit firmware byte offset for this slot.
    pub const fn vesc32_byte_offset(self) -> usize {
        self.offset
    }

    /// Return the slot index in the 32-bit function table.
    pub const fn slot_index(self) -> usize {
        self.offset / 4
    }

    /// Convert the 32-bit slot offset into a host-byte offset.
    pub const fn host_byte_offset(self, pointer_size: usize) -> usize {
        self.slot_index() * pointer_size
    }
}

/// ABI anchor and slot metadata for the VESC firmware function table.
pub struct VescIfAbi;

impl VescIfAbi {
    /// Base address of the firmware function table on VESC targets.
    pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
    /// Number of entries in the pinned upstream `vesc_c_if` table.
    pub const FIELD_COUNT: usize = c_vesc_if::FIELD_COUNT;
    /// Slot for `lbm_add_extension`.
    pub const LBM_ADD_EXTENSION: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_add_extension::NAME,
        c_vesc_if::lbm_add_extension::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_enc_i`.
    pub const LBM_ENC_I: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_enc_i::NAME,
        c_vesc_if::lbm_enc_i::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_dec_as_i32`.
    pub const LBM_DEC_AS_I32: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_dec_as_i32::NAME,
        c_vesc_if::lbm_dec_as_i32::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_is_number`.
    pub const LBM_IS_NUMBER: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_is_number::NAME,
        c_vesc_if::lbm_is_number::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_enc_sym_nil`.
    pub const LBM_ENC_SYM_NIL: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_enc_sym_nil::NAME,
        c_vesc_if::lbm_enc_sym_nil::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_enc_sym_true`.
    pub const LBM_ENC_SYM_TRUE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_enc_sym_true::NAME,
        c_vesc_if::lbm_enc_sym_true::VESC32_BYTE_OFFSET,
    );
    /// Slot for `lbm_enc_sym_eerror`.
    pub const LBM_ENC_SYM_EERROR: VescIfSlot = VescIfSlot::new(
        c_vesc_if::lbm_enc_sym_eerror::NAME,
        c_vesc_if::lbm_enc_sym_eerror::VESC32_BYTE_OFFSET,
    );
    /// Slot for `malloc`.
    pub const MALLOC: VescIfSlot = VescIfSlot::new(
        c_vesc_if::malloc::NAME,
        c_vesc_if::malloc::VESC32_BYTE_OFFSET,
    );
    /// Slot for `free`.
    pub const FREE: VescIfSlot =
        VescIfSlot::new(c_vesc_if::free::NAME, c_vesc_if::free::VESC32_BYTE_OFFSET);
    /// Slot for `spawn`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:382`.
    pub const SPAWN: VescIfSlot =
        VescIfSlot::new(c_vesc_if::spawn::NAME, c_vesc_if::spawn::VESC32_BYTE_OFFSET);
    /// Slot for `request_terminate`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:383`.
    pub const REQUEST_TERMINATE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::request_terminate::NAME,
        c_vesc_if::request_terminate::VESC32_BYTE_OFFSET,
    );
    /// Slot for `should_terminate`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:384`.
    pub const SHOULD_TERMINATE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::should_terminate::NAME,
        c_vesc_if::should_terminate::VESC32_BYTE_OFFSET,
    );
    /// Slot for `get_arg`.
    pub const GET_ARG: VescIfSlot = VescIfSlot::new(
        c_vesc_if::get_arg::NAME,
        c_vesc_if::get_arg::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_fault`.
    pub const MC_GET_FAULT: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_fault::NAME,
        c_vesc_if::mc_get_fault::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_amp_hours`.
    pub const MC_GET_AMP_HOURS: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_amp_hours::NAME,
        c_vesc_if::mc_get_amp_hours::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_amp_hours_charged`.
    pub const MC_GET_AMP_HOURS_CHARGED: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_amp_hours_charged::NAME,
        c_vesc_if::mc_get_amp_hours_charged::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_watt_hours`.
    pub const MC_GET_WATT_HOURS: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_watt_hours::NAME,
        c_vesc_if::mc_get_watt_hours::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_watt_hours_charged`.
    pub const MC_GET_WATT_HOURS_CHARGED: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_watt_hours_charged::NAME,
        c_vesc_if::mc_get_watt_hours_charged::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_input_voltage_filtered`.
    pub const MC_GET_INPUT_VOLTAGE_FILTERED: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_input_voltage_filtered::NAME,
        c_vesc_if::mc_get_input_voltage_filtered::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_temp_fet_filtered`.
    pub const MC_TEMP_FET_FILTERED: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_temp_fet_filtered::NAME,
        c_vesc_if::mc_temp_fet_filtered::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_temp_motor_filtered`.
    pub const MC_TEMP_MOTOR_FILTERED: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_temp_motor_filtered::NAME,
        c_vesc_if::mc_temp_motor_filtered::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_battery_level`.
    pub const MC_GET_BATTERY_LEVEL: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_battery_level::NAME,
        c_vesc_if::mc_get_battery_level::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_distance_abs`.
    pub const MC_GET_DISTANCE_ABS: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_distance_abs::NAME,
        c_vesc_if::mc_get_distance_abs::VESC32_BYTE_OFFSET,
    );
    /// Slot for `mc_get_odometer`.
    pub const MC_GET_ODOMETER: VescIfSlot = VescIfSlot::new(
        c_vesc_if::mc_get_odometer::NAME,
        c_vesc_if::mc_get_odometer::VESC32_BYTE_OFFSET,
    );
    /// Slot for `send_app_data`.
    pub const SEND_APP_DATA: VescIfSlot = VescIfSlot::new(
        c_vesc_if::send_app_data::NAME,
        c_vesc_if::send_app_data::VESC32_BYTE_OFFSET,
    );
    /// Slot for `set_app_data_handler`.
    pub const SET_APP_DATA_HANDLER: VescIfSlot = VescIfSlot::new(
        c_vesc_if::set_app_data_handler::NAME,
        c_vesc_if::set_app_data_handler::VESC32_BYTE_OFFSET,
    );
    /// Slot for `imu_startup_done`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:510`.
    pub const IMU_STARTUP_DONE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::imu_startup_done::NAME,
        c_vesc_if::imu_startup_done::VESC32_BYTE_OFFSET,
    );
    /// Slot for `imu_get_roll`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:511`.
    pub const IMU_GET_ROLL: VescIfSlot = VescIfSlot::new(
        c_vesc_if::imu_get_roll::NAME,
        c_vesc_if::imu_get_roll::VESC32_BYTE_OFFSET,
    );
    /// Slot for `imu_get_pitch`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:512`.
    pub const IMU_GET_PITCH: VescIfSlot = VescIfSlot::new(
        c_vesc_if::imu_get_pitch::NAME,
        c_vesc_if::imu_get_pitch::VESC32_BYTE_OFFSET,
    );
    /// Slot for `imu_get_yaw`, declared in Refloat v1.2.1
    /// `vesc_pkg_lib/vesc_c_if.h:513`.
    pub const IMU_GET_YAW: VescIfSlot = VescIfSlot::new(
        c_vesc_if::imu_get_yaw::NAME,
        c_vesc_if::imu_get_yaw::VESC32_BYTE_OFFSET,
    );
    /// Slot for Refloat/VESC Tool custom config registration.
    ///
    /// Refloat `v1.2.1` registers `get_cfg`/`set_cfg`/`get_cfg_xml` through this
    /// slot in `src/main.c:2456`; the slot is declared in
    /// `vesc_pkg_lib/vesc_c_if.h:549-552`.
    pub const CONF_CUSTOM_ADD_CONFIG: VescIfSlot = VescIfSlot::new(
        c_vesc_if::conf_custom_add_config::NAME,
        c_vesc_if::conf_custom_add_config::VESC32_BYTE_OFFSET,
    );
    /// Slot for clearing package custom config callbacks during stop.
    ///
    /// Refloat `v1.2.1` clears this slot in `src/main.c:2403`; the slot is
    /// declared in `vesc_pkg_lib/vesc_c_if.h:553`.
    pub const CONF_CUSTOM_CLEAR_CONFIGS: VescIfSlot = VescIfSlot::new(
        c_vesc_if::conf_custom_clear_configs::NAME,
        c_vesc_if::conf_custom_clear_configs::VESC32_BYTE_OFFSET,
    );
    /// Slot for `system_time_ticks`.
    pub const SYSTEM_TIME_TICKS: VescIfSlot = VescIfSlot::new(
        c_vesc_if::system_time_ticks::NAME,
        c_vesc_if::system_time_ticks::VESC32_BYTE_OFFSET,
    );
    /// Slot for `io_set_mode`.
    pub const IO_SET_MODE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::io_set_mode::NAME,
        c_vesc_if::io_set_mode::VESC32_BYTE_OFFSET,
    );
    /// Slot for `io_write`.
    pub const IO_WRITE: VescIfSlot = VescIfSlot::new(
        c_vesc_if::io_write::NAME,
        c_vesc_if::io_write::VESC32_BYTE_OFFSET,
    );
    /// Slot for `io_read`.
    pub const IO_READ: VescIfSlot = VescIfSlot::new(
        c_vesc_if::io_read::NAME,
        c_vesc_if::io_read::VESC32_BYTE_OFFSET,
    );

    /// The set of slots that this crate currently relies on.
    pub const USED_SLOTS: [VescIfSlot; 36] = [
        Self::LBM_ADD_EXTENSION,
        Self::LBM_ENC_I,
        Self::LBM_DEC_AS_I32,
        Self::LBM_IS_NUMBER,
        Self::LBM_ENC_SYM_NIL,
        Self::LBM_ENC_SYM_TRUE,
        Self::LBM_ENC_SYM_EERROR,
        Self::MALLOC,
        Self::FREE,
        Self::SPAWN,
        Self::REQUEST_TERMINATE,
        Self::SHOULD_TERMINATE,
        Self::GET_ARG,
        Self::MC_GET_FAULT,
        Self::MC_GET_AMP_HOURS,
        Self::MC_GET_AMP_HOURS_CHARGED,
        Self::MC_GET_WATT_HOURS,
        Self::MC_GET_WATT_HOURS_CHARGED,
        Self::MC_GET_INPUT_VOLTAGE_FILTERED,
        Self::MC_TEMP_FET_FILTERED,
        Self::MC_TEMP_MOTOR_FILTERED,
        Self::MC_GET_BATTERY_LEVEL,
        Self::MC_GET_DISTANCE_ABS,
        Self::MC_GET_ODOMETER,
        Self::SEND_APP_DATA,
        Self::SET_APP_DATA_HANDLER,
        Self::IMU_STARTUP_DONE,
        Self::IMU_GET_ROLL,
        Self::IMU_GET_PITCH,
        Self::IMU_GET_YAW,
        Self::CONF_CUSTOM_ADD_CONFIG,
        Self::CONF_CUSTOM_CLEAR_CONFIGS,
        Self::SYSTEM_TIME_TICKS,
        Self::IO_SET_MODE,
        Self::IO_WRITE,
        Self::IO_READ,
    ];
}
