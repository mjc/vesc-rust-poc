//! Target-side SDK for Rust VESC packages.
//!
//! Link this crate into native VESC package code. It wraps `vescpkg-rs-sys` with
//! lifecycle, LispBM extension, app-data, GPIO, and typed firmware helpers.
//!
//! Device builds stay `no_std`; package crates must opt into the `alloc`
//! feature before installing the VESC-backed global allocator.

#![doc = include_str!("compile_fail_contracts.md")]
#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(any(
    feature = "alloc",
    test,
    all(not(target_arch = "arm"), feature = "test-support")
))]
extern crate alloc as rust_alloc;
#[cfg(test)]
extern crate std;

/// Firmware allocation helpers backed by the VESC native package allocator.
mod alloc;

mod advanced_foc;
#[cfg(feature = "math")]
mod ahrs;
mod audio;
mod bindings;
mod capabilities;
mod eeprom;
mod encoder;
mod extension;
mod firmware;
mod gnss;
mod lifecycle_core;
mod logging;
/// Float math entrypoints backed by Rust `libm` on package and host builds.
#[cfg(feature = "math")]
mod math;
mod nvm;
mod packet;
mod plot;
mod pwm;
#[cfg(feature = "math")]
pub use math::{asin, atan2, cos, sin, sqrt, tan};
mod runtime;
/// Explicitly unsafe STM32 pad/port access, separate from leased abstract GPIO.
pub mod stm32;
mod sync;
mod terminal;
#[cfg(all(feature = "test-support", not(test)))]
mod test_ffi;
mod uart;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Internal ABI seam. `vescpkg-rs-sys` selects the real or test implementation.
pub(crate) mod ffi {
    #[cfg(any(test, not(feature = "test-support")))]
    pub use vescpkg_rs_sys::raw::printf_data;
    pub use vescpkg_rs_sys::raw::{
        CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback,
    };
    #[cfg(any(test, not(feature = "test-support")))]
    pub use vescpkg_rs_sys::raw::{
        app_is_output_disabled, can_ping, can_set_current, can_set_current_brake,
        can_set_current_brake_rel, can_set_current_off_delay, can_set_current_rel,
        can_set_current_rel_off_delay, can_set_duty, can_set_eid_callback, can_set_pos,
        can_set_rpm, can_set_sid_callback, can_status_msg_2_id, can_status_msg_3_id,
        can_status_msg_4_id, can_status_msg_5_id, can_status_msg_6_id, can_status_msg_id,
        can_transmit_eid, can_transmit_sid, get_ppm, get_ppm_age, remote_state, store_backup_data,
        timeout_has_timeout, timeout_reset, timeout_secs_since_update,
    };
    #[cfg(all(not(test), not(feature = "test-support")))]
    pub use vescpkg_rs_sys::raw::{
        clear_pad, io_get_st_pin, io_read, io_read_analog, io_set_mode, io_write, set_pad,
        set_pad_mode,
    };
    #[allow(unused_imports)]
    pub use vescpkg_rs_sys::raw::{
        conf_custom_add_config, conf_custom_clear_configs, lbm_add_extension, lbm_enc_sym_eerror,
        lbm_enc_sym_nil, lbm_enc_sym_true, vesc_clear_app_data_handler,
        vesc_clear_imu_read_callback, vesc_get_arg, vesc_malloc, vesc_send_app_data,
        vesc_set_app_data_handler, vesc_set_imu_read_callback,
    };
    pub use vescpkg_rs_sys::{AppDataHandler, LibInfo, NativeImage};

    #[cfg(all(feature = "test-support", not(test)))]
    use crate::test_ffi as selected_ffi;
    #[cfg(all(feature = "test-support", not(test)))]
    pub use crate::test_ffi::{
        app_is_output_disabled, can_ping, can_set_current, can_set_current_brake,
        can_set_current_brake_rel, can_set_current_off_delay, can_set_current_rel,
        can_set_current_rel_off_delay, can_set_duty, can_set_eid_callback, can_set_pos,
        can_set_rpm, can_set_sid_callback, can_status_msg_2_id, can_status_msg_3_id,
        can_status_msg_4_id, can_status_msg_5_id, can_status_msg_6_id, can_status_msg_id,
        can_transmit_eid, can_transmit_sid, clear_pad, get_ppm, get_ppm_age, io_get_st_pin,
        io_read, io_read_analog, io_set_mode, io_write, printf_data, remote_state, set_pad,
        set_pad_mode, store_backup_data, timeout_has_timeout, timeout_reset,
        timeout_secs_since_update,
    };
    #[allow(unused_imports)]
    pub use selected_ffi::{
        ahrs_get_pitch, ahrs_get_roll, ahrs_get_yaw, ahrs_init_attitude_info,
        ahrs_update_initial_orientation, ahrs_update_madgwick_imu, ahrs_update_mahony_imu,
        commands_process_packet, commands_unregister_reply_func, encoder_set_custom_callbacks, f_b,
        f_cons, f_float, f_i, f_i32, f_i64, f_lbm_array, f_sym, f_u32, f_u64, foc_beep,
        foc_get_audio_sample_table, foc_get_id, foc_get_iq, foc_get_vd, foc_get_vq,
        foc_play_audio_samples, foc_play_tone, foc_set_audio_sample_table,
        foc_set_openloop_current, foc_set_openloop_duty, foc_set_openloop_duty_phase,
        foc_set_openloop_phase, foc_stop_audio, get_cfg_float, get_cfg_int, gnss_snapshot,
        imu_derotate, imu_get_accel, imu_get_accel_derotated, imu_get_calibration, imu_get_gyro,
        imu_get_gyro_derotated, imu_get_mag, imu_get_pitch, imu_get_roll, imu_get_rpy, imu_get_yaw,
        imu_set_read_callback, imu_set_yaw, imu_startup_done, lbm_block_ctx_from_extension,
        lbm_car, lbm_cdr, lbm_cons, lbm_continue_eval, lbm_create_byte_array, lbm_dec_as_float,
        lbm_dec_as_i32, lbm_dec_as_u32, lbm_dec_char, lbm_dec_str, lbm_dec_sym, lbm_enc_char,
        lbm_enc_float, lbm_enc_i, lbm_enc_sym, lbm_enc_u32, lbm_eval_is_paused, lbm_finish_flatten,
        lbm_get_current_cid, lbm_get_symbol_by_name, lbm_is_byte_array, lbm_is_char, lbm_is_cons,
        lbm_is_number, lbm_is_symbol, lbm_list_destructive_reverse, lbm_pause_eval_with_gc,
        lbm_send_message, lbm_set_error_reason, lbm_start_flatten, lbm_unblock_ctx,
        lbm_unblock_ctx_unboxed, mc_dccal_done, mc_fault_to_string, mc_get_amp_hours,
        mc_get_amp_hours_charged, mc_get_battery_level, mc_get_distance, mc_get_distance_abs,
        mc_get_duty_cycle_now, mc_get_fault, mc_get_input_voltage_filtered, mc_get_motor_thread,
        mc_get_odometer, mc_get_pid_pos_now, mc_get_pid_pos_set, mc_get_rpm,
        mc_get_sampling_frequency_now, mc_get_speed, mc_get_tachometer_abs_value,
        mc_get_tachometer_value, mc_get_tot_current, mc_get_tot_current_directional,
        mc_get_tot_current_directional_filtered, mc_get_tot_current_filtered,
        mc_get_tot_current_in, mc_get_tot_current_in_filtered, mc_get_watt_hours,
        mc_get_watt_hours_charged, mc_release_motor, mc_select_motor_thread, mc_set_brake_current,
        mc_set_brake_current_rel, mc_set_current, mc_set_current_off_delay, mc_set_current_rel,
        mc_set_duty, mc_set_duty_noramp, mc_set_handbrake, mc_set_handbrake_rel, mc_set_odometer,
        mc_set_pid_pos, mc_set_pid_speed, mc_set_pwm_callback, mc_stat_count_time,
        mc_stat_current_avg, mc_stat_current_max, mc_stat_power_avg, mc_stat_power_max,
        mc_stat_reset, mc_stat_speed_avg, mc_stat_speed_max, mc_stat_temp_mosfet_avg,
        mc_stat_temp_mosfet_max, mc_stat_temp_motor_avg, mc_stat_temp_motor_max,
        mc_temp_fet_filtered, mc_temp_motor_filtered, mc_update_pid_pos_offset,
        mc_wait_for_motor_release, packet_init, packet_process_byte, packet_reset,
        packet_send_packet, plot_add_graph, plot_init, plot_send_points, plot_set_graph,
        read_eeprom_word, read_nvm, set_cfg_float, set_cfg_int, shutdown_disable, store_cfg,
        store_eeprom_word, terminal_register_command_callback, terminal_unregister_callback,
        uart_read, uart_start, uart_write, vesc_free, vesc_imu_get_quaternions, vesc_mutex_create,
        vesc_mutex_lock, vesc_mutex_unlock, vesc_request_terminate, vesc_sem_create,
        vesc_sem_reset, vesc_sem_signal, vesc_sem_wait, vesc_sem_wait_to, vesc_should_terminate,
        vesc_sleep_us, vesc_spawn, vesc_system_time_seconds, vesc_system_time_ticks,
        vesc_thread_set_priority, vesc_timer_seconds_elapsed_since, vesc_timer_time_now,
        vesc_timestamp_age_seconds, wipe_nvm, write_nvm,
    };
    #[cfg(test)]
    pub use selected_ffi::{clear_pad, io_get_st_pin, set_pad, set_pad_mode};
    #[cfg(any(test, not(feature = "test-support")))]
    use vescpkg_rs_sys::raw as selected_ffi;
}

/// Capability-safe ABI inspection for loaders and host fixtures.
pub use capabilities::{
    FirmwareCapabilities, FirmwareFloatSetting, FirmwareIntSetting, FirmwareSettings, SettingsError,
};
pub use vesc_protocol::buffer as protocol_buffer;
pub use vescpkg_rs_sys::{AbiError, Stm32AbiRevision, VescIfPresence};
use vescpkg_rs_units as units;
pub use vescpkg_rs_units::{
    AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, BatteryCellCount,
    BatteryCellCountError, Charge, Current, Distance, DistancePerEnergy, Energy, EnergyPerDistance,
    FluxLinkage, Frequency, Height, Inductance, Latitude, Longitude, MagneticFluxDensity,
    OdometerMeters, Percent, Power, Ratio, Resistance, Rpm, SYSTEM_TICK_RATE_HZ, SampleRate,
    SignedRatio, Speed, SystemTicks, Temperature, TimestampTicks, VescSeconds, Voltage,
};

#[cfg(feature = "alloc")]
pub use alloc::VescAllocator;
pub use can_bus::{
    CanBus, CanError, CanHardwareType, CanReceiverCallback, CanReceiverGuard, CanReceiverHandler,
    CanReceiverId, CanStatus, CanStatus2, CanStatus3, CanStatus4, CanStatus5, CanStatus6,
};
pub use eeprom::{CustomEeprom, CustomEepromAddress, EepromWord};
pub use encoder::{Encoder, EncoderError, EncoderHandler, EncoderRegistration};
pub use extension::{ExtensionDescriptor, ExtensionName, ExtensionRegistration};
pub use extension::{
    LbmExtension, LispArgs, LispContextId, LispFlatValue, LispIntegerError, LispList,
    LispListError, LispMessageError, LispProcess, LispSymbol, LispValue, StatefulLbmExtension,
};
pub use inputs::{
    FirmwareInputs, InputError, PpmSnapshot, RemoteInputSnapshot, ShutdownInhibit, TimeoutSnapshot,
};
pub use logging::{FirmwareLog, LogError};

// Exported macros need public implementation hooks after downstream expansion.
// Keep those hooks in one hidden namespace instead of the package-author root.
// The functions retain their existing definitions and generated symbols.
// Only macro expansion should name this module.
#[doc(hidden)]
pub mod __macro_support;

#[cfg(feature = "math")]
pub use ahrs::{Ahrs, Madgwick};
pub use audio::{FocAudio, FocAudioError, FocAudioSampleTable};
pub use firmware::{
    AppDataHandler, AppDataPacket, ConfigBytes, ConfigXml, StatefulCustomConfigCallback,
};
pub(crate) use firmware::{firmware_array, loader_info_mut};
pub use gnss::{Gnss, GnssError, GnssSnapshot};
pub use imu::{
    FirmwareAhrs, FirmwareAhrsError, FirmwareAhrsParameters, FirmwareAhrsSnapshot, Imu,
    ImuCalibration, ImuCalibrationError, ImuReadCallback, ImuReadCallbackError,
    ImuReadCallbackLease, ImuReadHandler, register_imu_read_callback,
};
pub use init::{PackageStart, PackageStartError};
pub use input::{ControllerInput, RemoteInput};
pub use lifecycle_core::AppDataSendError;
pub use motor::{MotorOutput, MotorTelemetry};
pub use nvm::{Nvm, NvmCapacity, NvmError, NvmOffset};
#[cfg(feature = "alloc")]
pub use packet::OwnedPacketRegistration;
pub use packet::{PacketCodec, PacketError, PacketHandler, PacketRegistration};
pub use plot::{Plot, PlotError};
pub use runtime::{PackageRuntimeState, PackageStateAccess, PackageStateStore};
pub use sync::{FirmwareMutex, FirmwareMutexGuard, FirmwareSemaphore};
pub use terminal::{Terminal, TerminalArgs, TerminalError, TerminalHandler, TerminalRegistration};
pub use thread::{
    Firmware, FirmwareAppData, FirmwareClock, FirmwareThread, FirmwareThreads,
    StatelessFirmwareThread, StatelessThreadContext, ThreadContext, ThreadError, ThreadName,
    ThreadSpec, ThreadWorkingAreaSize, ThreadWorkingAreaSizeError, TimerInstant,
};
pub use uart::{Uart, UartError, UartLease};

/// CAN transport and status snapshot helpers for package code.
mod can_bus;
/// Command packet processing and scoped reply callbacks.
mod commands;
/// GPIO bindings and convenience wrappers for package code.
mod gpio;
/// IMU bindings and convenience wrappers for package code.
mod imu;
/// Device package entrypoint and loader-hook helpers.
mod init;
/// Typed controller input and output-safety helpers for package code.
mod inputs;
/// Motor telemetry bindings and convenience wrappers for package code.
mod motor;
/// Firmware thread bindings and convenience wrappers for package code.
mod thread;

pub use advanced_foc::{AdvancedFoc, AdvancedFocError};
pub use commands::{CommandError, CommandReplyHandler, CommandReplyLease, Commands};
pub use gpio::{
    AnalogGpioLease, AnalogPin, DigitalGpioLease, DigitalOutputLevel, DigitalPin, Gpio, GpioError,
    GpioMode,
};
pub use pwm::{
    PwmCallback, PwmCallbackError, PwmCallbackHandler, PwmCallbackLease, TypedPwmCallbackLease,
};
/// VESC-domain semantic types re-exported at the crate root.
pub use types::*;

/// Common package-author imports for code running inside the controller.
pub mod prelude {
    #[cfg(feature = "alloc")]
    pub use crate::OwnedPacketRegistration;
    pub use crate::types::*;
    pub use crate::units::{
        AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, BoundedUnitError, Charge,
        Current, Distance, DistancePerEnergy, Energy, EnergyPerDistance, FluxLinkage, Frequency,
        Height, Inductance, Latitude, Longitude, MagneticFluxDensity, OdometerMeters, Percent,
        Power, Ratio, Resistance, Rpm, SYSTEM_TICK_RATE_HZ, SampleRate, SignedRatio, Speed,
        SystemTicks, Temperature, TimestampTicks, VescSeconds, Voltage,
    };
    #[cfg(feature = "math")]
    pub use crate::{Ahrs, Madgwick};
    pub use crate::{
        AnalogPin, AppDataHandler, AppDataSendError, CanBus, CanError, CanReceiverHandler,
        CanReceiverId, CanStatus, CommandError, CommandReplyHandler, Commands, ConfigBytes,
        ConfigXml, CustomEeprom, CustomEepromAddress, DigitalOutputLevel, DigitalPin, EepromWord,
        Encoder, EncoderError, EncoderHandler, EncoderRegistration, ExtensionDescriptor,
        ExtensionName, ExtensionRegistration, Firmware, FirmwareAhrs, FirmwareAhrsError,
        FirmwareAhrsParameters, FirmwareAhrsSnapshot, FirmwareAppData, FirmwareCapabilities,
        FirmwareClock, FirmwareFloatSetting, FirmwareInputs, FirmwareIntSetting, FirmwareLog,
        FirmwareMutex, FirmwareMutexGuard, FirmwareSemaphore, FirmwareSettings, FirmwareThread,
        FirmwareThreads, FocAudio, FocAudioError, FocAudioSampleTable, Gnss, GnssError,
        GnssSnapshot, Gpio, Imu, ImuReadCallback, ImuReadCallbackError, ImuReadCallbackLease,
        ImuReadHandler, InputError, LbmExtension, LispArgs, LispContextId, LispFlatValue,
        LispIntegerError, LispList, LispListError, LispMessageError, LispProcess, LispSymbol,
        LispValue, LogError, MotorOutput, MotorTelemetry, Nvm, NvmCapacity, NvmError, NvmOffset,
        PackageStartError, PacketCodec, PacketError, PacketHandler, Plot, PlotError, PpmSnapshot,
        PwmCallbackError, PwmCallbackHandler, PwmCallbackLease, RemoteInputSnapshot, SettingsError,
        ShutdownInhibit, StatefulCustomConfigCallback, StatefulLbmExtension,
        StatelessFirmwareThread, StatelessThreadContext, Terminal, TerminalError, TerminalHandler,
        TerminalRegistration, ThreadContext, ThreadError, ThreadName, ThreadSpec,
        ThreadWorkingAreaSize, ThreadWorkingAreaSizeError, TimeoutSnapshot, TimerInstant,
        TypedPwmCallbackLease, Uart, UartError, UartLease,
    };
}

/// VESC-domain semantic wrappers over generic embedded units.
mod types;
pub(crate) use types::loader::{LoaderInfo, PackageProgramAddress};

/// Define package data retained in a named firmware image section.
#[macro_export]
macro_rules! firmware_section_static {
    ($section:literal, $visibility:vis static $name:ident: $type:ty = $value:expr) => {
        #[cfg_attr(
            all(not(test), target_arch = "arm"),
            unsafe(link_section = $section)
        )]
        #[used]
        $visibility static $name: $type = $value;
    };
}

#[cfg(test)]
mod tests {
    use crate::types::{
        AdcDecodedLevel, AdcVoltage, AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate,
        AudioVoltage, AveragePower, BatteryCurrent, BatteryVoltage, BaudRate, BrakeCurrentRelative,
        BrakeLeverLevel, BrakeSwitch, CanControllerId, CanPayloadLen, CurrentRelative, DVoltage,
        DirectionalMotorCurrent, DutyCycle, ElectricalSpeed, FocMotorFluxLinkage,
        FocMotorInductance, FocMotorResistance, GearRatio, GnssAccuracy, GnssHdop, GnssLatitude,
        GnssLongitude, GnssSpeed, HandbrakeRelative, ImuAcceleration, ImuAngularRate, ImuAttitude,
        ImuOrientation, ImuPitch, ImuQuaternion, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY,
        ImuQuaternionZ, ImuRoll, ImuYaw, JoystickX, JoystickY, MechanicalSpeed, MotorCurrent,
        MotorPoleCount, OpenLoopPhase, PacketLength, PeakPower, PidPosition, PpmAge, PpmInput, Pwm,
        QVoltage, RemoteAge, SystemDuration, ThreadPriority, TimeoutDuration, TotalMotorCurrent,
        TripDistance, VehicleSpeed, WattHoursDischarged, WheelDiameter,
    };
    use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};
    use vescpkg_rs_units::{
        AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, Current, Distance, Energy,
        FluxLinkage, Frequency, Inductance, Latitude, Longitude, Power, Ratio, Resistance, Rpm,
        SampleRate, SignedRatio, Speed, SystemTicks, TimestampTicks, VescSeconds, Voltage,
    };

    use crate::types::{
        ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRatePitch,
        ImuAngularRateRoll, ImuAngularRateYaw,
    };

    #[test]
    fn device_side_can_use_the_shared_protocol_crate() {
        let frame = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[7, 8]);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Ping);
        assert_eq!(frame.payload(), &[7, 8]);
    }

    #[test]
    fn package_author_prelude_exports_runtime_surface() {
        use crate::prelude::*;

        let switch = BrakeSwitch::Released;
        let _inputs = FirmwareInputs::new();
        let _address = CustomEepromAddress::from_index(0);
        let _word = EepromWord::from_u32(1);
        let _eeprom = CustomEeprom::new();
        let _: Option<LispContextId> = None;
        let _: Option<LispFlatValue> = None;
        let _: Option<LispMessageError> = None;
        let _: Option<LispProcess> = None;
        let _: Option<LispSymbol> = None;
        let _: Option<FocAudio> = None;
        let _: Option<FocAudioError> = None;
        let _: Option<FocAudioSampleTable<'_>> = None;
        let _: Option<FirmwareLog<32>> = None;
        let _: Option<InputError> = None;
        let _: Option<PpmSnapshot> = None;
        let _: Option<RemoteInputSnapshot> = None;
        let _: Option<ShutdownInhibit> = None;
        let _: Option<TimeoutSnapshot> = None;

        assert_eq!(switch, BrakeSwitch::Released);
    }

    #[test]
    fn config_bytes_are_a_regular_borrowed_value() {
        use crate::prelude::*;

        let config: ConfigBytes<'_, 6> = ConfigBytes::new(b"config");
        let xml = ConfigXml::new(b"<config/>");

        assert_eq!(config.as_bytes(), b"config");
        assert_eq!(xml.as_bytes(), b"<config/>");
    }

    #[test]
    fn semantic_current_types_are_not_interchangeable() {
        let motor = MotorCurrent::new(Current::from_amps(10.0));
        let battery = BatteryCurrent::new(Current::from_amps(6.0));

        assert_eq!(motor.current().as_amps(), 10.0);
        assert_eq!(battery.current().as_amps(), 6.0);
    }

    #[test]
    fn semantic_current_types_support_same_domain_arithmetic() {
        let command = MotorCurrent::new(Current::from_amps(10.0))
            + MotorCurrent::new(Current::from_amps(2.0));
        let filtered = command * 0.25 - MotorCurrent::new(Current::from_amps(1.0));

        assert_eq!(command.current().as_amps(), 12.0);
        assert_eq!(filtered.current().as_amps(), 2.0);
        assert_eq!((-filtered).current().as_amps(), -2.0);
    }

    #[test]
    fn semantic_voltage_energy_and_aggregate_current_types_wrap_units() {
        let total = TotalMotorCurrent::new(Current::from_amps(18.0));
        let directional = DirectionalMotorCurrent::new(Current::from_amps(-2.0));
        let battery_voltage = BatteryVoltage::new(Voltage::from_volts(50.4));
        let discharged = WattHoursDischarged::new(Energy::from_watt_hours(42.0));
        let d_voltage = DVoltage::new(Voltage::from_volts(1.25));
        let q_voltage = QVoltage::new(Voltage::from_volts(2.5));
        let audio_voltage = AudioVoltage::new(Voltage::from_volts(0.75));
        let average_power = AveragePower::new(Power::from_watts(420.0));
        let peak_power = PeakPower::new(Power::from_watts(900.0));

        assert_eq!(total.current().as_amps(), 18.0);
        assert_eq!(directional.current().as_amps(), -2.0);
        assert_eq!(battery_voltage.voltage().as_volts(), 50.4);
        assert_eq!(discharged.energy().as_watt_hours(), 42.0);
        assert_eq!(d_voltage.voltage().as_volts(), 1.25);
        assert_eq!(q_voltage.voltage().as_volts(), 2.5);
        assert_eq!(audio_voltage.voltage().as_volts(), 0.75);
        assert_eq!(average_power.power().as_watts(), 420.0);
        assert_eq!(peak_power.power().as_watts(), 900.0);
    }

    #[test]
    fn semantic_motion_and_gnss_types_wrap_units() {
        let speed = VehicleSpeed::new(Speed::from_meters_per_second(4.0));
        let trip = TripDistance::new(Distance::from_meters(123.0));
        let mechanical = MechanicalSpeed::new(Rpm::from_revolutions_per_minute(3000.0));
        let latitude = GnssLatitude::new(Latitude::from_degrees(40.015));
        let longitude = GnssLongitude::new(Longitude::from_degrees(-105.2705));
        let gnss_speed = GnssSpeed::new(Speed::from_meters_per_second(3.5));
        let hdop = GnssHdop::from_unitless(0.9);
        let accuracy = GnssAccuracy::new(Distance::from_meters(1.8));

        assert_eq!(speed.speed().as_meters_per_second(), 4.0);
        assert_eq!(trip.distance().as_meters(), 123.0);
        assert_eq!(mechanical.rpm().as_revolutions_per_minute(), 3000.0);
        assert_eq!(latitude.latitude().as_degrees(), 40.015);
        assert_eq!(longitude.longitude().as_degrees(), -105.2705);
        assert_eq!(gnss_speed.speed().as_meters_per_second(), 3.5);
        assert_eq!(hdop.as_unitless(), 0.9);
        assert_eq!(accuracy.distance().as_meters(), 1.8);
    }

    #[test]
    fn semantic_rpm_types_wrap_generic_rpm_without_interchangeability() {
        fn mechanical_command(speed: MechanicalSpeed) -> Rpm {
            speed.rpm()
        }

        fn electrical_command(speed: ElectricalSpeed) -> Rpm {
            speed.rpm()
        }

        let mechanical = MechanicalSpeed::new(Rpm::from_revolutions_per_minute(3000.0));
        let electrical = ElectricalSpeed::new(Rpm::from_revolutions_per_minute(21_000.0));

        assert_eq!(
            mechanical_command(mechanical).as_revolutions_per_minute(),
            3000.0
        );
        assert_eq!(
            electrical_command(electrical).as_revolutions_per_minute(),
            21_000.0
        );
    }

    #[test]
    fn semantic_package_inputs_follow_vesc_c_if_angle_and_audio_units() {
        let channel = AudioChannel::try_new(3).expect("last valid audio channel");
        let position = PidPosition::new(AngleDegrees::from_degrees(90.0));
        let phase = OpenLoopPhase::new(AngleDegrees::from_degrees(180.0));
        let audio_frequency = AudioFrequency::new(Frequency::from_hertz(440.0));
        let audio_sample_rate = AudioSampleRate::new(SampleRate::from_hertz(22_050.0));
        let audio_duration = AudioDuration::new(VescSeconds::from_seconds(0.25));

        assert_eq!(channel.as_u8(), 3);
        assert_eq!(AudioChannel::try_new(0).expect("first channel").as_u8(), 0);
        assert_eq!(AudioChannel::try_new(4).expect_err("too high").value(), 4);
        assert_eq!(position.angle().as_degrees(), 90.0);
        assert_eq!(phase.angle().as_degrees(), 180.0);
        assert_eq!(audio_frequency.frequency().as_hertz(), 440.0);
        assert_eq!(audio_sample_rate.sample_rate().as_hertz(), 22_050.0);
        assert_eq!(audio_duration.duration().as_seconds(), 0.25);
    }

    #[test]
    fn semantic_config_types_wrap_units_and_checked_scalars() {
        let poles = MotorPoleCount::try_new(14).expect("valid pole count");
        let cells = crate::types::BatteryCellCount::try_new(12).expect("valid cell count");
        let gear_ratio = GearRatio::try_new(2.6).expect("valid gear ratio");
        let wheel = WheelDiameter::new(Distance::from_meters(0.165));
        let motor_r = FocMotorResistance::new(Resistance::from_ohms(0.03));
        let motor_l = FocMotorInductance::new(Inductance::from_henries(0.000_012));
        let flux = FocMotorFluxLinkage::new(FluxLinkage::from_webers(0.004));

        assert_eq!(poles.as_u16(), 14);
        assert_eq!(cells.as_u16(), 12);
        assert_eq!(gear_ratio.as_f32(), 2.6);
        assert_eq!(wheel.distance().as_meters(), 0.165);
        assert_eq!(motor_r.resistance().as_ohms(), 0.03);
        assert_eq!(motor_l.inductance().as_henries(), 0.000_012);
        assert_eq!(flux.flux_linkage().as_webers(), 0.004);
        assert!(MotorPoleCount::try_new(0).is_err());
        assert!(crate::types::BatteryCellCount::try_new(0).is_err());
        assert!(GearRatio::try_new(0.0).is_err());
    }

    #[test]
    fn semantic_time_types_wrap_fugit_and_tick_units() {
        let ticks = SystemTicks::from_ticks(25_000);
        let seconds = VescSeconds::from_seconds(2.5);
        let timestamp = TimestampTicks::from_ticks(123_456);
        let duration = SystemDuration::new(ticks);
        let timeout = TimeoutDuration::new(seconds);
        let remote_age = RemoteAge::new(seconds);
        let ppm_age = PpmAge::new(seconds);

        assert_eq!(timestamp.as_ticks(), 123_456);
        assert_eq!(duration.duration().as_ticks(), 25_000);
        assert_eq!(timeout.duration().as_seconds(), 2.5);
        assert_eq!(remote_age.duration().as_seconds(), 2.5);
        assert_eq!(ppm_age.duration().as_seconds(), 2.5);
    }

    #[test]
    fn semantic_raw_tokens_require_explicit_checked_construction() {
        let controller = CanControllerId::new(42);
        let priority = ThreadPriority::try_new(5).expect("valid priority");
        let baud = BaudRate::try_new(115_200).expect("valid baud rate");
        let packet_len = PacketLength::try_new(512).expect("valid packet length");
        let payload_len = CanPayloadLen::try_new(8).expect("valid CAN payload length");

        assert_eq!(controller.as_u8(), 42);
        assert_eq!(priority.as_i8(), 5);
        assert_eq!(baud.as_u32(), 115_200);
        assert_eq!(packet_len.as_u32(), 512);
        assert_eq!(payload_len.as_u8(), 8);
        assert!(ThreadPriority::try_new(6).is_err());
        assert!(ThreadPriority::try_new(-6).is_err());
        assert!(BaudRate::try_new(0).is_err());
        assert!(PacketLength::try_new(0).is_err());
        assert!(CanPayloadLen::try_new(9).is_err());
    }

    #[test]
    fn semantic_package_inputs_follow_vesc_c_if_signed_ratio_ranges() {
        let duty = DutyCycle::new(SignedRatio::from_ratio(-0.25).expect("signed duty"));
        let pwm = Pwm::new(Ratio::from_ratio(0.75).expect("normalized PWM"));
        let current_rel =
            CurrentRelative::new(SignedRatio::from_ratio(-0.5).expect("signed current command"));
        let brake_rel = BrakeCurrentRelative::new(Ratio::from_ratio(0.75).expect("brake ratio"));
        let handbrake_rel =
            HandbrakeRelative::new(Ratio::from_ratio(0.5).expect("handbrake ratio"));
        let ppm = PpmInput::new(SignedRatio::from_ratio(-1.0).expect("decoded PPM"));
        let joystick_x = JoystickX::new(SignedRatio::from_ratio(-0.2).expect("joystick X"));
        let joystick_y = JoystickY::new(SignedRatio::from_ratio(0.8).expect("joystick Y"));

        assert_eq!(duty.ratio().as_ratio(), -0.25);
        assert_eq!(pwm.ratio().as_ratio(), 0.75);
        assert_eq!(current_rel.ratio().as_ratio(), -0.5);
        assert_eq!(brake_rel.ratio().as_ratio(), 0.75);
        assert_eq!(handbrake_rel.ratio().as_ratio(), 0.5);
        assert_eq!(ppm.ratio().as_ratio(), -1.0);
        assert_eq!(joystick_x.ratio().as_ratio(), -0.2);
        assert_eq!(joystick_y.ratio().as_ratio(), 0.8);
    }

    #[test]
    fn semantic_adc_and_proven_imu_types_wrap_firmware_units() {
        let adc_voltage = AdcVoltage::new(Voltage::from_volts(1.65));
        let adc_level = AdcDecodedLevel::new(Ratio::from_ratio(0.5).expect("normalized level"));
        let brake_lever = BrakeLeverLevel::new(Ratio::from_ratio(0.35).expect("brake lever level"));
        let brake_switch = BrakeSwitch::Pressed;
        let roll = ImuRoll::new(AngleRadians::from_radians(0.25));
        let pitch = ImuPitch::new(AngleRadians::from_radians(-0.125));
        let yaw = ImuYaw::new(AngleRadians::from_radians(1.0));
        let accel = ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(0.0)),
            ImuAccelerationY::new(AccelerationG::from_g(0.0)),
            ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
        );
        let gyro = ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(1.0)),
            ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(2.0)),
            ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(3.0)),
        );
        let attitude = ImuAttitude::new(roll, pitch, yaw);
        let orientation = ImuOrientation::from_quaternion(ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(0.0),
            ImuQuaternionZ::new(0.0),
        ));

        assert_eq!(adc_voltage.voltage().as_volts(), 1.65);
        assert_eq!(adc_level.ratio().as_ratio(), 0.5);
        assert_eq!(brake_lever.ratio().as_ratio(), 0.35);
        assert_eq!(brake_switch, BrakeSwitch::Pressed);
        assert_eq!(roll.angle().as_radians(), 0.25);
        assert_eq!(pitch.angle().as_radians(), -0.125);
        assert_eq!(yaw.angle().as_radians(), 1.0);
        accel.map_axes(|_, _, z| assert_eq!(z.acceleration().as_g(), 1.0));
        gyro.map_axes(|_, pitch, yaw| {
            assert_eq!(pitch.angular_velocity().as_degrees_per_second(), 2.0);
            assert_eq!(yaw.angular_velocity().as_degrees_per_second(), 3.0);
        });
        assert_eq!(gyro.pitch().as_degrees_per_second(), 2.0);
        assert_eq!(gyro.yaw().as_degrees_per_second(), 3.0);
        assert_eq!(attitude.roll(), roll);
        assert_eq!(attitude.pitch(), pitch);
        assert_eq!(attitude.yaw(), yaw);
        let quaternion = orientation.quaternion();
        assert_eq!(quaternion.w(), ImuQuaternionW::new(1.0));
        assert_eq!(quaternion.x(), ImuQuaternionX::new(0.0));
        assert_eq!(quaternion.y(), ImuQuaternionY::new(0.0));
        assert_eq!(quaternion.z(), ImuQuaternionZ::new(0.0));
    }
}

#[cfg(all(test, feature = "test-support"))]
mod lifecycle_tests;
