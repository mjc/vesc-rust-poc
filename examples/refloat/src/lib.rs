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
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RECORDED_ITEMS,
        REFLOAT_REALTIME_RUNTIME_ITEMS, RefloatAppDataCommand, RefloatBeepReason,
        RefloatChargingState, RefloatDarkRideState, RefloatDataRecorderFlags,
        RefloatFatalErrorState, RefloatImuSample, RefloatMode, RefloatMotorCommand,
        RefloatMotorTelemetry, RefloatRealtimeDataHeader, RefloatRealtimeDataItem,
        RefloatRealtimeDataItemGroup, RefloatRealtimeDataRecordPolicy, RefloatRideState,
        RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition, RefloatWheelSlipState,
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

    #[test]
    fn package_author_parses_refloat_app_data_commands_as_domain_enum() {
        let commands = [
            (0, RefloatAppDataCommand::Info),
            (1, RefloatAppDataCommand::GetRealtimeData),
            (2, RefloatAppDataCommand::RuntimeTune),
            (3, RefloatAppDataCommand::TuneDefaults),
            (4, RefloatAppDataCommand::ConfigSave),
            (5, RefloatAppDataCommand::ConfigRestore),
            (6, RefloatAppDataCommand::TuneOther),
            (7, RefloatAppDataCommand::RcMove),
            (8, RefloatAppDataCommand::Booster),
            (9, RefloatAppDataCommand::PrintInfo),
            (10, RefloatAppDataCommand::GetAllData),
            (11, RefloatAppDataCommand::Experiment),
            (12, RefloatAppDataCommand::Lock),
            (13, RefloatAppDataCommand::HandTest),
            (14, RefloatAppDataCommand::TuneTilt),
            (20, RefloatAppDataCommand::LightsControl),
            (22, RefloatAppDataCommand::Flywheel),
            (24, RefloatAppDataCommand::LcmPoll),
            (25, RefloatAppDataCommand::LcmLightInfo),
            (26, RefloatAppDataCommand::LcmLightControl),
            (27, RefloatAppDataCommand::LcmDeviceInfo),
            (28, RefloatAppDataCommand::ChargingState),
            (29, RefloatAppDataCommand::LcmGetBattery),
            (31, RefloatAppDataCommand::RealtimeData),
            (32, RefloatAppDataCommand::RealtimeDataIds),
            (35, RefloatAppDataCommand::AlertsList),
            (36, RefloatAppDataCommand::AlertsControl),
            (41, RefloatAppDataCommand::DataRecordRequest),
            (99, RefloatAppDataCommand::LcmDebug),
        ];

        assert_eq!(REFLOAT_APP_DATA_PACKAGE_ID.get(), 101);
        assert!(commands.into_iter().all(|(id, command)| {
            RefloatAppDataCommand::try_from_id(id)
                .is_ok_and(|parsed| parsed == command && parsed.id() == id)
        }));
        assert_eq!(
            RefloatAppDataCommand::try_from_id(200)
                .expect_err("unstable command should stay explicit")
                .value(),
            200
        );
    }

    #[test]
    fn package_author_builds_realtime_data_header_without_raw_bit_flags() {
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::PushbackLowVoltage,
            RefloatStopCondition::QuickStop,
        )
        .with_charging(RefloatChargingState::Charging)
        .with_wheelslip(RefloatWheelSlipState::Detected)
        .with_darkride(RefloatDarkRideState::Active);
        let recorder = RefloatDataRecorderFlags::inactive()
            .with_recording()
            .with_autostop();
        let header = RefloatRealtimeDataHeader::new(
            SystemTimestamp::new(TimestampTicks::from_ticks(123_456)),
            ride_state,
            FootpadSensorState::Both,
            RefloatBeepReason::FirmwareFault,
        )
        .with_data_recorder(recorder)
        .with_fatal_error(RefloatFatalErrorState::Present);

        assert_eq!(header.timestamp().ticks().as_ticks(), 123_456);
        assert_eq!(header.data_mask_compat(), 0b0000_0111);
        assert_eq!(header.extra_flags_compat(), 0b0000_1101);
        assert_eq!(header.state_byte_compat(), 0x23);
        assert_eq!(header.footpad_flags_compat(), 0b1110_0011);
        assert_eq!(header.stop_setpoint_byte_compat(), 0xB6);
        assert_eq!(header.beep_reason_compat(), 19);
    }

    #[test]
    fn package_author_reads_realtime_data_item_ids_as_typed_contract() {
        assert_eq!(
            REFLOAT_REALTIME_DATA_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "motor.speed",
                "motor.erpm",
                "motor.current",
                "motor.dir_current",
                "motor.filt_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "motor.batt_current",
                "motor.mosfet_temp",
                "motor.motor_temp",
                "imu.pitch",
                "imu.balance_pitch",
                "imu.roll",
                "footpad.adc1",
                "footpad.adc2",
                "remote.input",
            ]
        );
        assert_eq!(
            REFLOAT_REALTIME_RUNTIME_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "setpoint",
                "atr.setpoint",
                "brake_tilt.setpoint",
                "torque_tilt.setpoint",
                "turn_tilt.setpoint",
                "remote.setpoint",
                "balance_current",
                "atr.accel_diff",
                "atr.speed_boost",
                "booster.current",
            ]
        );
        assert_eq!(
            REFLOAT_REALTIME_RECORDED_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "motor.erpm",
                "motor.dir_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "imu.pitch",
                "imu.balance_pitch",
                "setpoint",
                "atr.setpoint",
                "torque_tilt.setpoint",
                "balance_current",
            ]
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorSpeed.group(),
            RefloatRealtimeDataItemGroup::Always
        );
        assert_eq!(
            RefloatRealtimeDataItem::BalanceCurrent.group(),
            RefloatRealtimeDataItemGroup::Runtime
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorErpm.record_policy(),
            RefloatRealtimeDataRecordPolicy::Record
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorSpeed.record_policy(),
            RefloatRealtimeDataRecordPolicy::SendOnly
        );
    }
}
