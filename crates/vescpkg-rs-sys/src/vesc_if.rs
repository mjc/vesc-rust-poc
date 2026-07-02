//! Documented VESC firmware function-table slots used by Rust packages.

use crate::image::NativeAddress;

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
    /// Slot for `lbm_add_extension`.
    pub const LBM_ADD_EXTENSION: VescIfSlot = VescIfSlot::new("lbm_add_extension", 0);
    /// Slot for `lbm_enc_i`.
    pub const LBM_ENC_I: VescIfSlot = VescIfSlot::new("lbm_enc_i", 64);
    /// Slot for `lbm_dec_as_i32`.
    pub const LBM_DEC_AS_I32: VescIfSlot = VescIfSlot::new("lbm_dec_as_i32", 100);
    /// Slot for `lbm_is_number`.
    pub const LBM_IS_NUMBER: VescIfSlot = VescIfSlot::new("lbm_is_number", 124);
    /// Slot for `lbm_enc_sym_eerror`.
    pub const LBM_ENC_SYM_EERROR: VescIfSlot = VescIfSlot::new("lbm_enc_sym_eerror", 148);
    /// Slot for `malloc`.
    pub const MALLOC: VescIfSlot = VescIfSlot::new("malloc", 184);
    /// Slot for `free`.
    pub const FREE: VescIfSlot = VescIfSlot::new("free", 188);
    /// Slot for `get_arg`.
    pub const GET_ARG: VescIfSlot = VescIfSlot::new("get_arg", 204);
    /// Slot for `mc_get_amp_hours`.
    pub const MC_GET_AMP_HOURS: VescIfSlot = VescIfSlot::new("mc_get_amp_hours", 440);
    /// Slot for `mc_get_amp_hours_charged`.
    pub const MC_GET_AMP_HOURS_CHARGED: VescIfSlot =
        VescIfSlot::new("mc_get_amp_hours_charged", 444);
    /// Slot for `mc_get_watt_hours`.
    pub const MC_GET_WATT_HOURS: VescIfSlot = VescIfSlot::new("mc_get_watt_hours", 448);
    /// Slot for `mc_get_watt_hours_charged`.
    pub const MC_GET_WATT_HOURS_CHARGED: VescIfSlot =
        VescIfSlot::new("mc_get_watt_hours_charged", 452);
    /// Slot for `mc_temp_fet_filtered`.
    pub const MC_TEMP_FET_FILTERED: VescIfSlot = VescIfSlot::new("mc_temp_fet_filtered", 504);
    /// Slot for `mc_temp_motor_filtered`.
    pub const MC_TEMP_MOTOR_FILTERED: VescIfSlot = VescIfSlot::new("mc_temp_motor_filtered", 508);
    /// Slot for `mc_get_battery_level`.
    pub const MC_GET_BATTERY_LEVEL: VescIfSlot = VescIfSlot::new("mc_get_battery_level", 512);
    /// Slot for `mc_get_distance_abs`.
    pub const MC_GET_DISTANCE_ABS: VescIfSlot = VescIfSlot::new("mc_get_distance_abs", 524);
    /// Slot for `mc_get_odometer`.
    pub const MC_GET_ODOMETER: VescIfSlot = VescIfSlot::new("mc_get_odometer", 528);
    /// Slot for `send_app_data`.
    pub const SEND_APP_DATA: VescIfSlot = VescIfSlot::new("send_app_data", 592);
    /// Slot for `set_app_data_handler`.
    pub const SET_APP_DATA_HANDLER: VescIfSlot = VescIfSlot::new("set_app_data_handler", 596);
    /// Slot for `system_time_ticks`.
    pub const SYSTEM_TIME_TICKS: VescIfSlot = VescIfSlot::new("system_time_ticks", 952);
    /// Slot for `io_set_mode`.
    pub const IO_SET_MODE: VescIfSlot = VescIfSlot::new("io_set_mode", 220);
    /// Slot for `io_write`.
    pub const IO_WRITE: VescIfSlot = VescIfSlot::new("io_write", 224);
    /// Slot for `io_read`.
    pub const IO_READ: VescIfSlot = VescIfSlot::new("io_read", 228);

    /// The set of slots that this crate currently relies on.
    pub const USED_SLOTS: [VescIfSlot; 23] = [
        Self::LBM_ADD_EXTENSION,
        Self::LBM_ENC_I,
        Self::LBM_DEC_AS_I32,
        Self::LBM_IS_NUMBER,
        Self::LBM_ENC_SYM_EERROR,
        Self::MALLOC,
        Self::FREE,
        Self::GET_ARG,
        Self::MC_GET_AMP_HOURS,
        Self::MC_GET_AMP_HOURS_CHARGED,
        Self::MC_GET_WATT_HOURS,
        Self::MC_GET_WATT_HOURS_CHARGED,
        Self::MC_TEMP_FET_FILTERED,
        Self::MC_TEMP_MOTOR_FILTERED,
        Self::MC_GET_BATTERY_LEVEL,
        Self::MC_GET_DISTANCE_ABS,
        Self::MC_GET_ODOMETER,
        Self::SEND_APP_DATA,
        Self::SET_APP_DATA_HANDLER,
        Self::SYSTEM_TIME_TICKS,
        Self::IO_SET_MODE,
        Self::IO_WRITE,
        Self::IO_READ,
    ];
}
