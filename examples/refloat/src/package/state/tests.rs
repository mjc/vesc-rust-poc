use super::super::handle_refloat_app_data_packet;
use super::super::test_support::{
    RecordingAppDataBindings, sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
};
use super::super::{custom_config, imu_callback};
use super::{RefloatBalanceFilter, RefloatPackageLifecycle, RefloatPackageState};
use crate::domain::{
    FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude,
    RefloatAllDataBasePayload, RefloatAllDataMotorPayload, RefloatAllDataPayloads,
    RefloatAllDataStatus, RefloatAppDataCommand, RefloatChargingState, RefloatDarkRideState,
    RefloatFocIdCurrent, RefloatMode, RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch,
    RefloatRealtimeBoosterCurrent, RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints,
    RefloatRideState, RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition,
    RefloatWheelSlipState,
};
use core::ffi::c_void;
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::{
    FakeImuBindings, FakeMotorControlBindings, FakeMotorTelemetryBindings,
};
use vescpkg_rs::{AppDataBindings, ffi};

fn tick_refloat_state_and_handle_packet<B, M, I>(
    state: &mut RefloatPackageState,
    lifecycle: &RefloatPackageLifecycle<B>,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
    bytes: &[u8],
) -> bool
where
    B: AppDataBindings,
    M: MotorTelemetryBindings,
    I: ImuBindings,
{
    state.refresh_runtime_state(telemetry, imu, lifecycle.bindings().system_time_ticks());
    state.handle_packet_with_runtime(lifecycle, telemetry, imu, bytes)
}

fn balance_filter_with_pitch(pitch_radians: f32) -> RefloatBalanceFilter {
    // Refloat reads pitch from quaternion with
    // `balance_filter_get_pitch` at `third_party/refloat/src/balance_filter.c:145-154`.
    RefloatBalanceFilter::from_quaternions([
        libm::cosf(pitch_radians * 0.5),
        0.0,
        libm::sinf(pitch_radians * 0.5),
        0.0,
    ])
}

#[test]
fn app_data_callback_dispatches_without_main_loop_refresh_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));

    assert!(state.handle_packet_with_runtime(
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Upstream `on_command_received` only dispatches app commands at
    // `third_party/refloat/src/main.c:2143-2225`; READY engage and
    // IMU/motor refresh stay in `refloat_thd` at `third_party/refloat/src/main.c:772-1080`.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Ready
    );
}

#[test]
fn default_config_decodes_pid_scales_like_refloat_settings() {
    let state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));

    // Refloat generated settings serialize `kp` with scale 10 at
    // `third_party/refloat/src/conf/settings.xml:28-54`, `kp2` with scale
    // 100 at `third_party/refloat/src/conf/settings.xml:55-84`, and
    // `kp2_brake` with scale 100 at
    // `third_party/refloat/src/conf/settings.xml:199-222`.
    assert_eq!(
        state.config_scaled_field(super::REFLOAT_CONFIG_KP_FIELD),
        20.0
    );
    assert_eq!(
        state.config_scaled_field(super::REFLOAT_CONFIG_KP2_FIELD),
        0.6
    );
    assert_eq!(
        state.config_scaled_field(super::REFLOAT_CONFIG_KP2_BRAKE_FIELD),
        1.0
    );
}

#[test]
fn lifecycle_installs_app_data_handler_and_stop_cleanup() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

    assert_eq!(lifecycle.install(&mut info, handler), Ok(()));
    assert!(info.stop_fun.is_some());
    assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(
        lifecycle.bindings().last_handler.get(),
        handler as *const () as usize
    );

    assert_eq!(lifecycle.stop(), Ok(()));
    assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
    assert_eq!(lifecycle.bindings().last_handler.get(), 0);
    // Refloat v1.2.1 stop clears IMU/app-data/custom config callbacks at
    // `third_party/refloat/src/main.c:2401-2403`.
    assert_eq!(lifecycle.bindings().imu_read_callback_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_imu_read_callback.get(), 0);
    assert_eq!(lifecycle.bindings().custom_config_clear_calls.get(), 1);
}

#[test]
fn lifecycle_sends_refloat_app_data_responses_through_bindings() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());

    assert!(lifecycle.send_response(
        &sample_all_data_payloads(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            4,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
    assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 4]);

    assert!(lifecycle.send_response(
        &sample_all_data_payloads(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::Info.id(),
            2,
            0,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 2);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 60);
    assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 0, 2]);

    assert!(!lifecycle.send_response(
        &sample_all_data_payloads(),
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::PrintInfo.id(),
            4,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 2);
}

#[test]
fn app_data_state_handles_packets_through_lifecycle_send_boundary() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet(
        &lifecycle,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            4,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
    assert_eq!(state.all_data_payloads(), sample_all_data_payloads());
}

#[test]
fn app_data_state_refreshes_mode2_distance_from_motor_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::new()
            .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            2,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
    assert_eq!(
        lifecycle.bindings().last_sent_mode2_distance_bits.get(),
        12.5_f32.to_bits()
    );
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 1);
}

#[test]
fn app_data_state_refreshes_mode2_temperatures_from_motor_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_temperatures(
        MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
        MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
    ));
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            2,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 41);
    assert_eq!(
        lifecycle.bindings().last_sent_mode2_temperature_bytes.get(),
        [74, 97]
    );
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
    assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
}

#[test]
fn app_data_state_refreshes_mode3_ride_totals_from_motor_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_ride_totals(
        OdometerMeters::from_meters(123_456),
        AmpHoursDischarged::new(Charge::from_amp_hours(3.2)),
        AmpHoursCharged::new(Charge::from_amp_hours(0.8)),
        WattHoursDischarged::new(Energy::from_watt_hours(170.0)),
        WattHoursCharged::new(Energy::from_watt_hours(18.5)),
        BatteryLevel::new(Ratio::from_ratio_const(0.72)),
    ));
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            3,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 54);
    assert_eq!(
        lifecycle.bindings().last_sent_mode3_ride_total_bytes.get(),
        [0, 1, 226, 64, 0, 32, 0, 8, 0, 170, 0, 18, 144]
    );
    assert_eq!(telemetry.bindings().odometer_calls.get(), 1);
    assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 1);
    assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 1);
    assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 1);
    assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 1);
    assert_eq!(telemetry.bindings().battery_level_calls.get(), 1);
}

#[test]
fn app_data_state_sends_fault_response_before_refreshing_mode_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::new()
            .with_firmware_fault(FirmwareFaultCode::from_compat_code(5)),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            4,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 4);
    assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 69]);
    assert_eq!(telemetry.bindings().firmware_fault_calls.get(), 1);
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
    assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
}

#[test]
fn app_data_handtest_command_toggles_ready_mode_like_refloat_qml() {
    // QML sends COMMAND_HANDTEST at `refloat/ui.qml.in:764-768`; C toggles
    // mode and temporary safety config at `third_party/refloat/src/main.c:1421-1450`.
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let original_config = *state.serialized_config();

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::HandTest.id(),
            1,
        ],
    ));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        RefloatMode::HandTest
    );
    assert_eq!(state.config_be_u16(super::REFLOAT_CONFIG_KI_OFFSET), 0);
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_KP_BRAKE_OFFSET),
        100
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_KP2_BRAKE_OFFSET),
        100
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET),
        10_000
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET),
        10_000
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET),
        0
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET),
        50
    );
    assert_eq!(
        state.config_be_u16(super::REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET),
        50
    );

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::HandTest.id(),
            0,
        ],
    ));
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .mode(),
        RefloatMode::Normal
    );
    assert_eq!(state.serialized_config(), &original_config);
}

#[test]
fn app_data_state_updates_mode4_charging_fields_from_charging_state_command() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::ChargingState.id(),
            151,
            1,
            1,
            244,
            0,
            123,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 0);

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            4,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 58);
    assert_eq!(
        lifecycle.bindings().last_sent_mode4_charging_bytes.get(),
        [0, 123, 1, 244]
    );
}

#[test]
fn app_data_state_does_not_refresh_distance_for_base_all_data() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::new()
            .with_distance_abs(TripDistance::new(Distance::from_meters(12.5))),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
}

#[test]
fn app_data_state_refreshes_base_battery_voltage_from_motor_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::new()
            .with_input_voltage_filtered(InputVoltage::new(Voltage::from_volts(84.2))),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 34);
    assert_eq!(
        lifecycle
            .bindings()
            .last_sent_base_motor_voltage_bytes
            .get(),
        [3, 74]
    );
    assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 0);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 0);
    assert_eq!(telemetry.bindings().odometer_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 0);
    assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 0);
    assert_eq!(telemetry.bindings().battery_level_calls.get(), 0);
}

#[test]
fn app_data_state_refreshes_realtime_voltage_and_temperatures_from_motor_telemetry() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::with_input_voltage_and_temperatures(
            InputVoltage::new(Voltage::from_volts(84.2)),
            MosfetTemperature::new(Temperature::from_degrees_celsius(37.0)),
            MotorTemperature::new(Temperature::from_degrees_celsius(48.5)),
        ),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    // Refloat writes realtime values as float16 at `third_party/refloat/src/main.c:1943-1954`
    // using `buffer_append_float16_auto` from `third_party/refloat/src/conf/buffer.c:143-145`.
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_len.get(), 53);
    assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 31, 4]);
    assert_eq!(
        lifecycle.bindings().last_sent_realtime_voltage_bytes.get(),
        [85, 67]
    );
    assert_eq!(
        lifecycle
            .bindings()
            .last_sent_realtime_temperature_bytes
            .get(),
        [80, 160, 82, 16]
    );
    assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 0);
}

#[test]
fn app_data_state_refreshes_realtime_timestamp_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(0x0102_0304),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Refloat v1.2.1 writes `d->time.now` into realtime packets at
    // `third_party/refloat/src/main.c:1931`; VESC system ticks are 100 us ticks.
    assert_eq!(
        lifecycle
            .bindings()
            .last_sent_realtime_timestamp_bytes
            .get(),
        [1, 2, 3, 4]
    );
}

#[test]
fn app_data_runtime_refreshes_startup_ready_gate_and_imu_attitude_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.25)),
                ImuPitch::new(AngleRadians::from_radians(-0.125)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let payloads = state.all_data_payloads();
    assert_eq!(
        payloads.base().status().ride_state().run_state(),
        RefloatRunState::Ready
    );
    assert_eq!(payloads.base().attitude().roll().angle().as_radians(), 0.25);
    assert_eq!(
        payloads.base().attitude().pitch().angle().as_radians(),
        -0.125
    );
}

#[test]
fn app_data_startup_ready_resets_runtime_vars_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.25)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Startup,
        RefloatMode::Normal,
    ));
    state.balance_filter = balance_filter_with_pitch(1.2);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    assert_eq!(
        base.status().ride_state().run_state(),
        RefloatRunState::Ready
    );
    // Refloat calls `reset_runtime_vars(d)` before READY at
    // `third_party/refloat/src/main.c:833-837`; reset clears
    // `balance_current` at `third_party/refloat/src/main.c:246`, resets
    // module setpoints at `third_party/refloat/src/main.c:239-244`, then
    // seeds only the board setpoint from balance pitch at
    // `third_party/refloat/src/main.c:249-252`.
    assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
    assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
    let expected_startup_setpoint = 1.2 * 180.0 / core::f32::consts::PI;
    assert!(
        (base.setpoints().board().angle().as_degrees() - expected_startup_setpoint).abs() < 0.0001
    );
    [
        base.setpoints().atr(),
        base.setpoints().brake_tilt(),
        base.setpoints().torque_tilt(),
        base.setpoints().turn_tilt(),
        base.setpoints().remote(),
    ]
    .into_iter()
    .for_each(|setpoint| assert_eq!(setpoint.angle().as_degrees(), 0.0));
}

#[test]
fn app_data_ready_uses_configured_startup_tolerances_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET
        ..super::REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET + 2]
        .copy_from_slice(&400u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET
        ..super::REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET + 2]
        .copy_from_slice(&4500u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Upstream READY engages only inside configured startup pitch/roll
    // tolerances at `third_party/refloat/src/main.c:1033-1036`; default pitch tolerance is 4
    // degrees, not the broad 45 degree fallback used by earlier Rust code.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Ready
    );
}

#[test]
fn app_data_ready_pushstart_uses_wide_pitch_gate_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY push-start engages above 1000 ERPM when `can_engage`
    // passes and pitch/roll are within 45 degrees at `third_party/refloat/src/main.c:1056-1067`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_pushstart_reverse_stop_blocks_negative_erpm_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-1200.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(20.0_f32.to_radians())),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(20.0_f32.to_radians());
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET] = 1;
    config[super::REFLOAT_CONFIG_FAULT_REVERSESTOP_ENABLED_OFFSET] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream ignores backwards push-start when reverse stop is enabled
    // at `third_party/refloat/src/main.c:1061-1064`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
}

#[test]
fn app_data_running_flywheel_both_footpads_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Flywheel,
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops RUNNING FLYWHEEL when both footpads
    // are engaged at `third_party/refloat/src/main.c:491-493`; `state_stop` moves to READY
    // and stores the stop condition at `third_party/refloat/src/state.c:29-33`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_flywheel_stop_clears_wheelslip_like_refloat_state_stop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Flywheel);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(RefloatWheelSlipState::Detected);
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `state_stop` clears wheelslip at `third_party/refloat/src/state.c:29-33`.
    assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::None);
}

#[test]
fn app_data_running_reverse_stop_no_footpads_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::ReverseStop,
        RefloatStopCondition::None,
    );
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` immediately stops reverse-stop mode when
    // the footpad is fully open at `third_party/refloat/src/main.c:418-422`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchFull
    );
}

#[test]
fn app_data_running_quickstop_no_footpads_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(15.0_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let motor = RefloatAllDataMotorPayload::new(
        base.motor().battery_voltage(),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
        base.motor().vehicle_speed(),
        base.motor().motor_current(),
        base.motor().battery_current(),
        base.motor().duty_cycle(),
        base.motor().foc_id_current(),
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        no_footpads,
        setpoints,
        base.booster_current(),
        motor,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET] = 1;
    state.store_serialized_config(&config);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` quick-stops no-footpad low-speed
    // pitch-runaway cases at `third_party/refloat/src/main.c:419-423`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::QuickStop);
}

#[test]
fn app_data_running_full_switch_stopped_after_delay_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops a fully open switch after
    // `fault_delay_switch_full` at `third_party/refloat/src/main.c:397-404`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchFull
    );
}

#[test]
fn app_data_running_half_switch_stopped_after_delay_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let single_footpad = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::Left,
    );
    let motor = RefloatAllDataMotorPayload::new(
        base.motor().battery_voltage(),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
        base.motor().vehicle_speed(),
        base.motor().motor_current(),
        base.motor().battery_current(),
        base.motor().duty_cycle(),
        base.motor().foc_id_current(),
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        motor,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops a partially open switch below
    // `fault_adc_half_erpm` after `fault_delay_switch_half` at
    // `third_party/refloat/src/main.c:459-467`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_reverse_stop_high_pitch_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(19.0_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::ReverseStop,
        RefloatStopCondition::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` immediately stops reverse-stop mode when
    // `fabsf(d->imu.pitch) > 18` at `third_party/refloat/src/main.c:423-426`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_reverse_stop_pitch_timer_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(11_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(11.0_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::ReverseStop,
        RefloatStopCondition::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops reverse-stop mode when pitch stays
    // above 10 degrees for 1 second at `third_party/refloat/src/main.c:440-443`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_reverse_stop_total_erpm_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(201_000.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::Normal,
        RefloatSetpointAdjustment::ReverseStop,
        RefloatStopCondition::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    for _ in 0..2 {
        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
    }

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream accumulates reverse-stop ERPM at `third_party/refloat/src/main.c:522-525`, then
    // stops once it exceeds `reverse_tolerance * 10` at `third_party/refloat/src/main.c:450-452`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_darkride_footpads_stop_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream darkride `check_faults(d)` allows turning it off by
    // engaging foot sensors at `third_party/refloat/src/main.c:387-390`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_simple_start_single_footpad_stops_during_engage_grace() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(5_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let single_footpad = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::Left,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            single_footpad,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *include_bytes!("../../conf/default_config.dat");
    config[super::REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream simple-start `can_engage(d)` accepts one sensor during the
    // first second after engage at `third_party/refloat/src/main.c:338-344`; darkride
    // `check_faults(d)` then stops at `third_party/refloat/src/main.c:387-390`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::SwitchHalf
    );
}

#[test]
fn app_data_running_darkride_high_erpm_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2100.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream darkride `check_faults(d)` immediately reverse-stops above
    // 2000 ERPM at `third_party/refloat/src/main.c:363-373`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.stop_condition(),
        RefloatStopCondition::ReverseStop
    );
}

#[test]
fn app_data_running_roll_stopped_after_delay_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(70.0_f32.to_radians())),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let mut state = RefloatPackageState::new(payloads);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops roll above `fault_roll` after
    // `fault_delay_roll` at `third_party/refloat/src/main.c:474-482`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
}

#[test]
fn app_data_running_pitch_stopped_after_delay_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(3_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(70.0_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let mut state = RefloatPackageState::new(payloads);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `check_faults(d)` stops pitch above `fault_pitch` after
    // `fault_delay_pitch` when remote setpoint is below 30 degrees at
    // `third_party/refloat/src/main.c:497-503`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Pitch);
}

#[test]
fn app_data_running_darkride_enabled_high_roll_stops_like_refloat_fault_check() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(110.0_f32.to_radians())),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            base.status(),
            no_footpads,
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        ),
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_FAULT_DARKRIDE_ENABLED_OFFSET] = 1;
    state.store_serialized_config(&config);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream non-darkride `check_faults(d)` stops immediately when
    // darkride faults are enabled and roll is 100-135 degrees at
    // `third_party/refloat/src/main.c:465-470`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::Roll);
}

#[test]
fn app_data_ready_darkride_first_second_engages_without_roll_gate_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(5_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(170.0_f32.to_radians())),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY darkride ignores roll during the first second after
    // disengage at `third_party/refloat/src/main.c:1038-1054`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_normal_both_footpads_engages_like_refloat_start_conditions() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(0.05);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY engages when startup pitch/roll tolerances and
    // `can_engage(d)` pass at `third_party/refloat/src/main.c:1033-1036`; `state_engage`
    // moves to RUNNING and sets SAT_CENTERING at `third_party/refloat/src/state.c:36-39`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert_eq!(ride_state.stop_condition(), RefloatStopCondition::None);
}

#[test]
fn app_data_ready_engage_resets_runtime_vars_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(0.05);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    let ride_state = base.status().ride_state();
    // Upstream `engage(d)` calls `reset_runtime_vars(d)` before
    // `state_engage(d)` at `third_party/refloat/src/main.c:263-270`, then
    // breaks out of the READY branch without running the RUNNING
    // balance-current loop.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(base.balance_current().current().current().as_amps(), 0.0);
    assert_eq!(base.booster_current().current().current().as_amps(), 0.0);
    let expected_engage_setpoint = 0.05 * 180.0 / core::f32::consts::PI;
    assert_eq!(
        base.setpoints().board().angle().as_degrees(),
        expected_engage_setpoint
    );
    assert_eq!(base.setpoints().remote().angle().as_degrees(), 0.0);
    assert!(!state.apply_requested_motor_current(&motor));
}

#[test]
fn app_data_ready_normal_charging_does_not_engage_like_refloat_can_engage() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let charging_state = base
        .status()
        .ride_state()
        .with_charging(RefloatChargingState::Charging);
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(charging_state, base.status().beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` rejects charging state before checking
    // footpads at `third_party/refloat/src/main.c:328-331`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.charging(), RefloatChargingState::Charging);
}

#[test]
fn app_data_ready_remote_throttle_requests_idle_current_like_refloat_do_rc_move() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(1),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET
        ..super::REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET + 2]
        .copy_from_slice(&100i16.to_be_bytes());
    config[super::REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET
        ..super::REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET + 2]
        .copy_from_slice(&0i16.to_be_bytes());
    assert!(state.store_serialized_config(&config));
    state.set_remote_input_for_test(0.5);

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `do_rc_move(d)` uses default inverted throttle and filters
    // `rc_current = old * 0.95 + target * 0.05` before requesting current
    // at `third_party/refloat/src/main.c:291-298`; 10A max with 50% input requests -0.25A.
    assert_eq!(motor.bindings().current().current().as_amps(), -0.25);
}

#[test]
fn app_data_rc_move_command_steps_idle_current_like_refloat_do_rc_move() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RcMove.id(),
            1,
            40,
            2,
            42,
        ],
    ));
    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `cmd_rc_move` sets `rc_steps = time * 100` and target
    // current/10 at `third_party/refloat/src/main.c:1747-1756`; `do_rc_move` filters the first
    // READY tick by 5% at `third_party/refloat/src/main.c:276-286`.
    assert!((motor.bindings().current().current().as_amps() - 0.2).abs() < 0.0001);
}

#[test]
fn app_data_rc_move_halves_large_target_after_500_steps_like_refloat_do_rc_move() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        base.status(),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(state.handle_packet_with_telemetry(
        &lifecycle,
        &telemetry,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RcMove.id(),
            1,
            60,
            6,
            66,
        ],
    ));
    for _ in 0..500 {
        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
    }

    // Upstream `do_rc_move(d)` halves targets above 2A when `rc_counter`
    // reaches 500 at `third_party/refloat/src/main.c:281-284`, after decrementing steps.
    assert_eq!(state.rc_current_target_deciamps, 30);
    assert_eq!(state.rc_steps, 100);
}

#[test]
fn app_data_ready_flywheel_without_footpads_engages_like_refloat_can_engage() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Flywheel);
    let base = payloads.base();
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        no_footpads,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` keeps FLYWHEEL mode engaged after footpad
    // checks at `third_party/refloat/src/main.c:346-349`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_flywheel_both_footpads_stops_flywheel_like_refloat_ready_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Flywheel,
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream READY handles FLYWHEEL abort/both-footpad before start
    // conditions at `third_party/refloat/src/main.c:957-963`; `flywheel_stop` returns to
    // NORMAL mode at `third_party/refloat/src/main.c:1869-1873`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(ride_state.mode(), RefloatMode::Normal);
}

#[test]
fn app_data_ready_single_footpad_engages_when_dual_switch_config_is_set() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let single_footpad = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::Left,
    );
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *include_bytes!("../../conf/default_config.dat");
    config[super::REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` allows a single footpad when
    // `fault_is_dual_switch` is enabled at `third_party/refloat/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_ready_single_footpad_default_config_does_not_engage_like_refloat_can_engage() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let single_footpad = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::Left,
    );
    let upright_base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        upright_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` keeps a single footpad gated unless
    // `fault_is_dual_switch` or simple start is enabled at `third_party/refloat/src/main.c:338-342`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Ready);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::None
    );
}

#[test]
fn app_data_ready_simple_start_single_footpad_engages_after_disengage_grace_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(
        RecordingAppDataBindings::accepting().with_system_time_ticks(20_000),
    );
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.1)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let single_footpad = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.8)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::Left,
    );
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.05)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        single_footpad,
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *include_bytes!("../../conf/default_config.dat");
    config[super::REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream `can_engage(d)` allows simple-start single-sensor starts
    // two seconds after disengage at `third_party/refloat/src/main.c:338-344`.
    assert_eq!(ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(
        ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_runtime_applies_disabled_config_before_startup_ready_like_refloat() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = vescpkg_rs::ImuApi::new(
        vescpkg_rs::test_support::FakeImuBindings::new().with_startup_done(true),
    );
    let mut incoming = *include_bytes!("../../conf/default_config.dat");
    incoming[super::REFLOAT_CONFIG_DISABLED_OFFSET] = 1;
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(custom_config::refloat_set_cfg_with_state(
        incoming.as_mut_ptr(),
        Some(&mut state),
    ));
    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Upstream `configure(d)` applies `disabled` before the control-loop
    // startup gate at `third_party/refloat/src/main.c:184-190`; `state_set_disabled` forces
    // `STATE_DISABLED` at `third_party/refloat/src/state.c:41-47`, so `third_party/refloat/src/main.c:833-838`
    // cannot promote STARTUP to READY in this configuration.
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        RefloatRunState::Disabled,
    );
}

#[test]
fn app_data_configured_loop_time_uses_refloat_hertz_config() {
    let mut incoming = *include_bytes!("../../conf/default_config.dat");
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert_eq!(state.configured_loop_time_us(), 1201);

    incoming[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
        .copy_from_slice(&500u16.to_be_bytes());
    assert!(custom_config::refloat_set_cfg_with_state(
        incoming.as_mut_ptr(),
        Some(&mut state),
    ));

    // Upstream generated serialization places `hertz` after the first
    // seven float16 config fields; `configure(d)` then uses it as
    // `1e6 / d->float_conf.hertz` at `third_party/refloat/src/main.c:190-191`.
    assert_eq!(state.configured_loop_time_us(), 2000);
}

#[test]
fn app_data_footpad_runtime_refresh_decodes_adc_like_refloat_sensor_update() {
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    state.refresh_footpad_runtime_state(2.5, -1.0);

    let footpad = state.all_data_payloads().base().footpad();
    // C map: Refloat v1.2.1 `footpad_sensor_update` reads ADCs, clamps
    // missing ADC2 to zero, and decodes the switch state at
    // `third_party/refloat/src/footpad_sensor.c:28-61`.
    assert_eq!(footpad.state(), FootpadSensorState::Left);
    assert_eq!(footpad.adc1_volts(), 2.5);
    assert_eq!(footpad.adc2_volts(), 0.0);
}

#[test]
fn app_data_motor_control_applies_requested_current_like_refloat_motor_control() {
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    state.request_motor_current(MotorCurrent::new(Current::from_amps(6.25)));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `motor_control_apply` resets timeout, keeps current control
    // on for 50ms, sends the requested current, then clears the request at
    // `third_party/refloat/src/motor_control.c:92-99` and `third_party/refloat/src/motor_control.c:121-122`.
    assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
    assert_eq!(motor.bindings().set_current_off_delay_calls.get(), 1);
    assert_eq!(motor.bindings().current_off_delay_seconds(), 0.05);
    assert_eq!(motor.bindings().set_current_calls.get(), 1);
    assert_eq!(motor.bindings().current().current().as_amps(), 6.25);
    assert!(!state.apply_requested_motor_current(&motor));
    assert_eq!(motor.bindings().set_current_calls.get(), 1);
}

#[test]
fn app_data_running_runtime_requests_balance_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.0_f32.to_radians())),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(1.0_f32.to_radians());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream RUNNING computes `d->balance_current` and then requests it
    // via `motor_control_request_current` at `third_party/refloat/src/main.c:949-956`.
    assert_eq!(motor.bindings().current().current().as_amps(), 3.8);
}

#[test]
fn app_data_running_motor_apply_uses_current_branch_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(4.75))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(1.0_f32.to_radians())),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(1.0_f32.to_radians());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_motor_control(&motor, RefloatRunState::Running, 1));

    // Upstream RUNNING computes and requests balance current at
    // `third_party/refloat/src/main.c:918-956`, then `refloat_thd` calls
    // `motor_control_apply` at `third_party/refloat/src/main.c:1076`; a
    // current request takes the `mc_set_current` branch at
    // `third_party/refloat/src/motor_control.c:92-121`.
    assert_eq!(motor.bindings().set_current_calls.get(), 1);
    assert_eq!(motor.bindings().set_brake_current_calls.get(), 0);
    assert_eq!(motor.bindings().set_duty_calls.get(), 0);
    assert_eq!(motor.bindings().current().current().as_amps(), 3.8);
}

#[test]
fn app_data_handtest_running_recenters_start_setpoint_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::HandTest);
    let base = payloads.base();
    let ride_state = RefloatRideState::new(
        RefloatRunState::Running,
        RefloatMode::HandTest,
        RefloatSetpointAdjustment::Centering,
        RefloatStopCondition::None,
    );
    let board = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0));
    let zero = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(board, zero, zero, zero, zero, zero);
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        setpoints,
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(0.0);
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET
        ..super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET + 2]
        .copy_from_slice(&5000u16.to_be_bytes());
    state.serialized_config = config;

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    let base = state.all_data_payloads().base();
    // Refloat RUNNING `SAT_CENTERING` uses `startup_speed / hertz` from
    // `third_party/refloat/src/main.c:172` via
    // `get_setpoint_adjustment_step_size` at
    // `third_party/refloat/src/main.c:304-310`; `rate_limitf` applies that
    // step toward target zero at `third_party/refloat/src/utils.c:25-33`,
    // and the main loop publishes the new setpoint at
    // `third_party/refloat/src/main.c:869-875`.
    assert_eq!(base.setpoints().board().angle().as_degrees(), 1.5);
    assert_eq!(
        base.status().ride_state().setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
}

#[test]
fn app_data_normal_algorithm_trace_matches_refloat_loop_order() {
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(1.5_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            )
            .with_angular_rate(ImuAngularRate::new([
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(0.0),
            ])),
    );
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Ready, RefloatMode::Normal);
    let base = payloads.base();
    let stopped_base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.attitude(),
        base.status(),
        base.footpad(),
        base.setpoints(),
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataMotorPayload::new(
            BatteryVoltage::new(Voltage::from_volts(72.0)),
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
            MotorCurrent::new(Current::from_amps(0.0)),
            BatteryCurrent::new(Current::from_amps(0.0)),
            DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
            RefloatFocIdCurrent::measured(MotorCurrent::new(Current::from_amps(0.0))),
        ),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        stopped_base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.balance_filter = balance_filter_with_pitch(2.0_f32.to_radians());
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_HERTZ_OFFSET..super::REFLOAT_CONFIG_HERTZ_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET
        ..super::REFLOAT_CONFIG_STARTUP_SPEED_OFFSET + 2]
        .copy_from_slice(&5000u16.to_be_bytes());
    state.serialized_config = config;

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &RefloatPackageLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(0),
        ),
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    let engaged_base = state.all_data_payloads().base();
    let engaged_ride_state = engaged_base.status().ride_state();
    // Upstream READY/NORMAL engages through `engage(d)` at
    // `third_party/refloat/src/main.c:263-270`; `reset_runtime_vars(d)`
    // seeds only the board setpoint from balance pitch at
    // `third_party/refloat/src/main.c:239-252`, and READY breaks before
    // RUNNING PID in `third_party/refloat/src/main.c:1018-1037`.
    assert_eq!(engaged_ride_state.run_state(), RefloatRunState::Running);
    assert_eq!(engaged_ride_state.mode(), RefloatMode::Normal);
    assert_eq!(
        engaged_ride_state.setpoint_adjustment(),
        RefloatSetpointAdjustment::Centering
    );
    assert_eq!(engaged_base.setpoints().board().angle().as_degrees(), 2.0);
    assert_eq!(
        engaged_base.balance_current().current().current().as_amps(),
        0.0
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &RefloatPackageLifecycle::new(
            RecordingAppDataBindings::accepting().with_system_time_ticks(1),
        ),
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    let running_base = state.all_data_payloads().base();
    let kp = state.config_scaled_i16(super::REFLOAT_CONFIG_KP_OFFSET, 10.0);
    let ki = state.config_scaled_i16(super::REFLOAT_CONFIG_KI_OFFSET, 100_000.0);
    let ki_limit = state.config_scaled_i16(super::REFLOAT_CONFIG_KI_LIMIT_OFFSET, 10.0);
    let expected_board_setpoint = 1.5;
    let expected_setpoint_error = expected_board_setpoint - 2.0;
    let unclamped_i = expected_setpoint_error * ki;
    let expected_i = if ki_limit > 0.0 && unclamped_i.abs() > ki_limit {
        ki_limit * unclamped_i.signum()
    } else {
        unclamped_i
    };
    let current_limit = state.motor_current_max.current().as_amps();
    let expected_new_current =
        (expected_setpoint_error * kp + expected_i).clamp(-current_limit, current_limit);
    let expected_smoothed_current = expected_new_current * 0.2;
    // Upstream RUNNING centers with `startup_speed / hertz` at
    // `third_party/refloat/src/main.c:172`,
    // `third_party/refloat/src/main.c:304-310`, and
    // `third_party/refloat/src/main.c:869-875`;
    // then NORMAL PID and the regular motor-current limit run at
    // `third_party/refloat/src/main.c:918-956` before requesting motor
    // current. Raw pitch equals the centered board setpoint here, so the
    // booster proportional is zero by `third_party/refloat/src/main.c:921-922`.
    assert_eq!(
        running_base.setpoints().board().angle().as_degrees(),
        expected_board_setpoint
    );
    assert_eq!(
        running_base.booster_current().current().current().as_amps(),
        0.0
    );
    assert!(
        (running_base.balance_current().current().current().as_amps() - expected_smoothed_current)
            .abs()
            < 0.0001
    );

    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    assert!(state.apply_motor_control(&motor, running_base.status().ride_state().run_state(), 1,));
    // Upstream main loop calls `motor_control_apply` after the balance loop
    // at `third_party/refloat/src/main.c:1075-1079`; requested current
    // takes the current-control branch at
    // `third_party/refloat/src/motor_control.c:92-99`.
    assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
    assert_eq!(motor.bindings().set_current_off_delay_calls.get(), 1);
    assert_eq!(motor.bindings().set_current_calls.get(), 1);
    assert!(
        (motor.bindings().current().current().as_amps() - expected_smoothed_current).abs() < 0.0001
    );
    assert_eq!(motor.bindings().set_duty_calls.get(), 0);
    assert_eq!(motor.bindings().set_brake_current_calls.get(), 0);
}

#[test]
fn app_data_running_computes_angle_p_balance_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `pid_update` computes angle P at `third_party/refloat/src/pid.c:40` and scales
    // it by `kp` at `third_party/refloat/src/pid.c:69`; RUNNING then smooths balance current
    // as `old * 0.8 + new_current * 0.2` at `third_party/refloat/src/main.c:932-954`.
    assert!((motor.bindings().current().current().as_amps() - 12.001).abs() < 0.0001);
    assert!(
        (state
            .all_data_payloads()
            .base()
            .balance_current()
            .current()
            .current()
            .as_amps()
            - 12.001)
            .abs()
            < 0.0001
    );
}

#[test]
fn app_data_running_uses_balance_filter_pitch_like_refloat_pid() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_KP_OFFSET,
        100,
    ));
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_KP2_OFFSET,
        0,
    ));
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_KI_OFFSET,
        0,
    ));
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_KP_BRAKE_OFFSET,
        100,
    ));
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET,
        10_000,
    ));
    assert!(RefloatPackageState::set_config_be_u16(
        &mut config,
        super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET,
        0,
    ));
    assert!(state.store_serialized_config(&config));
    state.balance_filter = balance_filter_with_pitch(5.0_f32.to_radians());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // C refreshes `imu.balance_pitch` from `balance_filter_get_pitch` at
    // `third_party/refloat/src/imu.c:35-41` before `pid_update` computes
    // `setpoint - imu->balance_pitch` at `third_party/refloat/src/pid.c:40`.
    assert!((motor.bindings().current().current().as_amps() + 10.0).abs() < 0.0001);
    assert!(
        (state
            .all_data_payloads()
            .base()
            .attitude()
            .balance_pitch()
            .angle()
            .as_radians()
            * 180.0
            / core::f32::consts::PI
            - 5.0)
            .abs()
            < 0.0001
    );
}

#[test]
fn imu_callback_state_update_feeds_normal_balance_pitch_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Normal,
    ));

    imu_callback::refloat_imu_callback_with_state(
        &mut state,
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        0.1,
    );
    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));

    // Upstream `imu_ref_callback` updates the balance filter at
    // `third_party/refloat/src/main.c:760-765`; the main loop copies that
    // filter into `imu.balance_pitch` at `third_party/refloat/src/imu.c:35-41`
    // before RUNNING PID reads it at `third_party/refloat/src/pid.c:40`.
    assert!(
        state
            .all_data_payloads()
            .base()
            .attitude()
            .balance_pitch()
            .angle()
            .as_radians()
            > 0.0
    );
}

#[test]
fn app_data_running_computes_rate_p_balance_current_like_refloat_pid() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_angular_rate(ImuAngularRate::new([
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(10.0),
                AngularVelocity::from_degrees_per_second(0.0),
            ])),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
        .copy_from_slice(&20u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `imu_update` derives pitch rate from gyro at
    // `third_party/refloat/src/imu.c:45-53`; `kp2` uses generated config
    // scale 100 at `third_party/refloat/src/conf/settings.xml:55-84`;
    // `pid_update` computes `rate_p` at `third_party/refloat/src/pid.c:71-72`,
    // then RUNNING smooths it into `balance_current` at
    // `third_party/refloat/src/main.c:921-954`.
    assert!((motor.bindings().current().current().as_amps() + 0.4).abs() < 0.0001);
}

#[test]
fn app_data_running_softstarts_pitch_based_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_angular_rate(ImuAngularRate::new([
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(10.0),
                AngularVelocity::from_degrees_per_second(0.0),
            ])),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    state.softstart_pid_limit = 0.0;
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
        .copy_from_slice(&20u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream RUNNING soft-start limits only `rate_p + booster.current`
    // before adding Angle P/I at `third_party/refloat/src/main.c:926-930`; a zero first-tick
    // limit removes the -20A Rate-P contribution before smoothing.
    assert_eq!(motor.bindings().current().current().as_amps(), 0.0);
}

#[test]
fn app_data_running_scales_forward_braking_angle_p_like_refloat_pid() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1000.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(-2.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_BRAKE_OFFSET..super::REFLOAT_CONFIG_KP_BRAKE_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `pid_update` moves forward braking Angle-P scale toward
    // `kp_brake` by 1% per tick at `third_party/refloat/src/pid.c:56-69`; with kp_brake=0 the
    // first tick scales -40A to -39.6A before RUNNING smooths by 0.2.
    assert!(
        (motor.bindings().current().current().as_amps() + 7.92).abs() < 0.0001,
        "{:?}",
        motor.bindings().current()
    );
}

#[test]
fn app_data_running_accumulates_angle_i_balance_current_like_refloat_pid() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));
    assert!(
        (motor.bindings().current().current().as_amps() - 4.001).abs() < 0.0001,
        "{:?}",
        motor.bindings().current()
    );

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `pid_update` accumulates `pid->i += pid->p * config->ki`
    // and clamps it at `third_party/refloat/src/pid.c:40-46`; RUNNING adds P + I before
    // smoothing balance current at `third_party/refloat/src/main.c:932-954`.
    assert!(
        (motor.bindings().current().current().as_amps() - 7.2028).abs() < 0.0001,
        "{:?}",
        motor.bindings().current()
    );
}

#[test]
fn app_data_running_clamps_angle_i_at_default_ki_limit_like_refloat_pid() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10_000.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Refloat default `ki_limit` is 30A (`settings.xml:1679-1707`);
    // `pid_update` clamps the I term at `third_party/refloat/src/pid.c:40-46` before RUNNING
    // smooths it into `balance_current` at `third_party/refloat/src/main.c:932-954`.
    assert!((motor.bindings().current().current().as_amps() - 6.0).abs() < 0.0001);
}

#[test]
fn app_data_running_limits_handtest_and_flywheel_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());

    for (mode, expected_current) in [
        (RefloatMode::HandTest, 1.4_f32),
        (RefloatMode::Flywheel, 8.0_f32),
    ] {
        let payloads = sample_all_data_payloads_with_ride_state(RefloatRunState::Running, mode);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(10.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let footpad = if matches!(mode, RefloatMode::Flywheel) {
            FootpadSensorSample::new(
                AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
                FootpadSensorState::None,
            )
        } else {
            base.footpad()
        };
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            footpad,
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream RUNNING clamps `new_current` to 7A for HANDTEST and
        // 40A for FLYWHEEL at `third_party/refloat/src/main.c:932-942`, then smooths it into
        // `balance_current` at `third_party/refloat/src/main.c:949-954`.
        assert!(
            (motor.bindings().current().current().as_amps() - expected_current).abs() < 0.0001,
            "{mode:?}: {:?}",
            motor.bindings().current()
        );
    }
}

#[test]
fn app_data_running_limits_normal_current_from_motor_config_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());

    for (motor_current, expected_current) in [(1.0_f32, 0.6_f32), (-1.0_f32, -0.4_f32)] {
        let telemetry = MotorTelemetryApi::new(
            FakeMotorTelemetryBindings::new()
                .with_runtime_motor(
                    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
                    VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
                    MotorCurrent::new(Current::from_amps(motor_current)),
                    BatteryCurrent::new(Current::from_amps(0.0)),
                    DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
                )
                .with_motor_current_limits(
                    MotorCurrent::new(Current::from_amps(3.0)),
                    MotorCurrent::new(Current::from_amps(2.0)),
                ),
        );
        let payloads =
            sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
        let base = payloads.base();
        let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(
            10.0 * motor_current.signum(),
        ));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(
            setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
        );
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            base.status(),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        let mut config = *state.serialized_config();
        config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
            .copy_from_slice(&0u16.to_be_bytes());
        assert!(state.store_serialized_config(&config));

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            &lifecycle,
            &telemetry,
            &imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));
        assert!(state.apply_requested_motor_current(&motor));

        // Upstream `motor_data_update` caches `l_current_max` and
        // `fabsf(l_current_min)` at `third_party/refloat/src/motor_data.c:90-91`; RUNNING uses
        // max while accelerating and min while braking at `third_party/refloat/src/main.c:932-942`.
        assert!(
            (motor.bindings().current().current().as_amps() - expected_current).abs() < 0.0001,
            "{motor_current}: {:?}",
            motor.bindings().current()
        );
    }
}

#[test]
fn app_data_running_adds_booster_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(3.0));
    let zero_setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint,
        zero_setpoint,
        zero_setpoint,
        zero_setpoint,
        zero_setpoint,
        zero_setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET
        ..super::REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config
        [super::REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET..super::REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET
        ..super::REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET + 2]
        .copy_from_slice(&2000u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream subtracts brake tilt at `third_party/refloat/src/main.c:921`,
    // `booster_update` applies full configured booster current above
    // angle+ramp at `third_party/refloat/src/booster.c:35-46`, filters it
    // at `third_party/refloat/src/booster.c:74-75`, and RUNNING adds it to
    // rate-P before smoothing `balance_current` at
    // `third_party/refloat/src/main.c:921-954`.
    assert!(
        (state
            .all_data_payloads()
            .base()
            .booster_current()
            .current()
            .current()
            .as_amps()
            - 0.2)
            .abs()
            < 0.0001
    );
    assert!((motor.bindings().current().current().as_amps() - 0.04).abs() < 0.0001);
}

#[test]
fn app_data_running_adds_braking_booster_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(0.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(-2.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    ));
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(3.0_f32.to_radians())),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        base.status(),
        base.footpad(),
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));
    let mut config = *state.serialized_config();
    config[super::REFLOAT_CONFIG_KP_OFFSET..super::REFLOAT_CONFIG_KP_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KP2_OFFSET..super::REFLOAT_CONFIG_KP2_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_KI_OFFSET..super::REFLOAT_CONFIG_KI_OFFSET + 2]
        .copy_from_slice(&0u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET
        ..super::REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET
        ..super::REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET + 2]
        .copy_from_slice(&100u16.to_be_bytes());
    config[super::REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET
        ..super::REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET + 2]
        .copy_from_slice(&2000u16.to_be_bytes());
    assert!(state.store_serialized_config(&config));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream `motor_data_update` marks braking from negative motor
    // current at `third_party/refloat/src/motor_data.c:121-123`; `booster_update` then uses
    // `brkbooster_*` config at `third_party/refloat/src/booster.c:35-41`, applies sign from
    // proportional at `third_party/refloat/src/booster.c:60-64`, filters at `third_party/refloat/src/booster.c:68`,
    // and RUNNING smooths balance current at `third_party/refloat/src/main.c:921-954`.
    assert!(
        (state
            .all_data_payloads()
            .base()
            .booster_current()
            .current()
            .current()
            .as_amps()
            + 0.2)
            .abs()
            < 0.0001
    );
    assert!((motor.bindings().current().current().as_amps() + 0.04).abs() < 0.0001);
}

#[test]
fn app_data_running_inverts_darkride_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        no_footpads,
        setpoints,
        RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream RUNNING negates `new_current` for darkride at
    // `third_party/refloat/src/main.c:944-946`, before smoothing/requesting motor current.
    assert!((motor.bindings().current().current().as_amps() + 4.001).abs() < 0.0001);
}

#[test]
fn app_data_running_wheelslip_without_traction_control_smooths_current_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_wheelslip(RefloatWheelSlipState::Detected);
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        base.footpad(),
        setpoints,
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    // Upstream RUNNING only sets `balance_current = 0` when
    // `traction_control` is set at `third_party/refloat/src/main.c:949-954`; wheelslip alone
    // remains a UI/state flag and the current path still smooths.
    assert_ne!(motor.bindings().current().current().as_amps(), 0.0);
    assert_ne!(
        state
            .all_data_payloads()
            .base()
            .balance_current()
            .current()
            .current()
            .as_amps(),
        0.0
    );
}

#[test]
fn app_data_running_darkride_detects_traction_loss_like_refloat_loop() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(-3_000.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(0.0)),
        MotorCurrent::new(Current::from_amps(0.0)),
        BatteryCurrent::new(Current::from_amps(0.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.5)),
    ));
    let imu = ImuApi::new(
        FakeImuBindings::new()
            .with_startup_done(true)
            .with_attitude(
                ImuRoll::new(AngleRadians::from_radians(0.0)),
                ImuPitch::new(AngleRadians::from_radians(0.0)),
                ImuYaw::new(AngleRadians::from_radians(0.0)),
            ),
    );
    let motor = MotorControlApi::new(FakeMotorControlBindings::new());
    let payloads =
        sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal);
    let base = payloads.base();
    let ride_state = base
        .status()
        .ride_state()
        .with_darkride(RefloatDarkRideState::Active);
    let no_footpads = FootpadSensorSample::new(
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        AdcDecodedLevel::new(Ratio::from_ratio_const(0.0)),
        FootpadSensorState::None,
    );
    let setpoint = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(1.0));
    let setpoints = RefloatRealtimeRuntimeSetpoints::new(
        setpoint, setpoint, setpoint, setpoint, setpoint, setpoint,
    );
    let base = RefloatAllDataBasePayload::new(
        RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(10.0))),
        RefloatAllDataAttitude::new(
            RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
            base.attitude().roll(),
            base.attitude().pitch(),
        ),
        RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
        no_footpads,
        setpoints,
        base.booster_current(),
        base.motor(),
    );
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
        ],
    ));
    assert!(state.apply_requested_motor_current(&motor));

    let ride_state = state.all_data_payloads().base().status().ride_state();
    // Upstream detects traction loss from acceleration, ERPM, and duty at
    // `third_party/refloat/src/main.c:551-562`, then freewheels while traction control is set at
    // `third_party/refloat/src/main.c:949-954`.
    assert_eq!(ride_state.wheelslip(), RefloatWheelSlipState::Detected);
    assert_eq!(motor.bindings().current().current().as_amps(), 0.0);
}

#[test]
fn app_data_runtime_refreshes_motor_payload_like_refloat_motor_data_update() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
        MotorCurrent::new(Current::from_amps(12.25)),
        BatteryCurrent::new(Current::from_amps(4.0)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
    ));
    let imu = ImuApi::new(FakeImuBindings::new());
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));

    let motor = state.all_data_payloads().base().motor();
    assert_eq!(
        motor.electrical_speed().rpm().as_revolutions_per_minute(),
        1234.0
    );
    assert_eq!(motor.vehicle_speed().speed().as_meters_per_second(), 5.5);
    assert_eq!(motor.motor_current().current().as_amps(), 12.25);
    assert_eq!(motor.battery_current().current().as_amps(), 0.04);
    assert_eq!(motor.duty_cycle().ratio().as_ratio(), 0.375);
}

#[test]
fn app_data_runtime_refreshes_foc_id_current_like_refloat_all_data() {
    // Refloat v1.2.1 encodes `fabsf(VESC_IF->foc_get_id()) * 3` for
    // compact all-data at `third_party/refloat/src/main.c:1364-1368`.
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::new()
            .with_foc_id_current(Some(MotorCurrent::new(Current::from_amps(-4.0)))),
    );
    let imu = ImuApi::new(FakeImuBindings::new());
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert!(tick_refloat_state_and_handle_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        &[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
            0,
        ],
    ));

    let motor = state.all_data_payloads().base().motor();
    assert_eq!(
        motor
            .foc_id_current()
            .as_measured()
            .expect("measured Id current")
            .current()
            .as_amps(),
        -4.0
    );
    assert_eq!(lifecycle.bindings().last_sent_base_foc_id_byte.get(), 12);
}

#[test]
fn lifecycle_installs_typed_refloat_state_for_handler_retrieval() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

    assert_eq!(
        lifecycle.install_with_state(&mut info, &mut state, handler),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(
        RefloatPackageState::from_info_arg(&mut info)
            .expect("installed state")
            .all_data_payloads(),
        sample_all_data_payloads()
    );
    let mut empty_info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0,
    };
    assert!(RefloatPackageState::from_info_arg(&mut empty_info).is_none());
}

#[test]
fn lifecycle_installs_refloat_state_before_callbacks_like_refloat_startup() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

    assert!(lifecycle.install_refloat_state(&mut info, &mut state, handler));
    // Upstream sets `info->stop_fun` and `info->arg` at `third_party/refloat/src/main.c:2431-2432`,
    // before registering custom config/app-data/extensions at `third_party/refloat/src/main.c:2455-2459`.
    assert_eq!(lifecycle.bindings().handler_calls.get(), 0);
    assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 0);
    assert!(info.stop_fun.is_some());
    assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
}

#[test]
fn raw_handler_boundary_rejects_null_and_sends_valid_packets() {
    let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
    let mut state = RefloatPackageState::new(sample_all_data_payloads());

    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new());
    assert!(!handle_refloat_app_data_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        core::ptr::null_mut(),
        0,
    ));

    let mut request = [101, 10, 0];
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
    let imu = ImuApi::new(FakeImuBindings::new());
    assert!(handle_refloat_app_data_packet(
        &mut state,
        &lifecycle,
        &telemetry,
        &imu,
        request.as_mut_ptr(),
        request.len() as u32,
    ));
    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_sent_prefix.get(), [101, 10, 0]);
}
