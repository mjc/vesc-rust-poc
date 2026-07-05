//! Target-side SDK for Rust VESC packages.
//!
//! Link this crate into native VESC package code. It wraps `vescpkg-rs-sys` with
//! lifecycle, LispBM extension, app-data, GPIO, and protocol helpers.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

#[cfg(test)]
extern crate std;

/// Firmware allocation helpers backed by the VESC native package allocator.
pub mod alloc;

mod bindings;
mod extension;
mod lifecycle_core;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Safe and unsafe raw ABI re-exports for SDK consumers that need them.
pub mod ffi {
    pub use crate::bindings::*;
    pub use crate::extension::*;
    pub use crate::lifecycle_core::*;
    #[cfg(any(test, feature = "test-support"))]
    pub use crate::test_support;
    pub use vescpkg_rs_sys::*;
}

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};
pub use vescpkg_rs_units as units;

pub use alloc::{AllocBindings, AllocError, FirmwareAllocation, FirmwareAllocator};
pub use bindings::{AppDataBindings, CustomConfigBindings, LbmBindings};
pub use extension::{ExtensionDescriptor, ExtensionNameError, RegisterError};
pub use imu::{ImuApi, ImuBindings};
pub use lifecycle_core::{
    AppDataHandlerRegistrationError, LbmApi, LoopbackLifecycle, PackageLifecycle,
};
pub use motor::{MotorControlApi, MotorControlBindings, MotorTelemetryApi, MotorTelemetryBindings};
pub use thread::{FirmwareThreadHandle, ThreadApi, ThreadBindings};

#[cfg(not(test))]
pub use bindings::RealBindings;
#[cfg(not(test))]
pub use imu::RealImuBindings;
#[cfg(not(test))]
pub use motor::{RealMotorControlBindings, RealMotorTelemetryBindings};
#[cfg(not(test))]
pub use thread::RealThreadBindings;

/// BLE loopback helpers and package-side packet handlers.
pub mod ble_loopback;
/// GPIO bindings and convenience wrappers for package code.
pub mod gpio;
/// IMU bindings and convenience wrappers for package code.
pub mod imu;
/// Device package entrypoint and loader-hook helpers.
pub mod init;
/// Motor telemetry bindings and convenience wrappers for package code.
pub mod motor;
/// Firmware thread bindings and convenience wrappers for package code.
pub mod thread;

#[cfg(not(test))]
pub use gpio::RealGpioBindings;
pub use gpio::{GpioApi, GpioBindings};
/// LispBM value encoding helpers and raw device-side integer packing.
pub mod lbm;
/// Higher-level lifecycle helpers for package startup and runtime behavior.
pub mod lifecycle;
/// Common package-author imports for code running inside the controller.
pub mod prelude {
    pub use crate::types::*;
    pub use crate::units::{
        AccelerationG, AmpHours, AngleDegrees, AngleRadians, AngularVelocity, BoundedUnitError,
        Charge, Current, Distance, DistancePerEnergy, Energy, EnergyPerDistance, FluxLinkage,
        Frequency, Height, Inductance, Latitude, Longitude, OdometerMeters, Percent, Power, Ratio,
        Resistance, Rpm, SYSTEM_TICK_RATE_HZ, SampleRate, SignedRatio, Speed, SystemInstant,
        SystemTicks, Temperature, TimestampTicks, VescSeconds, Voltage, WattHours,
    };
    pub use crate::{
        AllocBindings, AllocError, AppDataBindings, AppDataHandlerRegistrationError,
        CustomConfigBindings, ExtensionDescriptor, ExtensionNameError, FirmwareAllocation,
        FirmwareAllocator, FirmwareThreadHandle, GpioApi, GpioBindings, ImuApi, ImuBindings,
        LbmApi, LbmBindings, LoopbackLifecycle, MotorControlApi, MotorControlBindings,
        MotorTelemetryApi, MotorTelemetryBindings, PackageLifecycle, ProtocolFrame, RegisterError,
        ThreadApi, ThreadBindings, WireCommand, WireVersion,
    };

    #[cfg(not(test))]
    pub use crate::{
        RealBindings, RealGpioBindings, RealImuBindings, RealMotorControlBindings,
        RealMotorTelemetryBindings, RealThreadBindings,
    };
}
/// VESC-domain semantic wrappers over generic embedded units.
pub mod types;

#[cfg(test)]
mod tests {
    use super::{ProtocolFrame, WireCommand, WireVersion};
    use crate::types::{
        AdcDecodedLevel, AdcVoltage, AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate,
        AudioVoltage, AveragePower, BatteryCurrent, BatteryVoltage, BaudRate, BrakeCurrentRelative,
        BrakeLeverLevel, BrakeSwitch, CanControllerId, CanPayloadLen, CurrentRelative, DVoltage,
        DirectionalMotorCurrent, DutyCycle, ElectricalSpeed, FocMotorFluxLinkage,
        FocMotorInductance, FocMotorResistance, GearRatio, GnssAccuracy, GnssHdop, GnssLatitude,
        GnssLongitude, GnssSpeed, HandbrakeRelative, ImuAcceleration, ImuAngularRate, ImuPitch,
        ImuQuaternion, ImuRoll, ImuYaw, JoystickX, JoystickY, MechanicalSpeed, MotorCurrent,
        MotorPoleCount, OpenLoopPhase, PacketLength, PeakPower, PidPosition, PpmAge, PpmInput, Pwm,
        QVoltage, RemoteAge, SystemDuration, SystemTimestamp, ThreadPriority, TimeoutDuration,
        TotalMotorCurrent, TripDistance, VehicleSpeed, WattHoursDischarged, WheelDiameter,
    };
    use vescpkg_rs_units::{
        AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, Current, Distance, Energy,
        FluxLinkage, Frequency, Inductance, Latitude, Longitude, Power, Ratio, Resistance, Rpm,
        SampleRate, SignedRatio, Speed, SystemTicks, TimestampTicks, VescSeconds, Voltage,
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

        let _package = PackageLifecycle::new(crate::test_support::FakeBindings::new());
        let _loopback = LoopbackLifecycle::new(crate::test_support::FakeAppDataBindings::new());
        let telemetry = MotorTelemetryApi::new(
            crate::test_support::FakeMotorTelemetryBindings::new()
                .with_distance_abs(TripDistance::new(Distance::from_meters(1.25))),
        );
        let motor = MotorControlApi::new(crate::test_support::FakeMotorControlBindings::new());
        let imu = ImuApi::new(crate::test_support::FakeImuBindings::new());
        let _descriptor = ExtensionDescriptor::new(
            c"ext-rust-prelude",
            crate::test_support::stubs::extension_handler,
        );
        let command = WireCommand::Ping;
        let switch = BrakeSwitch::Released;

        assert_eq!(command, WireCommand::Ping);
        assert_eq!(switch, BrakeSwitch::Released);
        assert_eq!(telemetry.distance_abs().distance().as_meters(), 1.25);
        motor.set_current(MotorCurrent::new(Current::from_amps(2.5)));
        assert_eq!(motor.bindings().current().current().as_amps(), 2.5);
        assert!(!imu.startup_done());
    }

    #[test]
    fn semantic_current_types_are_not_interchangeable() {
        let motor = MotorCurrent::new(Current::from_amps(10.0));
        let battery = BatteryCurrent::new(Current::from_amps(6.0));

        assert_eq!(motor.current().as_amps(), 10.0);
        assert_eq!(battery.current().as_amps(), 6.0);
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

        assert_eq!(channel.get(), 3);
        assert_eq!(AudioChannel::try_new(0).expect("first channel").get(), 0);
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

        assert_eq!(poles.get(), 14);
        assert_eq!(cells.get(), 12);
        assert_eq!(gear_ratio.get(), 2.6);
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
        let timestamp = SystemTimestamp::new(TimestampTicks::from_ticks(123_456));
        let duration = SystemDuration::new(ticks);
        let timeout = TimeoutDuration::new(seconds);
        let remote_age = RemoteAge::new(seconds);
        let ppm_age = PpmAge::new(seconds);

        assert_eq!(timestamp.ticks().as_ticks(), 123_456);
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

        assert_eq!(controller.get(), 42);
        assert_eq!(priority.get(), 5);
        assert_eq!(baud.get(), 115_200);
        assert_eq!(packet_len.get(), 512);
        assert_eq!(payload_len.get(), 8);
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
        let adc_level = AdcDecodedLevel::try_new(Ratio::from_ratio(0.5).expect("normalized level"))
            .expect("firmware decoded ADC level");
        let brake_lever = BrakeLeverLevel::new(Ratio::from_ratio(0.35).expect("brake lever level"));
        let brake_switch = BrakeSwitch::Pressed;
        let roll = ImuRoll::new(AngleRadians::from_radians(0.25));
        let pitch = ImuPitch::new(AngleRadians::from_radians(-0.125));
        let yaw = ImuYaw::new(AngleRadians::from_radians(1.0));
        let accel = ImuAcceleration::new([
            AccelerationG::from_g(0.0),
            AccelerationG::from_g(0.0),
            AccelerationG::from_g(1.0),
        ]);
        let gyro = ImuAngularRate::new([
            AngularVelocity::from_degrees_per_second(1.0),
            AngularVelocity::from_degrees_per_second(2.0),
            AngularVelocity::from_degrees_per_second(3.0),
        ]);
        let quat = ImuQuaternion::from_components([1.0, 0.0, 0.0, 0.0]);

        assert_eq!(adc_voltage.voltage().as_volts(), 1.65);
        assert_eq!(adc_level.ratio().as_ratio(), 0.5);
        assert_eq!(brake_lever.ratio().as_ratio(), 0.35);
        assert_eq!(brake_switch, BrakeSwitch::Pressed);
        assert_eq!(roll.angle().as_radians(), 0.25);
        assert_eq!(pitch.angle().as_radians(), -0.125);
        assert_eq!(yaw.angle().as_radians(), 1.0);
        assert_eq!(accel.xyz()[2].as_g(), 1.0);
        assert_eq!(gyro.xyz()[1].as_degrees_per_second(), 2.0);
        assert_eq!(quat.components(), [1.0, 0.0, 0.0, 0.0]);
    }
}

#[cfg(all(test, feature = "test-support"))]
mod lifecycle_tests;
