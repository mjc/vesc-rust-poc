//! Refloat VESC package payload.
//!
//! This crate owns Refloat-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

pub mod domain;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::domain::{
        FootpadSensorSample, FootpadSensorState, RefloatChargingState, RefloatImuSample,
        RefloatMode, RefloatMotorCommand, RefloatMotorTelemetry, RefloatRideState, RefloatRunState,
        RefloatSetpointAdjustment, RefloatStopCondition, RefloatWheelSlipState,
    };
    use vescpkg_rs::prelude::*;

    #[test]
    fn package_author_models_refloat_ride_inputs_without_raw_float_handoff() {
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.65)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.72)),
            FootpadSensorState::Both,
        );
        let imu = RefloatImuSample::new(
            ImuPitch::new(AngleRadians::from_radians(0.03)),
            ImuRoll::new(AngleRadians::from_radians(-0.01)),
            ImuYaw::new(AngleRadians::from_radians(1.25)),
            ImuAngularRate::new([
                AngularVelocity::from_degrees_per_second(12.0),
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(-1.0),
            ]),
        );
        let telemetry = RefloatMotorTelemetry::new(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(4.5)),
            DirectionalMotorCurrent::new(Current::from_amps(8.0)),
            BatteryCurrent::new(Current::from_amps(3.0)),
            DutyCycle::new(SignedRatio::from_ratio_const(0.18)),
            BatteryVoltage::new(Voltage::from_volts(74.0)),
        );

        assert_eq!(footpad.state(), FootpadSensorState::Both);
        assert_eq!(imu.pitch().angle().as_radians(), 0.03);
        assert_eq!(
            telemetry
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute(),
            2400.0
        );
        assert_eq!(telemetry.motor_current().current().as_amps(), 8.0);
        assert_eq!(telemetry.battery_current().current().as_amps(), 3.0);
    }

    #[test]
    fn package_author_requests_refloat_motor_current_with_domain_intent() {
        fn apply_requested_current(command: RefloatMotorCommand) -> MotorCurrent {
            command.requested_current()
        }

        let command = RefloatMotorCommand::new(MotorCurrent::new(Current::from_amps(11.0)));

        assert_eq!(apply_requested_current(command).current().as_amps(), 11.0);
    }

    #[test]
    fn package_author_reads_refloat_state_as_enums_not_bool_or_integer_flags() {
        let ready_pitch_fault = RefloatRideState::new(
            RefloatRunState::Ready,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::Pitch,
        );
        let running_tiltback = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::PushbackHighVoltage,
            RefloatStopCondition::None,
        );

        assert_eq!(ready_pitch_fault.float_state_compat(), 6);
        assert_eq!(running_tiltback.float_state_compat(), 2);
        assert_eq!(running_tiltback.setpoint_adjustment_compat(), 4);
        assert_eq!(
            running_tiltback
                .with_wheelslip(RefloatWheelSlipState::Detected)
                .float_state_compat(),
            2
        );
        assert_eq!(
            running_tiltback
                .with_charging(RefloatChargingState::Charging)
                .with_wheelslip(RefloatWheelSlipState::Detected)
                .float_state_compat(),
            14
        );
    }
}
