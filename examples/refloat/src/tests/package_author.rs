use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RECORDED_ITEMS,
    REFLOAT_REALTIME_RUNTIME_ITEMS, RefloatAlertId, RefloatAllDataAttitude,
    RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature, RefloatAllDataMode,
    RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
    RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataRequest,
    RefloatAllDataRequestError, RefloatAllDataResponse, RefloatAllDataStatus,
    RefloatAppDataCommand, RefloatBeepReason, RefloatChargingState, RefloatDarkRideState,
    RefloatDataRecorderFlags, RefloatFatalErrorState, RefloatFocIdCurrent, RefloatFootpadSample,
    RefloatFootpadState, RefloatMode, RefloatMotorCommand, RefloatRealtimeAlertMask,
    RefloatRealtimeAlwaysPayload, RefloatRealtimeAtrAccelerationDiff, RefloatRealtimeAtrSpeedBoost,
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingPayload, RefloatRealtimeChargingVoltage,
    RefloatRealtimeDataHeader, RefloatRealtimeDataItem, RefloatRealtimeDataItemGroup,
    RefloatRealtimeDataRecordPolicy, RefloatRealtimeFilteredMotorCurrent,
    RefloatRealtimeImuPayload, RefloatRealtimeMotorCurrents, RefloatRealtimeMotorPayload,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRemoteInput, RefloatRealtimeReservedFlags,
    RefloatRealtimeRuntimeAtrPayload, RefloatRealtimeRuntimePayload,
    RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints, RefloatRealtimeTail,
    RefloatRideState, RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition,
    RefloatWheelSlipState,
};
use crate::leds::{
    RefloatLedAnimationMode, RefloatLedAnimationSpeed, RefloatLedBarConfig, RefloatLedColor,
    RefloatLedColorOrder, RefloatLedPin, RefloatLedPinConfig, RefloatLedStripConfig,
    RefloatLedStripOrder, RefloatLedTransition, RefloatLedsConfig, RefloatStatusBarConfig,
    RefloatStatusBarIdleTimeout,
};
use vescpkg_rs::prelude::*;

#[test]
fn test_package_lib_init_uses_side_effect_free_registration_tail() {
    let mut info = LoaderInfo::new();

    assert!(crate::package_lib_init(&mut info));
    // Upstream Refloat v1.2.1 installs `stop`/`Data *` at
    // `third_party/refloat/src/main.c:2431-2432` before the registration tail at
    // `third_party/refloat/src/main.c:2456-2459`; the test build keeps that tail side-effect free.
    assert!(!info.has_stop_handler());
    assert!(info.argument().is_none());
}

#[test]
fn package_author_builds_source_startup_all_data_payload() {
    let payloads = RefloatAllDataPayloads::source_startup();
    let response =
        payloads.encode_response(RefloatAllDataRequest::new(RefloatAllDataMode::with_mode4()));

    assert_eq!(
        payloads.base().status(),
        RefloatAllDataStatus::new(
            RefloatRideState::new(
                RefloatRunState::Startup,
                RefloatMode::Normal,
                RefloatSetpointAdjustment::None,
                RefloatStopCondition::None,
            ),
            RefloatBeepReason::None,
        )
    );
    assert_eq!(payloads.base().footpad().state(), RefloatFootpadState::None);
    assert_eq!(
        response.as_bytes(),
        &[
            101, 10, 4, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 128, 128, 128, 128, 128, 128, 0, 0, 128, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 222, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ],
    );
}

#[test]
fn package_author_models_refloat_ride_inputs_without_raw_float_handoff() {
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.65),
        Voltage::from_volts(0.72),
        RefloatFootpadState::Both,
    );
    let attitude = ImuAttitude::new(
        ImuRoll::new(AngleRadians::from_radians(-0.01)),
        ImuPitch::new(AngleRadians::from_radians(0.03)),
        ImuYaw::new(AngleRadians::from_radians(1.25)),
    );
    let angular_rate = ImuAngularRate::from_axes(
        ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(12.0)),
        ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(0.0)),
        ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(-1.0)),
    );
    let electrical_speed = ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0));
    let motor_current = DirectionalMotorCurrent::new(Current::from_amps(8.0));
    let battery_current = BatteryCurrent::new(Current::from_amps(3.0));

    assert_eq!(footpad.state(), RefloatFootpadState::Both);
    assert_eq!(attitude.pitch().angle().as_radians(), 0.03);
    assert_eq!(angular_rate.roll().as_degrees_per_second(), 12.0);
    assert_eq!(electrical_speed.rpm().as_revolutions_per_minute(), 2400.0);
    assert_eq!(motor_current.current().as_amps(), 8.0);
    assert_eq!(battery_current.current().as_amps(), 3.0);
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
fn package_author_parses_all_data_requests_without_raw_packet_checks() {
    let request = RefloatAllDataRequest::parse(&[
        REFLOAT_APP_DATA_PACKAGE_ID.get(),
        RefloatAppDataCommand::GetAllData.id(),
        4,
    ])
    .expect("all-data mode 4 request should parse");

    assert_eq!(request.mode(), RefloatAllDataMode::with_mode4());
    assert_eq!(request.mode().source_id(), 4);
    assert!(request.mode().includes_mode2());
    assert!(request.mode().includes_mode3());
    assert!(request.mode().includes_mode4());
    assert_eq!(
        RefloatAllDataRequest::parse(&[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id()
        ])
        .expect_err("truncated request should be rejected"),
        RefloatAllDataRequestError::Length { actual: 2 }
    );
    assert_eq!(
        RefloatAllDataRequest::parse(&[102, RefloatAppDataCommand::GetAllData.id(), 4])
            .expect_err("wrong package ID should be rejected"),
        RefloatAllDataRequestError::PackageId { value: 102 }
    );
    assert_eq!(
        RefloatAllDataRequest::parse(&[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::PrintInfo.id(),
            4
        ])
        .expect_err("wrong command should be rejected"),
        RefloatAllDataRequestError::Command { value: 9 }
    );
    assert_eq!(
        RefloatAllDataRequest::parse(&[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            9
        ])
        .expect("Refloat source accepts high all-data modes")
        .mode()
        .source_id(),
        9
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
        RefloatFootpadState::Both,
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

#[test]
fn package_author_builds_realtime_always_payload_without_raw_values() {
    let motor = RefloatRealtimeMotorPayload::new(
        VehicleSpeed::new(Speed::from_kilometers_per_hour(12.6)),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0)),
        RefloatRealtimeMotorCurrents::new(
            MotorCurrent::new(Current::from_amps(7.0)),
            DirectionalMotorCurrent::new(Current::from_amps(-6.75)),
            RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                Current::from_amps(-6.5),
            )),
            BatteryCurrent::new(Current::from_amps(3.5)),
        ),
        DutyCycle::new(SignedRatio::from_ratio_const(0.21)),
        BatteryVoltage::new(Voltage::from_volts(73.0)),
        RefloatRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(41.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(52.0)),
        ),
    );
    let imu = RefloatRealtimeImuPayload::new(
        ImuPitch::new(AngleRadians::from_radians(0.04)),
        RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.03)),
        ImuRoll::new(AngleRadians::from_radians(-0.02)),
    );
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.61),
        Voltage::from_volts(0.58),
        RefloatFootpadState::Both,
    );
    let payload = RefloatRealtimeAlwaysPayload::new(
        motor,
        imu,
        footpad,
        RefloatRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.18)),
    );

    assert_eq!(
        payload.item_contract().map(RefloatRealtimeDataItem::id),
        REFLOAT_REALTIME_DATA_ITEMS.map(RefloatRealtimeDataItem::id)
    );
    assert_eq!(
        payload.motor().speed().speed().as_kilometers_per_hour(),
        12.6
    );
    assert_eq!(
        payload
            .motor()
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        2400.0
    );
    assert_eq!(
        payload
            .motor()
            .currents()
            .filtered()
            .current()
            .current()
            .as_amps(),
        -6.5
    );
    assert_eq!(
        payload
            .motor()
            .temperatures()
            .motor()
            .temperature()
            .as_degrees_celsius(),
        52.0
    );
    assert_eq!(payload.imu().balance_pitch().angle().as_radians(), 0.03);
    assert_eq!(payload.footpad().state(), RefloatFootpadState::Both);
    assert_eq!(payload.remote_input().ratio().as_ratio(), 0.18);
}

#[test]
fn package_author_builds_realtime_runtime_payload_without_raw_values() {
    let payload = RefloatRealtimeRuntimePayload::new(
        RefloatRealtimeRuntimeSetpoints::new(
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.5)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.25)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.5)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.75)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.125)),
            RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        ),
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.5))),
        RefloatRealtimeRuntimeAtrPayload::new(
            RefloatRealtimeAtrAccelerationDiff::from_erpm_delta(12.0),
            RefloatRealtimeAtrSpeedBoost::from_units(-0.1),
        ),
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(1.25))),
    );

    assert_eq!(
        payload.item_contract().map(RefloatRealtimeDataItem::id),
        REFLOAT_REALTIME_RUNTIME_ITEMS.map(RefloatRealtimeDataItem::id)
    );
    assert_eq!(payload.setpoints().board().angle().as_degrees(), 1.5);
    assert_eq!(payload.setpoints().brake_tilt().angle().as_degrees(), -0.5);
    assert_eq!(payload.balance_current().current().current().as_amps(), 9.5);
    assert_eq!(payload.atr().accel_diff().as_erpm_delta(), 12.0);
    assert_eq!(payload.atr().speed_boost().as_units(), -0.1);
    assert_eq!(
        payload.booster_current().current().current().as_amps(),
        1.25
    );
}

#[test]
fn package_author_builds_realtime_charging_and_tail_without_raw_values() {
    let charging = RefloatRealtimeChargingPayload::new(
        RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(4.2))),
        RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.5))),
    );
    let tail = RefloatRealtimeTail::new(
        RefloatRealtimeAlertMask::empty().with_alert(RefloatAlertId::FirmwareFault),
        RefloatRealtimeReservedFlags::none(),
        FirmwareFaultCompatCode::from_compat_code(12),
    );

    assert_eq!(charging.current().current().current().as_amps(), 4.2);
    assert_eq!(charging.voltage().voltage().voltage().as_volts(), 82.5);
    assert!(tail.active_alerts().contains(RefloatAlertId::FirmwareFault));
    assert_eq!(tail.active_alerts().active_alert_mask_compat(), 0x1);
    assert_eq!(tail.reserved_flags().extra_flags_compat(), 0);
    assert_eq!(tail.firmware_fault_code().compat_code(), 12);
}

#[test]
fn package_author_builds_all_data_base_payload_without_raw_values() {
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatStopCondition::None,
    );
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.62),
        Voltage::from_volts(0.57),
        RefloatFootpadState::Both,
    );
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.5)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.25)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.5)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.75)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.125)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
    );
    let motor = RefloatAllDataMotorPayload::new(
        BatteryVoltage::new(Voltage::from_volts(73.5)),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2420.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(3.4)),
        RefloatRealtimeMotorCurrents::new(
            MotorCurrent::new(Current::from_amps(8.25)),
            DirectionalMotorCurrent::new(Current::from_amps(8.25)),
            RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                Current::from_amps(8.25),
            )),
            BatteryCurrent::new(Current::from_amps(3.75)),
        ),
        DutyCycle::new(SignedRatio::from_ratio_const(0.34)),
        RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(1.5))),
    );
    let payload = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.5))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.03)),
            ImuRoll::new(AngleRadians::from_radians(-0.02)),
            ImuPitch::new(AngleRadians::from_radians(0.04)),
        ),
        RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
        footpad,
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(1.25))),
        motor,
    );

    assert_eq!(payload.command(), RefloatAppDataCommand::GetAllData);
    assert_eq!(payload.status().ride_state().float_state_compat(), 1);
    assert_eq!(payload.status().beep_reason().id(), 1);
    assert_eq!(payload.footpad().state(), RefloatFootpadState::Both);
    assert_eq!(payload.setpoints().remote().angle().as_degrees(), 2.0);
    assert_eq!(payload.attitude().pitch().angle().as_radians(), 0.04);
    assert_eq!(payload.balance_current().current().current().as_amps(), 9.5);
    assert_eq!(
        payload.booster_current().current().current().as_amps(),
        1.25
    );
    assert_eq!(
        payload
            .motor()
            .vehicle_speed()
            .speed()
            .as_meters_per_second(),
        3.4
    );
    assert_eq!(
        payload
            .motor()
            .foc_id_current()
            .as_measured()
            .expect("FOC ID current should be measured")
            .current()
            .as_amps(),
        1.5
    );
}

#[test]
fn package_author_builds_all_data_extension_payloads_without_raw_values() {
    let mode2 = RefloatAllDataMode2Payload::new(
        TripDistance::new(Distance::from_meters(42.5)),
        RefloatRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
        ),
        RefloatAllDataBatteryTemperature::unavailable(),
    );
    let mode3 = RefloatAllDataMode3Payload::new(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::new(Ratio::from_ratio_const(0.72)),
    );
    let mode4 = RefloatAllDataMode4Payload::new(
        RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
        RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
    );

    assert_eq!(mode2.distance_abs().distance().as_meters(), 42.5);
    assert_eq!(
        mode2
            .temperatures()
            .mosfet()
            .temperature()
            .as_degrees_celsius(),
        44.0
    );
    assert!(mode2.battery_temperature().as_measured().is_none());
    assert_eq!(mode3.odometer().as_meters(), 123_456);
    assert_eq!(mode3.discharged_charge().charge().as_amp_hours(), 3.2);
    assert_eq!(mode3.charged_charge().charge().as_amp_hours(), 0.8);
    assert_eq!(mode3.discharged_energy().energy().as_watt_hours(), 170.0);
    assert_eq!(mode3.charged_energy().energy().as_watt_hours(), 18.5);
    assert_eq!(mode3.battery_level().ratio().as_ratio(), 0.72);
    assert_eq!(mode4.current().current().current().as_amps(), 1.2);
    assert_eq!(mode4.voltage().voltage().voltage().as_volts(), 82.4);
}

#[test]
fn package_author_encodes_all_data_base_response_like_refloat_v1_2_1() {
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatStopCondition::None,
    );
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        RefloatFootpadState::Both,
    );
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payload = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
            ImuRoll::new(AngleRadians::from_radians(-0.5)),
            ImuPitch::new(AngleRadians::from_radians(2.3)),
        ),
        RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
        footpad,
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
        RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
            RefloatRealtimeMotorCurrents::new(
                MotorCurrent::new(Current::from_amps(5.0)),
                DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                    Current::from_amps(5.0),
                )),
                BatteryCurrent::new(Current::from_amps(-2.0)),
            ),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
            RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
        ),
    );

    assert_eq!(
        payload.encode_base_response(1),
        [
            101, 10, 1, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6,
        ]
    );
}

#[test]
fn package_author_encodes_all_data_fault_response_like_refloat_v1_2_1() {
    assert_eq!(
        RefloatAllDataResponse::fault(FirmwareFaultCompatCode::from_compat_code(5)).as_bytes(),
        &[101, 10, 69, 5]
    );
}

#[test]
fn package_author_encodes_all_data_mode4_response_like_refloat_v1_2_1() {
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatStopCondition::None,
    );
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        RefloatFootpadState::Both,
    );
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payload = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
            ImuRoll::new(AngleRadians::from_radians(-0.5)),
            ImuPitch::new(AngleRadians::from_radians(2.3)),
        ),
        RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
        footpad,
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
        RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
            RefloatRealtimeMotorCurrents::new(
                MotorCurrent::new(Current::from_amps(5.0)),
                DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                    Current::from_amps(5.0),
                )),
                BatteryCurrent::new(Current::from_amps(-2.0)),
            ),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
            RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
        ),
    );
    let mode2 = RefloatAllDataMode2Payload::new(
        TripDistance::new(Distance::from_meters(64.0)),
        RefloatRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
        ),
        RefloatAllDataBatteryTemperature::unavailable(),
    );
    let mode3 = RefloatAllDataMode3Payload::new(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::new(Ratio::from_ratio_const(0.72)),
    );
    let mode4 = RefloatAllDataMode4Payload::new(
        RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
        RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
    );

    assert_eq!(
        payload.encode_mode4_response(mode2, mode3, mode4),
        [
            101, 10, 4, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0, 0,
            1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144, 0, 12, 3, 56,
        ]
    );
}

#[test]
fn package_author_dispatches_all_data_responses_from_typed_request_mode() {
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::None,
        RefloatStopCondition::None,
    );
    let footpad = RefloatFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        RefloatFootpadState::Both,
    );
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payloads = RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
                ImuRoll::new(AngleRadians::from_radians(-0.5)),
                ImuPitch::new(AngleRadians::from_radians(2.3)),
            ),
            RefloatAllDataStatus::new(ride_state, RefloatBeepReason::LowVoltage),
            footpad,
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
            RefloatAllDataMotorPayload::new(
                BatteryVoltage::new(Voltage::from_volts(72.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
                RefloatRealtimeMotorCurrents::new(
                    MotorCurrent::new(Current::from_amps(5.0)),
                    DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                    RefloatRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                        Current::from_amps(5.0),
                    )),
                    BatteryCurrent::new(Current::from_amps(-2.0)),
                ),
                DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
                RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
            ),
        ),
        RefloatAllDataMode2Payload::new(
            TripDistance::new(Distance::from_meters(64.0)),
            RefloatRealtimeMotorTemperatures::new(
                MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
                MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
            ),
            RefloatAllDataBatteryTemperature::unavailable(),
        ),
        RefloatAllDataMode3Payload::new(
            OdometerMeters::from_meters(123_456),
            AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
            AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
            WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
            WattHoursCharged::new(Energy::from_watt_hours(18.5)),
            BatteryLevel::new(Ratio::from_ratio_const(0.72)),
        ),
        RefloatAllDataMode4Payload::new(
            RefloatRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
            RefloatRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
        ),
    );

    assert_eq!(
        payloads
            .encode_response(RefloatAllDataRequest::new(RefloatAllDataMode::base()))
            .as_bytes(),
        &[
            101, 10, 1, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(RefloatAllDataRequest::new(RefloatAllDataMode::with_mode2()))
            .as_bytes(),
        &[
            101, 10, 2, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(RefloatAllDataRequest::new(RefloatAllDataMode::with_mode3()))
            .as_bytes(),
        &[
            101, 10, 3, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0, 0,
            1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(RefloatAllDataRequest::new(
                RefloatAllDataMode::from_source_id(9)
            ))
            .as_bytes(),
        &[
            101, 10, 9, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0, 0,
            1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144, 0, 12, 3, 56,
        ]
    );
}

#[test]
fn package_author_reads_led_wiring_config_without_raw_numbers() {
    let strip =
        RefloatLedStripConfig::new(RefloatLedStripOrder::Second, 24, RefloatLedColorOrder::Grbw)
            .with_reverse(true);

    assert_eq!(RefloatLedPin::B6.id(), 0);
    assert_eq!(RefloatLedPin::B7.id(), 1);
    assert_eq!(RefloatLedPin::C9.id(), 2);
    assert_eq!(RefloatLedPinConfig::PullupTo5v.id(), 0);
    assert_eq!(RefloatLedPinConfig::NoPullup.id(), 1);
    assert_eq!(RefloatLedColorOrder::Grb.id(), 0);
    assert_eq!(RefloatLedColorOrder::Grbw.id(), 1);
    assert_eq!(RefloatLedColorOrder::Rgb.id(), 2);
    assert_eq!(RefloatLedColorOrder::Wrgb.id(), 3);
    assert_eq!(RefloatLedStripOrder::None.id(), 0);
    assert_eq!(RefloatLedStripOrder::First.id(), 1);
    assert_eq!(RefloatLedStripOrder::Second.id(), 2);
    assert_eq!(RefloatLedStripOrder::Third.id(), 3);
    assert_eq!(strip.order(), RefloatLedStripOrder::Second);
    assert_eq!(strip.count(), 24);
    assert_eq!(strip.color_order(), RefloatLedColorOrder::Grbw);
    assert!(strip.is_reversed());
}

#[test]
fn package_author_reads_led_bar_config_without_raw_ids() {
    let bar = RefloatLedBarConfig::new(
        Ratio::from_ratio_const(0.8),
        RefloatLedColor::Gold,
        RefloatLedColor::Black,
        RefloatLedAnimationMode::Pulse,
        RefloatLedAnimationSpeed::from_units(1.5),
    );

    let color_ids = [
        (RefloatLedColor::Black, 0),
        (RefloatLedColor::WhiteFull, 1),
        (RefloatLedColor::WhiteRgb, 2),
        (RefloatLedColor::WhiteSingle, 3),
        (RefloatLedColor::Red, 4),
        (RefloatLedColor::Ferrari, 5),
        (RefloatLedColor::Flame, 6),
        (RefloatLedColor::Coral, 7),
        (RefloatLedColor::Sunset, 8),
        (RefloatLedColor::Sunrise, 9),
        (RefloatLedColor::Gold, 10),
        (RefloatLedColor::Orange, 11),
        (RefloatLedColor::Yellow, 12),
        (RefloatLedColor::Banana, 13),
        (RefloatLedColor::Lime, 14),
        (RefloatLedColor::Acid, 15),
        (RefloatLedColor::Sage, 16),
        (RefloatLedColor::Green, 17),
        (RefloatLedColor::Mint, 18),
        (RefloatLedColor::Tiffany, 19),
        (RefloatLedColor::Cyan, 20),
        (RefloatLedColor::Steel, 21),
        (RefloatLedColor::Sky, 22),
        (RefloatLedColor::Azure, 23),
        (RefloatLedColor::Sapphire, 24),
        (RefloatLedColor::Blue, 25),
        (RefloatLedColor::Violet, 26),
        (RefloatLedColor::Amethyst, 27),
        (RefloatLedColor::Magenta, 28),
        (RefloatLedColor::Pink, 29),
        (RefloatLedColor::Fuchsia, 30),
        (RefloatLedColor::Lavender, 31),
    ];
    let animation_ids = [
        (RefloatLedAnimationMode::Solid, 0),
        (RefloatLedAnimationMode::Fade, 1),
        (RefloatLedAnimationMode::Pulse, 2),
        (RefloatLedAnimationMode::Strobe, 3),
        (RefloatLedAnimationMode::KnightRider, 4),
        (RefloatLedAnimationMode::Felony, 5),
        (RefloatLedAnimationMode::RainbowCycle, 6),
        (RefloatLedAnimationMode::RainbowFade, 7),
        (RefloatLedAnimationMode::RainbowRoll, 8),
    ];
    let transition_ids = [
        (RefloatLedTransition::Fade, 0),
        (RefloatLedTransition::FadeOutIn, 1),
        (RefloatLedTransition::Cipher, 2),
        (RefloatLedTransition::MonoCipher, 3),
    ];

    assert!(
        color_ids
            .iter()
            .all(|(color, expected)| color.id() == *expected)
    );
    assert!(
        animation_ids
            .iter()
            .all(|(mode, expected)| mode.id() == *expected)
    );
    assert!(
        transition_ids
            .iter()
            .all(|(transition, expected)| transition.id() == *expected)
    );
    assert!((bar.brightness().as_ratio() - 0.8).abs() < f32::EPSILON);
    assert_eq!(bar.primary_color(), RefloatLedColor::Gold);
    assert_eq!(bar.secondary_color(), RefloatLedColor::Black);
    assert_eq!(bar.animation_mode(), RefloatLedAnimationMode::Pulse);
    assert!((bar.animation_speed().as_units() - 1.5).abs() < f32::EPSILON);
}

#[test]
fn package_author_reads_status_bar_config_without_raw_scalars() {
    let status = RefloatStatusBarConfig::new(
        RefloatStatusBarIdleTimeout::from_seconds(30),
        Ratio::from_ratio_const(0.12),
        Ratio::from_ratio_const(0.25),
        Ratio::from_ratio_const(0.70),
        Ratio::from_ratio_const(0.20),
    )
    .showing_sensors_while_running();

    assert_eq!(status.idle_timeout().as_seconds(), 30);
    assert!((status.duty_threshold().as_ratio() - 0.12).abs() < f32::EPSILON);
    assert!((status.red_bar_percentage().as_ratio() - 0.25).abs() < f32::EPSILON);
    assert!(status.shows_sensors_while_running());
    assert!((status.brightness_headlights_on().as_ratio() - 0.70).abs() < f32::EPSILON);
    assert!((status.brightness_headlights_off().as_ratio() - 0.20).abs() < f32::EPSILON);
}

#[test]
fn package_author_composes_leds_config_without_raw_flags() {
    let headlights = RefloatLedBarConfig::new(
        Ratio::from_ratio_const(0.9),
        RefloatLedColor::WhiteFull,
        RefloatLedColor::Black,
        RefloatLedAnimationMode::Solid,
        RefloatLedAnimationSpeed::from_units(1.0),
    );
    let taillights = RefloatLedBarConfig::new(
        Ratio::from_ratio_const(0.5),
        RefloatLedColor::Red,
        RefloatLedColor::Black,
        RefloatLedAnimationMode::Pulse,
        RefloatLedAnimationSpeed::from_units(1.5),
    );
    let status = RefloatStatusBarConfig::new(
        RefloatStatusBarIdleTimeout::from_seconds(45),
        Ratio::from_ratio_const(0.10),
        Ratio::from_ratio_const(0.20),
        Ratio::from_ratio_const(0.75),
        Ratio::from_ratio_const(0.25),
    );

    let leds = RefloatLedsConfig::new(
        headlights, taillights, headlights, taillights, status, taillights,
    )
    .with_headlights_transition(RefloatLedTransition::FadeOutIn)
    .with_direction_transition(RefloatLedTransition::Cipher)
    .enabled()
    .with_headlights_on()
    .lights_off_when_lifted()
    .status_on_front_when_lifted();

    assert!(leds.is_enabled());
    assert!(leds.are_headlights_on());
    assert_eq!(
        leds.headlights_transition(),
        RefloatLedTransition::FadeOutIn
    );
    assert_eq!(leds.direction_transition(), RefloatLedTransition::Cipher);
    assert!(leds.turns_lights_off_when_lifted());
    assert!(leds.shows_status_on_front_when_lifted());
    assert_eq!(
        leds.headlights().primary_color(),
        RefloatLedColor::WhiteFull
    );
    assert_eq!(leds.taillights().primary_color(), RefloatLedColor::Red);
    assert_eq!(
        leds.front().animation_mode(),
        RefloatLedAnimationMode::Solid
    );
    assert_eq!(leds.rear().animation_mode(), RefloatLedAnimationMode::Pulse);
    assert_eq!(leds.status().idle_timeout().as_seconds(), 45);
    assert_eq!(leds.status_idle().primary_color(), RefloatLedColor::Red);
}
