use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_DATA_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS, FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS,
    FloatOutBoyAlertId, FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataBatteryTemperature, FloatOutBoyAllDataMode, FloatOutBoyAllDataMode2Payload,
    FloatOutBoyAllDataMode3Payload, FloatOutBoyAllDataMode4Payload, FloatOutBoyAllDataMotorPayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataRequest, FloatOutBoyAllDataRequestError,
    FloatOutBoyAllDataResponse, FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand,
    FloatOutBoyBeepReason, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState, FloatOutBoyFocIdCurrent,
    FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyMotorCommand,
    FloatOutBoyRealtimeAlertMask, FloatOutBoyRealtimeAlwaysPayload,
    FloatOutBoyRealtimeAtrAccelerationDiff, FloatOutBoyRealtimeAtrSpeedBoost,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeChargingCurrent,
    FloatOutBoyRealtimeChargingPayload, FloatOutBoyRealtimeChargingVoltage,
    FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem, FloatOutBoyRealtimeDataItemGroup,
    FloatOutBoyRealtimeDataRecordPolicy, FloatOutBoyRealtimeFilteredMotorCurrent,
    FloatOutBoyRealtimeImuPayload, FloatOutBoyRealtimeMotorCurrents,
    FloatOutBoyRealtimeMotorPayload, FloatOutBoyRealtimeMotorTemperatures,
    FloatOutBoyRealtimeRemoteInput, FloatOutBoyRealtimeReservedFlags,
    FloatOutBoyRealtimeRuntimeAtrPayload, FloatOutBoyRealtimeRuntimePayload,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyRealtimeTail, FloatOutBoyRideState, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};
use crate::leds::{
    FloatOutBoyLedAnimationMode, FloatOutBoyLedAnimationSpeed, FloatOutBoyLedBarConfig,
    FloatOutBoyLedColor, FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
    FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder, FloatOutBoyLedTransition,
    FloatOutBoyLedsConfig, FloatOutBoyStatusBarConfig, FloatOutBoyStatusBarIdleTimeout,
};
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::LoaderInfo;

#[test]
fn test_package_lib_init_uses_side_effect_free_registration_tail() {
    let mut info = LoaderInfo::new();

    assert!(crate::package_lib_init(&raw mut info));
    // Upstream Float Out Boy v1.2.1 installs `stop`/`Data *` at
    // `third_party/float-out-boy/src/main.c:2431-2432` before the registration tail at
    // `third_party/float-out-boy/src/main.c:2456-2459`; the test build keeps that tail side-effect free.
    assert!(!info.has_stop_handler());
    assert!(info.argument().is_none());
}

#[test]
fn package_author_builds_source_startup_all_data_payload() {
    let payloads = FloatOutBoyAllDataPayloads::source_startup();
    let response = payloads.encode_response(FloatOutBoyAllDataRequest::new(
        FloatOutBoyAllDataMode::with_mode4(),
    ));

    assert_eq!(
        payloads.base().status(),
        FloatOutBoyAllDataStatus::new(
            FloatOutBoyRideState::new(
                FloatOutBoyRunState::Startup,
                FloatOutBoyMode::Normal,
                FloatOutBoySetpointAdjustment::None,
                FloatOutBoyStopCondition::None,
            ),
            FloatOutBoyBeepReason::None,
        )
    );
    assert_eq!(
        payloads.base().footpad().state(),
        FloatOutBoyFootpadState::None
    );
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
fn package_author_models_float_out_boy_ride_inputs_without_raw_float_handoff() {
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.65),
        Voltage::from_volts(0.72),
        FloatOutBoyFootpadState::Both,
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

    assert_eq!(footpad.state(), FloatOutBoyFootpadState::Both);
    assert_f32_eq!(attitude.pitch().angle().as_radians(), 0.03);
    assert_f32_eq!(angular_rate.roll().as_degrees_per_second(), 12.0);
    assert_f32_eq!(electrical_speed.rpm().as_revolutions_per_minute(), 2400.0);
    assert_f32_eq!(motor_current.current().as_amps(), 8.0);
    assert_f32_eq!(battery_current.current().as_amps(), 3.0);
}

#[test]
fn package_author_requests_float_out_boy_motor_current_with_domain_intent() {
    fn apply_requested_current(command: FloatOutBoyMotorCommand) -> MotorCurrent {
        command.requested_current()
    }

    let command = FloatOutBoyMotorCommand::new(MotorCurrent::new(Current::from_amps(11.0)));

    assert_f32_eq!(apply_requested_current(command).current().as_amps(), 11.0);
}

#[test]
fn package_author_reads_float_out_boy_state_as_enums_not_bool_or_integer_flags() {
    let ready_pitch_fault = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::Pitch,
    );
    let running_tiltback = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Flywheel,
        FloatOutBoySetpointAdjustment::PushbackHighVoltage,
        FloatOutBoyStopCondition::None,
    );

    assert_eq!(ready_pitch_fault.float_state_compat(), 6);
    assert_eq!(running_tiltback.float_state_compat(), 2);
    assert_eq!(running_tiltback.setpoint_adjustment_compat(), 4);
    assert_eq!(
        running_tiltback
            .with_wheelslip(FloatOutBoyWheelSlipState::Detected)
            .float_state_compat(),
        2
    );
    assert_eq!(
        running_tiltback
            .with_charging(FloatOutBoyChargingState::Charging)
            .with_wheelslip(FloatOutBoyWheelSlipState::Detected)
            .float_state_compat(),
        14
    );
}

#[test]
fn package_author_parses_float_out_boy_app_data_commands_as_domain_enum() {
    let commands = [
        (0, FloatOutBoyAppDataCommand::Info),
        (1, FloatOutBoyAppDataCommand::GetRealtimeData),
        (2, FloatOutBoyAppDataCommand::RuntimeTune),
        (3, FloatOutBoyAppDataCommand::TuneDefaults),
        (4, FloatOutBoyAppDataCommand::ConfigSave),
        (5, FloatOutBoyAppDataCommand::ConfigRestore),
        (6, FloatOutBoyAppDataCommand::TuneOther),
        (7, FloatOutBoyAppDataCommand::RcMove),
        (8, FloatOutBoyAppDataCommand::Booster),
        (9, FloatOutBoyAppDataCommand::PrintInfo),
        (10, FloatOutBoyAppDataCommand::GetAllData),
        (11, FloatOutBoyAppDataCommand::Experiment),
        (12, FloatOutBoyAppDataCommand::Lock),
        (13, FloatOutBoyAppDataCommand::HandTest),
        (14, FloatOutBoyAppDataCommand::TuneTilt),
        (20, FloatOutBoyAppDataCommand::LightsControl),
        (22, FloatOutBoyAppDataCommand::Flywheel),
        (24, FloatOutBoyAppDataCommand::LcmPoll),
        (25, FloatOutBoyAppDataCommand::LcmLightInfo),
        (26, FloatOutBoyAppDataCommand::LcmLightControl),
        (27, FloatOutBoyAppDataCommand::LcmDeviceInfo),
        (28, FloatOutBoyAppDataCommand::ChargingState),
        (29, FloatOutBoyAppDataCommand::LcmGetBattery),
        (31, FloatOutBoyAppDataCommand::RealtimeData),
        (32, FloatOutBoyAppDataCommand::RealtimeDataIds),
        (35, FloatOutBoyAppDataCommand::AlertsList),
        (36, FloatOutBoyAppDataCommand::AlertsControl),
        (41, FloatOutBoyAppDataCommand::DataRecordRequest),
        (99, FloatOutBoyAppDataCommand::LcmDebug),
    ];

    assert_eq!(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(), 101);
    assert!(commands.into_iter().all(|(id, command)| {
        FloatOutBoyAppDataCommand::try_from_id(id)
            .is_ok_and(|parsed| parsed == command && parsed.id() == id)
    }));
    assert_eq!(
        FloatOutBoyAppDataCommand::try_from_id(200)
            .expect_err("unstable command should stay explicit")
            .value(),
        200
    );
}

#[test]
fn package_author_parses_all_data_requests_without_raw_packet_checks() {
    let request = FloatOutBoyAllDataRequest::parse(&[
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
        FloatOutBoyAppDataCommand::GetAllData.id(),
        4,
    ])
    .expect("all-data mode 4 request should parse");

    assert_eq!(request.mode(), FloatOutBoyAllDataMode::with_mode4());
    assert_eq!(request.mode().source_id(), 4);
    assert!(request.mode().includes_mode2());
    assert!(request.mode().includes_mode3());
    assert!(request.mode().includes_mode4());
    assert_eq!(
        FloatOutBoyAllDataRequest::parse(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::GetAllData.id()
        ])
        .expect_err("truncated request should be rejected"),
        FloatOutBoyAllDataRequestError::Length { actual: 2 }
    );
    assert_eq!(
        FloatOutBoyAllDataRequest::parse(&[102, FloatOutBoyAppDataCommand::GetAllData.id(), 4])
            .expect_err("wrong package ID should be rejected"),
        FloatOutBoyAllDataRequestError::PackageId { value: 102 }
    );
    assert_eq!(
        FloatOutBoyAllDataRequest::parse(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::PrintInfo.id(),
            4
        ])
        .expect_err("wrong command should be rejected"),
        FloatOutBoyAllDataRequestError::Command { value: 9 }
    );
    assert_eq!(
        FloatOutBoyAllDataRequest::parse(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::GetAllData.id(),
            9
        ])
        .expect("Float Out Boy source accepts high all-data modes")
        .mode()
        .source_id(),
        9
    );
}

#[test]
fn package_author_builds_realtime_data_header_without_raw_bit_flags() {
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Flywheel,
        FloatOutBoySetpointAdjustment::PushbackLowVoltage,
        FloatOutBoyStopCondition::QuickStop,
    )
    .with_charging(FloatOutBoyChargingState::Charging)
    .with_wheelslip(FloatOutBoyWheelSlipState::Detected)
    .with_darkride(FloatOutBoyDarkRideState::Active);
    let recorder = FloatOutBoyDataRecorderFlags::inactive()
        .with_recording()
        .with_autostop();
    let header = FloatOutBoyRealtimeDataHeader::new(
        TimestampTicks::from_ticks(123_456),
        ride_state,
        FloatOutBoyFootpadState::Both,
        FloatOutBoyBeepReason::FirmwareFault,
    )
    .with_data_recorder(recorder)
    .with_fatal_error(FloatOutBoyFatalErrorState::Present);

    assert_eq!(header.timestamp().as_ticks(), 123_456);
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
        FLOAT_OUT_BOY_REALTIME_DATA_ITEMS.map(FloatOutBoyRealtimeDataItem::id),
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
        FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS.map(FloatOutBoyRealtimeDataItem::id),
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
        FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS.map(FloatOutBoyRealtimeDataItem::id),
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
        FloatOutBoyRealtimeDataItem::MotorSpeed.group(),
        FloatOutBoyRealtimeDataItemGroup::Always
    );
    assert_eq!(
        FloatOutBoyRealtimeDataItem::BalanceCurrent.group(),
        FloatOutBoyRealtimeDataItemGroup::Runtime
    );
    assert_eq!(
        FloatOutBoyRealtimeDataItem::MotorErpm.record_policy(),
        FloatOutBoyRealtimeDataRecordPolicy::Record
    );
    assert_eq!(
        FloatOutBoyRealtimeDataItem::MotorSpeed.record_policy(),
        FloatOutBoyRealtimeDataRecordPolicy::SendOnly
    );
}

#[test]
fn package_author_builds_realtime_always_payload_without_raw_values() {
    let motor = FloatOutBoyRealtimeMotorPayload::new(
        VehicleSpeed::new(Speed::from_kilometers_per_hour(12.6)),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0)),
        FloatOutBoyRealtimeMotorCurrents::new(
            MotorCurrent::new(Current::from_amps(7.0)),
            DirectionalMotorCurrent::new(Current::from_amps(-6.75)),
            FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                Current::from_amps(-6.5),
            )),
            BatteryCurrent::new(Current::from_amps(3.5)),
        ),
        DutyCycle::new(SignedRatio::from_ratio_const(0.21)),
        BatteryVoltage::new(Voltage::from_volts(73.0)),
        FloatOutBoyRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(41.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(52.0)),
        ),
    );
    let imu = FloatOutBoyRealtimeImuPayload::new(
        ImuPitch::new(AngleRadians::from_radians(0.04)),
        FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.03)),
        ImuRoll::new(AngleRadians::from_radians(-0.02)),
    );
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.61),
        Voltage::from_volts(0.58),
        FloatOutBoyFootpadState::Both,
    );
    let payload = FloatOutBoyRealtimeAlwaysPayload::new(
        motor,
        imu,
        footpad,
        FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.18)),
    );

    assert_eq!(
        payload.item_contract().map(FloatOutBoyRealtimeDataItem::id),
        FLOAT_OUT_BOY_REALTIME_DATA_ITEMS.map(FloatOutBoyRealtimeDataItem::id)
    );
    assert_f32_eq!(
        payload.motor().speed().speed().as_kilometers_per_hour(),
        12.6
    );
    assert_f32_eq!(
        payload
            .motor()
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        2400.0
    );
    assert_f32_eq!(
        payload
            .motor()
            .currents()
            .filtered()
            .current()
            .current()
            .as_amps(),
        -6.5
    );
    assert_f32_eq!(
        payload
            .motor()
            .temperatures()
            .motor()
            .temperature()
            .as_degrees_celsius(),
        52.0
    );
    assert_f32_eq!(payload.imu().balance_pitch().angle().as_radians(), 0.03);
    assert_eq!(payload.footpad().state(), FloatOutBoyFootpadState::Both);
    assert_f32_eq!(payload.remote_input().ratio().as_ratio(), 0.18);
}

#[test]
fn package_author_builds_realtime_runtime_payload_without_raw_values() {
    let payload = FloatOutBoyRealtimeRuntimePayload::new(
        FloatOutBoyRealtimeRuntimeSetpoints::new(
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.5)),
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.25)),
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.5)),
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.75)),
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.125)),
            FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        ),
        FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.5))),
        FloatOutBoyRealtimeRuntimeAtrPayload::new(
            FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(12.0),
            FloatOutBoyRealtimeAtrSpeedBoost::from_units(-0.1),
        ),
        FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(1.25))),
    );

    assert_eq!(
        payload.item_contract().map(FloatOutBoyRealtimeDataItem::id),
        FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS.map(FloatOutBoyRealtimeDataItem::id)
    );
    assert_f32_eq!(payload.setpoints().board().angle().as_degrees(), 1.5);
    assert_f32_eq!(payload.setpoints().brake_tilt().angle().as_degrees(), -0.5);
    assert_f32_eq!(payload.balance_current().current().current().as_amps(), 9.5);
    assert_f32_eq!(payload.atr().accel_diff().as_erpm_delta(), 12.0);
    assert_f32_eq!(payload.atr().speed_boost().as_units(), -0.1);
    assert_f32_eq!(
        payload.booster_current().current().current().as_amps(),
        1.25
    );
}

#[test]
fn package_author_builds_realtime_charging_and_tail_without_raw_values() {
    let charging = FloatOutBoyRealtimeChargingPayload::new(
        FloatOutBoyRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(4.2))),
        FloatOutBoyRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.5))),
    );
    let tail = FloatOutBoyRealtimeTail::new(
        FloatOutBoyRealtimeAlertMask::empty().with_alert(FloatOutBoyAlertId::FirmwareFault),
        FloatOutBoyRealtimeReservedFlags::none(),
        FirmwareFaultWireCode::from_wire_code(12),
    );

    assert_f32_eq!(charging.current().current().current().as_amps(), 4.2);
    assert_f32_eq!(charging.voltage().voltage().voltage().as_volts(), 82.5);
    assert!(
        tail.active_alerts()
            .contains(FloatOutBoyAlertId::FirmwareFault)
    );
    assert_eq!(tail.active_alerts().active_alert_mask_compat(), 0x1);
    assert_eq!(tail.reserved_flags().extra_flags_compat(), 0);
    assert_eq!(tail.firmware_fault_code().wire_code(), 12);
}

#[test]
fn package_author_builds_all_data_base_payload_without_raw_values() {
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    );
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.62),
        Voltage::from_volts(0.57),
        FloatOutBoyFootpadState::Both,
    );
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.5)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.25)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.5)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.75)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-0.125)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
    );
    let motor = FloatOutBoyAllDataMotorPayload::new(
        BatteryVoltage::new(Voltage::from_volts(73.5)),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2420.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(3.4)),
        FloatOutBoyRealtimeMotorCurrents::new(
            MotorCurrent::new(Current::from_amps(8.25)),
            DirectionalMotorCurrent::new(Current::from_amps(8.25)),
            FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                Current::from_amps(8.25),
            )),
            BatteryCurrent::new(Current::from_amps(3.75)),
        ),
        DutyCycle::new(SignedRatio::from_ratio_const(0.34)),
        FloatOutBoyFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(1.5))),
    );
    let payload = FloatOutBoyAllDataBasePayload::new(
        FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.5))),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.03)),
            ImuRoll::new(AngleRadians::from_radians(-0.02)),
            ImuPitch::new(AngleRadians::from_radians(0.04)),
        ),
        FloatOutBoyAllDataStatus::new(ride_state, FloatOutBoyBeepReason::LowVoltage),
        footpad,
        setpoints,
        FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(1.25))),
        motor,
    );

    assert_eq!(payload.command(), FloatOutBoyAppDataCommand::GetAllData);
    assert_eq!(payload.status().ride_state().float_state_compat(), 1);
    assert_eq!(payload.status().beep_reason().id(), 1);
    assert_eq!(payload.footpad().state(), FloatOutBoyFootpadState::Both);
    assert_f32_eq!(payload.setpoints().remote().angle().as_degrees(), 2.0);
    assert_f32_eq!(payload.attitude().pitch().angle().as_radians(), 0.04);
    assert_f32_eq!(payload.balance_current().current().current().as_amps(), 9.5);
    assert_f32_eq!(
        payload.booster_current().current().current().as_amps(),
        1.25
    );
    assert_f32_eq!(
        payload
            .motor()
            .vehicle_speed()
            .speed()
            .as_meters_per_second(),
        3.4
    );
    assert_f32_eq!(
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
    let mode2 = FloatOutBoyAllDataMode2Payload::new(
        TripDistance::new(Distance::from_meters(42.5)),
        FloatOutBoyRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
        ),
        FloatOutBoyAllDataBatteryTemperature::unavailable(),
    );
    let mode3 = FloatOutBoyAllDataMode3Payload::new(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::from_fraction(0.72),
    );
    let mode4 = FloatOutBoyAllDataMode4Payload::new(
        FloatOutBoyRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
        FloatOutBoyRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
    );

    assert_f32_eq!(mode2.distance_abs().distance().as_meters(), 42.5);
    assert_f32_eq!(
        mode2
            .temperatures()
            .mosfet()
            .temperature()
            .as_degrees_celsius(),
        44.0
    );
    assert!(mode2.battery_temperature().as_measured().is_none());
    assert_eq!(mode3.odometer().as_meters(), 123_456);
    assert_f32_eq!(mode3.discharged_charge().charge().as_amp_hours(), 3.2);
    assert_f32_eq!(mode3.charged_charge().charge().as_amp_hours(), 0.8);
    assert_f32_eq!(mode3.discharged_energy().energy().as_watt_hours(), 170.0);
    assert_f32_eq!(mode3.charged_energy().energy().as_watt_hours(), 18.5);
    assert_f32_eq!(mode3.battery_level().as_fraction(), 0.72);
    assert_f32_eq!(mode4.current().current().current().as_amps(), 1.2);
    assert_f32_eq!(mode4.voltage().voltage().voltage().as_volts(), 82.4);
}

#[test]
fn package_author_encodes_all_data_base_response_like_float_out_boy_v1_2_1() {
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    );
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        FloatOutBoyFootpadState::Both,
    );
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payload = FloatOutBoyAllDataBasePayload::new(
        FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
            ImuRoll::new(AngleRadians::from_radians(-0.5)),
            ImuPitch::new(AngleRadians::from_radians(2.3)),
        ),
        FloatOutBoyAllDataStatus::new(ride_state, FloatOutBoyBeepReason::LowVoltage),
        footpad,
        setpoints,
        FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
        FloatOutBoyAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
            FloatOutBoyRealtimeMotorCurrents::new(
                MotorCurrent::new(Current::from_amps(5.0)),
                DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                    Current::from_amps(5.0),
                )),
                BatteryCurrent::new(Current::from_amps(-2.0)),
            ),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
            FloatOutBoyFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
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
fn package_author_encodes_all_data_fault_response_like_float_out_boy_v1_2_1() {
    assert_eq!(
        FloatOutBoyAllDataResponse::fault(FirmwareFaultWireCode::from_wire_code(5)).as_bytes(),
        &[101, 10, 69, 5]
    );
}

#[test]
fn package_author_encodes_all_data_mode4_response_like_float_out_boy_v1_2_1() {
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    );
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        FloatOutBoyFootpadState::Both,
    );
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payload = FloatOutBoyAllDataBasePayload::new(
        FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
        FloatOutBoyAllDataAttitude::new(
            FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
            ImuRoll::new(AngleRadians::from_radians(-0.5)),
            ImuPitch::new(AngleRadians::from_radians(2.3)),
        ),
        FloatOutBoyAllDataStatus::new(ride_state, FloatOutBoyBeepReason::LowVoltage),
        footpad,
        setpoints,
        FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
        FloatOutBoyAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
            FloatOutBoyRealtimeMotorCurrents::new(
                MotorCurrent::new(Current::from_amps(5.0)),
                DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                    Current::from_amps(5.0),
                )),
                BatteryCurrent::new(Current::from_amps(-2.0)),
            ),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
            FloatOutBoyFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
        ),
    );
    let mode2 = FloatOutBoyAllDataMode2Payload::new(
        TripDistance::new(Distance::from_meters(64.0)),
        FloatOutBoyRealtimeMotorTemperatures::new(
            MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
        ),
        FloatOutBoyAllDataBatteryTemperature::unavailable(),
    );
    let mode3 = FloatOutBoyAllDataMode3Payload::new(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::from_fraction(0.72),
    );
    let mode4 = FloatOutBoyAllDataMode4Payload::new(
        FloatOutBoyRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
        FloatOutBoyRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
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
    let ride_state = FloatOutBoyRideState::new(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
        FloatOutBoySetpointAdjustment::None,
        FloatOutBoyStopCondition::None,
    );
    let footpad = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.60),
        Voltage::from_volts(0.40),
        FloatOutBoyFootpadState::Both,
    );
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-1.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0)),
        FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0)),
    );
    let payloads = FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(9.0))),
            FloatOutBoyAllDataAttitude::new(
                FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(1.2)),
                ImuRoll::new(AngleRadians::from_radians(-0.5)),
                ImuPitch::new(AngleRadians::from_radians(2.3)),
            ),
            FloatOutBoyAllDataStatus::new(ride_state, FloatOutBoyBeepReason::LowVoltage),
            footpad,
            setpoints,
            FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(4.0))),
            FloatOutBoyAllDataMotorPayload::new(
                BatteryVoltage::new(Voltage::from_volts(72.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(3.0)),
                FloatOutBoyRealtimeMotorCurrents::new(
                    MotorCurrent::new(Current::from_amps(5.0)),
                    DirectionalMotorCurrent::new(Current::from_amps(5.0)),
                    FloatOutBoyRealtimeFilteredMotorCurrent::new(DirectionalMotorCurrent::new(
                        Current::from_amps(5.0),
                    )),
                    BatteryCurrent::new(Current::from_amps(-2.0)),
                ),
                DutyCycle::new(SignedRatio::from_ratio_const(-0.25)),
                FloatOutBoyFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(2.0))),
            ),
        ),
        FloatOutBoyAllDataMode2Payload::new(
            TripDistance::new(Distance::from_meters(64.0)),
            FloatOutBoyRealtimeMotorTemperatures::new(
                MosfetTemperature::new(Temperature::from_degrees_celsius(44.0)),
                MotorTemperature::new(Temperature::from_degrees_celsius(51.5)),
            ),
            FloatOutBoyAllDataBatteryTemperature::unavailable(),
        ),
        FloatOutBoyAllDataMode3Payload::new(
            OdometerMeters::from_meters(123_456),
            AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
            AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
            WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
            WattHoursCharged::new(Energy::from_watt_hours(18.5)),
            BatteryLevel::from_fraction(0.72),
        ),
        FloatOutBoyAllDataMode4Payload::new(
            FloatOutBoyRealtimeChargingCurrent::new(BatteryCurrent::new(Current::from_amps(1.2))),
            FloatOutBoyRealtimeChargingVoltage::new(BatteryVoltage::new(Voltage::from_volts(82.4))),
        ),
    );

    assert_eq!(
        payloads
            .encode_response(FloatOutBoyAllDataRequest::new(
                FloatOutBoyAllDataMode::base()
            ))
            .as_bytes(),
        &[
            101, 10, 1, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(FloatOutBoyAllDataRequest::new(
                FloatOutBoyAllDataMode::with_mode2()
            ))
            .as_bytes(),
        &[
            101, 10, 2, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(FloatOutBoyAllDataRequest::new(
                FloatOutBoyAllDataMode::with_mode3()
            ))
            .as_bytes(),
        &[
            101, 10, 3, 0, 90, 2, 175, 254, 226, 33, 18, 30, 20, 133, 128, 123, 138, 118, 143, 5,
            37, 132, 2, 208, 4, 176, 0, 30, 0, 50, 255, 236, 103, 6, 66, 128, 0, 0, 88, 103, 0, 0,
            1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144,
        ]
    );
    assert_eq!(
        payloads
            .encode_response(FloatOutBoyAllDataRequest::new(
                FloatOutBoyAllDataMode::from_source_id(9)
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
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Second,
        24,
        FloatOutBoyLedColorOrder::Grbw,
    )
    .with_reverse(true);

    assert_eq!(FloatOutBoyLedPin::B6.id(), 0);
    assert_eq!(FloatOutBoyLedPin::B7.id(), 1);
    assert_eq!(FloatOutBoyLedPin::C9.id(), 2);
    assert_eq!(FloatOutBoyLedPinConfig::PullupTo5v.id(), 0);
    assert_eq!(FloatOutBoyLedPinConfig::NoPullup.id(), 1);
    assert_eq!(FloatOutBoyLedColorOrder::Grb.id(), 0);
    assert_eq!(FloatOutBoyLedColorOrder::Grbw.id(), 1);
    assert_eq!(FloatOutBoyLedColorOrder::Rgb.id(), 2);
    assert_eq!(FloatOutBoyLedColorOrder::Wrgb.id(), 3);
    assert_eq!(FloatOutBoyLedStripOrder::None.id(), 0);
    assert_eq!(FloatOutBoyLedStripOrder::First.id(), 1);
    assert_eq!(FloatOutBoyLedStripOrder::Second.id(), 2);
    assert_eq!(FloatOutBoyLedStripOrder::Third.id(), 3);
    assert_eq!(strip.order(), FloatOutBoyLedStripOrder::Second);
    assert_eq!(strip.count(), 24);
    assert_eq!(strip.color_order(), FloatOutBoyLedColorOrder::Grbw);
    assert!(strip.is_reversed());
}

#[test]
fn package_author_reads_led_bar_config_without_raw_ids() {
    let bar = FloatOutBoyLedBarConfig::new(
        Ratio::from_ratio_const(0.8),
        FloatOutBoyLedColor::Gold,
        FloatOutBoyLedColor::Black,
        FloatOutBoyLedAnimationMode::Pulse,
        FloatOutBoyLedAnimationSpeed::from_units(1.5),
    );

    let color_ids = [
        (FloatOutBoyLedColor::Black, 0),
        (FloatOutBoyLedColor::WhiteFull, 1),
        (FloatOutBoyLedColor::WhiteRgb, 2),
        (FloatOutBoyLedColor::WhiteSingle, 3),
        (FloatOutBoyLedColor::Red, 4),
        (FloatOutBoyLedColor::Ferrari, 5),
        (FloatOutBoyLedColor::Flame, 6),
        (FloatOutBoyLedColor::Coral, 7),
        (FloatOutBoyLedColor::Sunset, 8),
        (FloatOutBoyLedColor::Sunrise, 9),
        (FloatOutBoyLedColor::Gold, 10),
        (FloatOutBoyLedColor::Orange, 11),
        (FloatOutBoyLedColor::Yellow, 12),
        (FloatOutBoyLedColor::Banana, 13),
        (FloatOutBoyLedColor::Lime, 14),
        (FloatOutBoyLedColor::Acid, 15),
        (FloatOutBoyLedColor::Sage, 16),
        (FloatOutBoyLedColor::Green, 17),
        (FloatOutBoyLedColor::Mint, 18),
        (FloatOutBoyLedColor::Tiffany, 19),
        (FloatOutBoyLedColor::Cyan, 20),
        (FloatOutBoyLedColor::Steel, 21),
        (FloatOutBoyLedColor::Sky, 22),
        (FloatOutBoyLedColor::Azure, 23),
        (FloatOutBoyLedColor::Sapphire, 24),
        (FloatOutBoyLedColor::Blue, 25),
        (FloatOutBoyLedColor::Violet, 26),
        (FloatOutBoyLedColor::Amethyst, 27),
        (FloatOutBoyLedColor::Magenta, 28),
        (FloatOutBoyLedColor::Pink, 29),
        (FloatOutBoyLedColor::Fuchsia, 30),
        (FloatOutBoyLedColor::Lavender, 31),
    ];
    let animation_ids = [
        (FloatOutBoyLedAnimationMode::Solid, 0),
        (FloatOutBoyLedAnimationMode::Fade, 1),
        (FloatOutBoyLedAnimationMode::Pulse, 2),
        (FloatOutBoyLedAnimationMode::Strobe, 3),
        (FloatOutBoyLedAnimationMode::KnightRider, 4),
        (FloatOutBoyLedAnimationMode::Felony, 5),
        (FloatOutBoyLedAnimationMode::RainbowCycle, 6),
        (FloatOutBoyLedAnimationMode::RainbowFade, 7),
        (FloatOutBoyLedAnimationMode::RainbowRoll, 8),
    ];
    let transition_ids = [
        (FloatOutBoyLedTransition::Fade, 0),
        (FloatOutBoyLedTransition::FadeOutIn, 1),
        (FloatOutBoyLedTransition::Cipher, 2),
        (FloatOutBoyLedTransition::MonoCipher, 3),
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
    assert_eq!(bar.primary_color(), FloatOutBoyLedColor::Gold);
    assert_eq!(bar.secondary_color(), FloatOutBoyLedColor::Black);
    assert_eq!(bar.animation_mode(), FloatOutBoyLedAnimationMode::Pulse);
    assert!((bar.animation_speed().as_units() - 1.5).abs() < f32::EPSILON);
}

#[test]
fn package_author_reads_status_bar_config_without_raw_scalars() {
    let status = FloatOutBoyStatusBarConfig::new(
        FloatOutBoyStatusBarIdleTimeout::from_seconds(30),
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
    let headlights = FloatOutBoyLedBarConfig::new(
        Ratio::from_ratio_const(0.9),
        FloatOutBoyLedColor::WhiteFull,
        FloatOutBoyLedColor::Black,
        FloatOutBoyLedAnimationMode::Solid,
        FloatOutBoyLedAnimationSpeed::from_units(1.0),
    );
    let taillights = FloatOutBoyLedBarConfig::new(
        Ratio::from_ratio_const(0.5),
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Black,
        FloatOutBoyLedAnimationMode::Pulse,
        FloatOutBoyLedAnimationSpeed::from_units(1.5),
    );
    let status = FloatOutBoyStatusBarConfig::new(
        FloatOutBoyStatusBarIdleTimeout::from_seconds(45),
        Ratio::from_ratio_const(0.10),
        Ratio::from_ratio_const(0.20),
        Ratio::from_ratio_const(0.75),
        Ratio::from_ratio_const(0.25),
    );

    let leds = FloatOutBoyLedsConfig::new(
        headlights, taillights, headlights, taillights, status, taillights,
    )
    .with_headlights_transition(FloatOutBoyLedTransition::FadeOutIn)
    .with_direction_transition(FloatOutBoyLedTransition::Cipher)
    .enabled()
    .with_headlights_on()
    .lights_off_when_lifted()
    .status_on_front_when_lifted();

    assert!(leds.is_enabled());
    assert!(leds.are_headlights_on());
    assert_eq!(
        leds.headlights_transition(),
        FloatOutBoyLedTransition::FadeOutIn
    );
    assert_eq!(
        leds.direction_transition(),
        FloatOutBoyLedTransition::Cipher
    );
    assert!(leds.turns_lights_off_when_lifted());
    assert!(leds.shows_status_on_front_when_lifted());
    assert_eq!(
        leds.headlights().primary_color(),
        FloatOutBoyLedColor::WhiteFull
    );
    assert_eq!(leds.taillights().primary_color(), FloatOutBoyLedColor::Red);
    assert_eq!(
        leds.front().animation_mode(),
        FloatOutBoyLedAnimationMode::Solid
    );
    assert_eq!(
        leds.rear().animation_mode(),
        FloatOutBoyLedAnimationMode::Pulse
    );
    assert_eq!(leds.status().idle_timeout().as_seconds(), 45);
    assert_eq!(leds.status_idle().primary_color(), FloatOutBoyLedColor::Red);
}
