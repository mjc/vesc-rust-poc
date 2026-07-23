use super::FloatOutBoyPackageState;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload,
    FloatOutBoyAllDataMode, FloatOutBoyAllDataMotorPayload, FloatOutBoyAllDataPayloads,
    FloatOutBoyAllDataStatus, FloatOutBoyAppDataCommand, FloatOutBoyDarkRideState,
    FloatOutBoyFootpadSample, FloatOutBoyFootpadState, FloatOutBoyMode,
    FloatOutBoyRealtimeBalanceCurrent, FloatOutBoyRealtimeBalancePitch,
    FloatOutBoyRealtimeBoosterCurrent, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyRealtimeRuntimeSetpoints, FloatOutBoyRunState, FloatOutBoyWheelSlipState,
};
use crate::package::test_support::{
    sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
};
use std::vec::Vec;
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn handle_all_data_mode(
    state: &mut FloatOutBoyPackageState,
    now: TimestampTicks,
    telemetry: &impl vescpkg_rs::MotorTelemetry,
    mode: u8,
) -> Option<Vec<u8>> {
    let mut packet = None;
    let mut now = || now;
    let mut send = |bytes: &[u8]| {
        packet = Some(Vec::from(bytes));
        true
    };

    state
        .handle_packet_with_telemetry(
            telemetry,
            &mut now,
            &mut send,
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                FloatOutBoyAppDataCommand::GetAllData.id(),
                mode,
            ],
        )
        .then_some(packet)
        .flatten()
}

#[test]
fn mode2_distance_refreshes_from_motor_telemetry() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings =
        FirmwareTest::new().with_trip_distance(TripDistance::new(Distance::from_meters(12.5)));
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let packet = handle_all_data_mode(&mut state, app_data, telemetry, 2).unwrap();
    assert_eq!(packet.len(), 41);
    assert_eq!(
        u32::from_be_bytes(packet[34..38].try_into().unwrap()),
        12.5_f32.to_bits()
    );
}

#[test]
fn mode2_temperatures_refresh_from_motor_telemetry() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings = FirmwareTest::new().with_temperatures(
        MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
        MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
    );
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let packet = handle_all_data_mode(&mut state, app_data, telemetry, 2).unwrap();
    assert_eq!(packet.len(), 41);
    assert_eq!(&packet[38..40], &[74, 97]);
}

#[test]
fn motor_temperature_limit_starts_are_typed_firmware_config() {
    let firmware = FirmwareTest::new().with_temperature_limit_starts(
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(82.0)),
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(91.0)),
    );

    assert_eq!(
        firmware.telemetry().mosfet_temperature_limit_start(),
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(82.0))
    );
    assert_eq!(
        firmware.telemetry().motor_temperature_limit_start(),
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(91.0))
    );
}

#[test]
fn mode3_ride_totals_refresh_from_motor_telemetry() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings = FirmwareTest::new().with_ride_totals(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::from_fraction(1.10),
    );
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let packet = handle_all_data_mode(&mut state, app_data, telemetry, 3).unwrap();
    assert_eq!(packet.len(), 54);
    assert_eq!(
        &packet[41..54],
        &[0, 1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 220]
    );
}

#[test]
fn fault_response_skips_mode_telemetry_refresh() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings = FirmwareTest::new()
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let packet = handle_all_data_mode(&mut state, app_data, telemetry, 4).unwrap();
    assert_eq!(packet, &[101, 10, 69, 5]);
}

#[test]
fn unknown_fault_fails_closed_without_emitting_normal_data() {
    let app_data = TimestampTicks::from_ticks(0);
    let bindings = FirmwareTest::new().with_firmware_fault(FirmwareFault::Unknown);
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    assert_eq!(
        handle_all_data_mode(&mut state, app_data, telemetry, 4),
        None
    );
}

#[test]
fn base_all_data_does_not_refresh_distance_or_temperatures() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings =
        FirmwareTest::new().with_trip_distance(TripDistance::new(Distance::from_meters(12.5)));
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    assert_eq!(
        handle_all_data_mode(
            &mut state,
            app_data,
            telemetry,
            FloatOutBoyAllDataMode::base().source_id(),
        )
        .unwrap()
        .len(),
        34
    );
}

#[test]
fn base_battery_voltage_refreshes_from_motor_telemetry() {
    let app_data = TimestampTicks::from_ticks(0);

    let bindings =
        FirmwareTest::new().with_input_voltage(InputVoltage::new(Voltage::from_volts(84.2)));
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    let packet = handle_all_data_mode(
        &mut state,
        app_data,
        telemetry,
        FloatOutBoyAllDataMode::base().source_id(),
    )
    .unwrap();
    assert_eq!(packet.len(), 34);
    assert_eq!(&packet[22..24], &[3, 74]);
}

#[test]
fn motor_runtime_tracks_typed_float_out_boy_wheelslip_duty_inputs() {
    let firmware = FirmwareTest::new()
        .with_runtime_motor(
            ElectricalSpeed::new(Rpm::ZERO),
            VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
            TotalMotorCurrent::new(Current::ZERO),
            InputCurrent::new(Current::ZERO),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.84)),
        )
        .with_duty_cycle_limit(DutyCycleLimit::new(Ratio::from_ratio_const(0.95)));
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    state.refresh_motor_runtime_state(firmware.telemetry());

    assert_eq!(state.motor_duty_raw, Ratio::from_ratio_const(0.84));
    assert_eq!(
        state.duty_max_with_margin,
        DutyCycleLimit::new(Ratio::from_ratio_const(0.90))
    );
}

#[test]
fn motor_runtime_refreshes_live_limits_before_auxiliary_side_effects() {
    let firmware = FirmwareTest::new()
        .with_motor_current_limits(
            MotorCurrentLimit::new(Current::from_amps(42.0)),
            MotorCurrentLimit::new(Current::from_amps(17.0)),
        )
        .with_temperature_limit_starts(
            TemperatureLimitStart::new(Temperature::from_degrees_celsius(82.0)),
            TemperatureLimitStart::new(Temperature::from_degrees_celsius(91.0)),
        );
    let settings = FirmwareSettings;
    settings
        .set_input_current_max(InputCurrent::new(Current::from_amps(31.0)))
        .unwrap();
    settings
        .set_input_current_min(InputCurrent::new(Current::from_amps(13.0)))
        .unwrap();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    // The source aux loop reads the live motor/config values before its LED and
    // backup side effects; exercise that complete tick order (`main.c:796-806`,
    // `main.c:1131-1155`) rather than only the refresh helper.
    state.initialize_aux_odometer(OdometerMeters::from_meters(0));
    let _ = crate::package::threads::tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        OdometerMeters::from_meters(201),
        || true,
    );

    assert_eq!(
        state.motor_current_max,
        MotorCurrentLimit::new(Current::from_amps(42.0))
    );
    assert_eq!(
        state.motor_current_min,
        MotorCurrentLimit::new(Current::from_amps(17.0))
    );
    assert_eq!(
        state.battery_current_max,
        InputCurrent::new(Current::from_amps(31.0))
    );
    assert_eq!(
        state.battery_current_min,
        InputCurrent::new(Current::from_amps(13.0))
    );
    assert_eq!(
        state.mosfet_temperature_limit_start,
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(82.0))
    );
    assert_eq!(
        state.motor_temperature_limit_start,
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(91.0))
    );
}

#[test]
fn realtime_voltage_and_temperatures_refresh_from_motor_telemetry() {
    let now = TimestampTicks::from_ticks(0);
    let bindings = FirmwareTest::new().with_input_voltage_and_temperatures(
        InputVoltage::new(Voltage::from_volts(84.2)),
        MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
        MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
    );
    let telemetry = bindings.telemetry();
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    let mut packet = Vec::new();
    let mut now = || now;
    let mut send = |bytes: &[u8]| {
        packet.extend_from_slice(bytes);
        true
    };

    assert!(state.handle_packet_with_telemetry(
        telemetry,
        &mut now,
        &mut send,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));
    // Float Out Boy writes realtime values as float16 at `third_party/float-out-boy/src/main.c:1943-1954`
    // using `buffer_append_float16_auto` from `third_party/float-out-boy/src/conf/buffer.c:143-145`.
    assert_eq!(packet.len(), 53);
    assert_eq!(&packet[..3], &[101, 31, 4]);
    assert_eq!(&packet[24..26], &[85, 67]);
    assert_eq!(&packet[28..32], &[80, 160, 82, 16]);
}

#[test]
fn darkride_traction_loss_refreshes_like_float_out_boy_loop() {
    let now = TimestampTicks::from_ticks(1_234);
    let firmware = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-3_000.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        TotalMotorCurrent::new(Current::from_amps(0.0)),
        InputCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.5)),
    );
    firmware.set_imu_ready(true);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.0)),
        ImuPitch::new(AngleRadians::from_radians(0.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let telemetry = firmware.telemetry();
    let imu = firmware.imu();
    let bindings = firmware.motor();
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let source_motor = base.motor();
    let motor = FloatOutBoyAllDataMotorPayload::new(
        source_motor.battery_voltage(),
        source_motor.electrical_speed(),
        source_motor.vehicle_speed(),
        source_motor.currents(),
        DutyCycle::new(SignedRatio::from_ratio_const(0.5)),
        source_motor.foc_id_current(),
    );
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(FloatOutBoyDarkRideState::Active);
    let no_footpads = FloatOutBoyFootpadSample::new(
        Voltage::from_volts(0.0),
        Voltage::from_volts(0.0),
        FloatOutBoyFootpadState::None,
    );
    let setpoint = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = FloatOutBoyRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        FloatOutBoyAllDataBasePayload::new(
            FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
            FloatOutBoyAllDataAttitude::new(
                FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            setpoints,
            FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            motor,
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    state.refresh_runtime_state(telemetry, imu, now);
    let expected_wheelslip_ticks = now;
    let mut now = || now;
    let mut discard = |_bytes: &[u8]| true;
    assert!(state.handle_packet_with_runtime(
        telemetry,
        imu,
        &mut now,
        &mut discard,
        &[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(bindings));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream detects traction loss from acceleration, ERPM, and duty at
    // `third_party/float-out-boy/src/main.c:551-562`, then freewheels while traction control is set at
    // `third_party/float-out-boy/src/main.c:949-954`.
    assert_eq!(ride_state.wheelslip(), FloatOutBoyWheelSlipState::Detected);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        crate::domain::FloatOutBoySetpointAdjustment::None
    );
    assert_eq!(state.wheelslip_ticks, expected_wheelslip_ticks);
    assert_eq!(firmware.commanded_current().current().as_amps(), 0.0);
}
