//! Documented VESC firmware function-table slots used by Rust packages.

use crate::image::NativeAddress;

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
    pub const IO_SET_MODE: VescIfSlot = VescIfSlot::new("io_set_mode", 220);
    pub const IO_WRITE: VescIfSlot = VescIfSlot::new("io_write", 224);
    pub const IO_READ: VescIfSlot = VescIfSlot::new("io_read", 228);

    pub const USED_SLOTS: [VescIfSlot; 11] = [
        Self::LBM_ADD_EXTENSION,
        Self::LBM_ENC_I,
        Self::LBM_DEC_AS_I32,
        Self::LBM_IS_NUMBER,
        Self::LBM_ENC_SYM_EERROR,
        Self::SEND_APP_DATA,
        Self::SET_APP_DATA_HANDLER,
        Self::SYSTEM_TIME_TICKS,
        Self::IO_SET_MODE,
        Self::IO_WRITE,
        Self::IO_READ,
    ];
}
