//! Raw/minimal VESC firmware ABI bindings.
//!
//! This crate mirrors the VESC native package ABI. It does not provide
//! high-level vehicle semantics, package building, or host transport code.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.
//!
//! Testing strategy: see `docs/testing/vescpkg-rs-sys.md`.

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

#[cfg(test)]
extern crate std;

mod image;
macro_rules! count_idents {
    ($($ident:ident),* $(,)?) => {
        0usize $(+ count_idents!(@one $ident))*
    };
    (@one $ident:ident) => {
        1usize
    };
}

macro_rules! vesc_if_used_slots {
    ($macro:ident) => {
        $macro! {
            LBM_ADD_EXTENSION => lbm_add_extension,
            LBM_DEC_AS_FLOAT => lbm_dec_as_float,
            LBM_DEC_AS_I32 => lbm_dec_as_i32,
            LBM_ENC_I => lbm_enc_i,
            LBM_IS_NUMBER => lbm_is_number,
            SET_APP_DATA_HANDLER => set_app_data_handler,
            IMU_SET_READ_CALLBACK => imu_set_read_callback,
            LBM_ENC_SYM_NIL => lbm_enc_sym_nil,
            LBM_ENC_SYM_TRUE => lbm_enc_sym_true,
            LBM_ENC_SYM_EERROR => lbm_enc_sym_eerror,
            CONF_CUSTOM_ADD_CONFIG => conf_custom_add_config,
            CONF_CUSTOM_CLEAR_CONFIGS => conf_custom_clear_configs,
            MUTEX_CREATE => mutex_create,
            MUTEX_LOCK => mutex_lock,
            MUTEX_UNLOCK => mutex_unlock,
            MALLOC => malloc,
            FREE => free,
            SLEEP_US => sleep_us,
            SPAWN => spawn,
            REQUEST_TERMINATE => request_terminate,
            SHOULD_TERMINATE => should_terminate,
            GET_ARG => get_arg,
            MC_GET_FAULT => mc_get_fault,
            MC_GET_RPM => mc_get_rpm,
            MC_GET_SPEED => mc_get_speed,
            MC_GET_TOT_CURRENT_FILTERED => mc_get_tot_current_filtered,
            MC_GET_TOT_CURRENT_DIRECTIONAL_FILTERED => mc_get_tot_current_directional_filtered,
            MC_GET_TOT_CURRENT_IN_FILTERED => mc_get_tot_current_in_filtered,
            MC_GET_DUTY_CYCLE_NOW => mc_get_duty_cycle_now,
            MC_GET_INPUT_VOLTAGE_FILTERED => mc_get_input_voltage_filtered,
            MC_GET_AMP_HOURS => mc_get_amp_hours,
            MC_GET_AMP_HOURS_CHARGED => mc_get_amp_hours_charged,
            MC_GET_WATT_HOURS => mc_get_watt_hours,
            MC_GET_WATT_HOURS_CHARGED => mc_get_watt_hours_charged,
            MC_GET_BATTERY_LEVEL => mc_get_battery_level,
            MC_GET_DISTANCE_ABS => mc_get_distance_abs,
            MC_GET_ODOMETER => mc_get_odometer,
            GET_CFG_FLOAT => get_cfg_float,
            GET_CFG_INT => get_cfg_int,
            MC_SET_DUTY => mc_set_duty,
            MC_SET_CURRENT => mc_set_current,
            MC_SET_CURRENT_OFF_DELAY => mc_set_current_off_delay,
            MC_SET_BRAKE_CURRENT => mc_set_brake_current,
            TIMEOUT_RESET => timeout_reset,
            FOC_GET_ID => foc_get_id,
            MC_TEMP_FET_FILTERED => mc_temp_fet_filtered,
            MC_TEMP_MOTOR_FILTERED => mc_temp_motor_filtered,
            IMU_STARTUP_DONE => imu_startup_done,
            IMU_GET_ROLL => imu_get_roll,
            IMU_GET_PITCH => imu_get_pitch,
            IMU_GET_YAW => imu_get_yaw,
            IMU_GET_GYRO => imu_get_gyro,
            IMU_GET_QUATERNIONS => imu_get_quaternions,
            SEND_APP_DATA => send_app_data,
            SYSTEM_TIME => system_time,
            SYSTEM_TIME_TICKS => system_time_ticks,
            THREAD_SET_PRIORITY => thread_set_priority,
            IO_SET_MODE => io_set_mode,
            IO_WRITE => io_write,
            IO_READ => io_read,
            IO_READ_ANALOG => io_read_analog,
        }
    };
}

#[allow(dead_code)]
mod c_vesc_if {
    include!(concat!(env!("OUT_DIR"), "/c_vesc_if.rs"));
}
mod loader;
mod types;
mod vesc_if;

#[cfg(test)]
pub mod test_support;

/// Raw firmware layout mirrors used when host code needs to inspect payloads directly.
pub mod raw;
/// Typed borrowed views over raw firmware packet bytes.
pub mod views;

pub use image::{ImageOffset, NativeAddress, NativeImage};
pub use loader::{AppDataHandler, ExtensionHandler, LibInfo, LibInfoAbi, StopHandler};
pub use types::*;
pub use vesc_if::{VescIfAbi, VescIfSlot};
pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[cfg(test)]
mod tests;
