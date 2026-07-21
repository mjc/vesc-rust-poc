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

#[cfg(any(test, all(not(target_arch = "arm"), feature = "test-support")))]
extern crate alloc as rust_alloc;
#[cfg(test)]
extern crate std;

/// Firmware allocation helpers backed by the VESC native package allocator.
mod alloc;

mod bindings;
mod eeprom;
mod extension;
mod firmware;
mod lifecycle_core;
/// Float math entrypoints backed by Rust `libm` on package and host builds.
#[cfg(feature = "math")]
mod math;
#[cfg(feature = "math")]
pub use math::{asin, cos, sin, sqrt, tan};
mod runtime;
#[cfg(all(feature = "test-support", not(test)))]
mod test_ffi;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Internal ABI seam. `vescpkg-rs-sys` selects the real or test implementation.
pub(crate) mod ffi {
    pub use vescpkg_rs_sys::raw::{
        CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback,
    };
    #[allow(unused_imports)]
    pub use vescpkg_rs_sys::raw::{
        conf_custom_add_config, conf_custom_clear_configs, io_read, io_read_analog, io_set_mode,
        io_write, lbm_add_extension, lbm_dec_as_float, lbm_dec_as_i32, lbm_enc_i,
        lbm_enc_sym_eerror, lbm_enc_sym_nil, lbm_enc_sym_true, lbm_is_number,
        vesc_clear_app_data_handler, vesc_clear_imu_read_callback, vesc_free, vesc_get_arg,
        vesc_malloc, vesc_mutex_create, vesc_mutex_lock, vesc_mutex_unlock, vesc_send_app_data,
        vesc_set_app_data_handler, vesc_set_imu_read_callback, vesc_system_time_ticks,
    };
    pub use vescpkg_rs_sys::{AppDataHandler, LibInfo, NativeImage};

    #[cfg(all(feature = "test-support", not(test)))]
    use crate::test_ffi as selected_ffi;
    #[allow(unused_imports)]
    pub use selected_ffi::{
        foc_get_id, get_cfg_float, get_cfg_int, imu_get_gyro, imu_get_pitch, imu_get_roll,
        imu_get_yaw, imu_startup_done, mc_get_amp_hours, mc_get_amp_hours_charged,
        mc_get_battery_level, mc_get_distance_abs, mc_get_duty_cycle_now, mc_get_fault,
        mc_get_input_voltage_filtered, mc_get_odometer, mc_get_rpm, mc_get_speed,
        mc_get_tot_current_directional_filtered, mc_get_tot_current_filtered,
        mc_get_tot_current_in_filtered, mc_get_watt_hours, mc_get_watt_hours_charged,
        mc_set_brake_current, mc_set_current, mc_set_current_off_delay, mc_set_duty,
        mc_temp_fet_filtered, mc_temp_motor_filtered, read_eeprom_word, store_eeprom_word,
        timeout_reset, vesc_imu_get_quaternions, vesc_request_terminate, vesc_should_terminate,
        vesc_sleep_us, vesc_spawn, vesc_thread_set_priority,
    };
    #[cfg(any(test, not(feature = "test-support")))]
    use vescpkg_rs_sys::raw as selected_ffi;
}

pub use vesc_protocol::buffer as protocol_buffer;
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
pub use eeprom::{CustomEeprom, CustomEepromAddress, EepromWord};
pub use extension::{ExtensionDescriptor, ExtensionName, ExtensionRegistration};
pub use extension::{LbmExtension, LispArgs, LispIntegerError, LispValue, StatefulLbmExtension};

// Exported macros need public implementation hooks after downstream expansion.
// Keep those hooks in one hidden namespace instead of the package-author root.
// The functions retain their existing definitions and generated symbols.
// Only macro expansion should name this module.
#[doc(hidden)]
pub mod __macro_support;

pub use firmware::{
    AppDataHandler, AppDataPacket, ConfigBytes, ConfigXml, StatefulCustomConfigCallback,
};
pub(crate) use firmware::{firmware_array, loader_info_mut};
pub use imu::{Imu, ImuReadHandler};
pub use init::{PackageStart, PackageStartError};
pub use lifecycle_core::AppDataSendError;
pub use motor::{MotorOutput, MotorTelemetry};
pub use runtime::{PackageRuntimeState, PackageStateAccess, PackageStateStore};
pub use thread::{
    Firmware, FirmwareAppData, FirmwareClock, FirmwareThread, FirmwareThreads,
    StatelessFirmwareThread, StatelessThreadContext, ThreadContext, ThreadError, ThreadName,
    ThreadSpec, ThreadWorkingAreaSize, ThreadWorkingAreaSizeError,
};

/// GPIO bindings and convenience wrappers for package code.
mod gpio;
/// IMU bindings and convenience wrappers for package code.
mod imu;
/// Device package entrypoint and loader-hook helpers.
mod init;
/// Motor telemetry bindings and convenience wrappers for package code.
mod motor;
/// Firmware thread bindings and convenience wrappers for package code.
mod thread;

pub use gpio::{AnalogPin, DigitalOutputLevel, DigitalPin, Gpio};
/// VESC-domain semantic types re-exported at the crate root.
pub use types::*;

/// Common package-author imports for code running inside the controller.
pub mod prelude {
    pub use crate::types::*;
    pub use crate::units::{
        AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, BoundedUnitError, Charge,
        Current, Distance, DistancePerEnergy, Energy, EnergyPerDistance, FluxLinkage, Frequency,
        Height, Inductance, Latitude, Longitude, MagneticFluxDensity, OdometerMeters, Percent,
        Power, Ratio, Resistance, Rpm, SYSTEM_TICK_RATE_HZ, SampleRate, SignedRatio, Speed,
        SystemTicks, Temperature, TimestampTicks, VescSeconds, Voltage,
    };
    pub use crate::{
        AnalogPin, AppDataHandler, AppDataSendError, ConfigBytes, ConfigXml, DigitalOutputLevel,
        DigitalPin, ExtensionDescriptor, ExtensionName, ExtensionRegistration, Firmware,
        FirmwareAppData, FirmwareClock, FirmwareThread, FirmwareThreads, Gpio, Imu, ImuReadHandler,
        LbmExtension, LispArgs, LispIntegerError, LispValue, MotorOutput, MotorTelemetry,
        PackageRuntimeState, PackageStart, PackageStartError, StatefulCustomConfigCallback,
        StatefulLbmExtension, StatelessFirmwareThread, StatelessThreadContext, ThreadContext,
        ThreadError, ThreadName, ThreadSpec, ThreadWorkingAreaSize, ThreadWorkingAreaSizeError,
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
