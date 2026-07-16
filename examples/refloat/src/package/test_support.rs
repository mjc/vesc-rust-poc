use super::state::RefloatPackageState;
use crate::balance::BalanceFilter;
use crate::config::{RefloatConfigEditor, RefloatConfigImage};
use crate::domain::*;
use vescpkg_rs::prelude::*;

static REFLOAT_RUNTIME_STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) struct RefloatRuntimeStateLock {
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl Drop for RefloatRuntimeStateLock {
    fn drop(&mut self) {
        vescpkg_rs::test_support::clear_state(&super::REFLOAT_RUNTIME_STATE);
    }
}

pub(crate) fn lock_refloat_runtime_state() -> RefloatRuntimeStateLock {
    let guard = REFLOAT_RUNTIME_STATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    vescpkg_rs::test_support::clear_state(&super::REFLOAT_RUNTIME_STATE);
    RefloatRuntimeStateLock { _guard: guard }
}

pub(crate) fn sample_all_data_payloads() -> RefloatAllDataPayloads {
    sample_all_data_payloads_with_ride_state(RefloatRunState::Running, RefloatMode::Normal)
}

pub(super) fn sample_all_data_payloads_with_ride_state(
    run_state: RefloatRunState,
    mode: RefloatMode,
) -> RefloatAllDataPayloads {
    let ride_state = RefloatRideState::new(
        run_state,
        mode,
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

    RefloatAllDataPayloads::new(
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
                MotorCurrent::new(Current::from_amps(5.0)),
                BatteryCurrent::new(Current::from_amps(-2.0)),
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
    )
}

pub(super) fn balance_filter_with_pitch(pitch: AngleRadians) -> BalanceFilter {
    let pitch_radians = pitch.as_radians();
    // Refloat reads pitch from quaternion with
    // `balance_filter_get_pitch` at `third_party/refloat/src/balance_filter.c:145-154`.
    BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(cos(pitch_radians * 0.5)),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(sin(pitch_radians * 0.5)),
            ImuQuaternionZ::new(0.0),
        ),
    ))
}

pub(super) fn default_refloat_config_bytes() -> [u8; 276] {
    *include_bytes!("../conf/default_config.dat")
}

pub(super) fn editable_config_from_bytes(bytes: &[u8]) -> RefloatConfigImage {
    RefloatConfigImage::from_serialized(bytes).expect("valid Refloat config")
}

pub(super) fn editable_config_from_state(state: &RefloatPackageState) -> RefloatConfigImage {
    editable_config_from_bytes(state.serialized_config())
}

pub(super) fn store_config(state: &mut RefloatPackageState, config: &RefloatConfigImage) {
    assert!(state.store_serialized_config(config.as_bytes()));
}

pub(super) trait RefloatConfigTestBytes {
    fn edit_refloat_config(&mut self, edit: impl FnOnce(&mut RefloatConfigEditor<'_>));
}

impl RefloatConfigTestBytes for [u8; 276] {
    fn edit_refloat_config(&mut self, edit: impl FnOnce(&mut RefloatConfigEditor<'_>)) {
        let mut config = RefloatConfigImage::from_serialized(self).expect("valid Refloat config");
        edit(&mut config.editor());
        *self = *config.as_bytes();
    }
}

pub(super) fn tick_refloat_state_and_handle_packet(
    state: &mut RefloatPackageState,
    now: TimestampTicks,
    telemetry: &impl vescpkg_rs::MotorTelemetry,
    imu: &impl vescpkg_rs::Imu,
    bytes: &[u8],
) -> bool {
    state.refresh_runtime_state(telemetry, imu, now);
    let mut now = || now;
    let mut discard = |_bytes: &[u8]| true;
    state.handle_packet_with_runtime(telemetry, imu, &mut now, &mut discard, bytes)
}

pub(super) fn edit_config(
    state: &mut RefloatPackageState,
    edit: impl FnOnce(&mut RefloatConfigEditor<'_>),
) {
    let mut config = editable_config_from_state(state);
    edit(&mut config.editor());
    store_config(state, &config);
}

pub(super) fn imu_accel_x(acceleration: AccelerationG) -> ImuAccelerationX {
    ImuAccelerationX::new(acceleration)
}

pub(super) fn imu_accel_y(acceleration: AccelerationG) -> ImuAccelerationY {
    ImuAccelerationY::new(acceleration)
}

pub(super) fn imu_accel_z(acceleration: AccelerationG) -> ImuAccelerationZ {
    ImuAccelerationZ::new(acceleration)
}

pub(super) fn imu_acceleration(
    x: ImuAccelerationX,
    y: ImuAccelerationY,
    z: ImuAccelerationZ,
) -> ImuAcceleration {
    ImuAcceleration::from_axes(x, y, z)
}

pub(super) fn imu_period(period: VescSeconds) -> ImuSamplePeriod {
    ImuSamplePeriod::new(period)
}

pub(super) fn imu_read_sample(
    acceleration: ImuAcceleration,
    angular_rate: ImuAngularRate,
    period: ImuSamplePeriod,
) -> ImuReadSample {
    ImuReadSample::from_parts(acceleration, angular_rate, period)
}

pub(super) fn imu_roll_rate(rate: AngularVelocity) -> ImuAngularRateRoll {
    ImuAngularRateRoll::new(rate)
}

pub(super) fn imu_pitch_rate(rate: AngularVelocity) -> ImuAngularRatePitch {
    ImuAngularRatePitch::new(rate)
}

pub(super) fn imu_yaw_rate(rate: AngularVelocity) -> ImuAngularRateYaw {
    ImuAngularRateYaw::new(rate)
}

pub(super) fn imu_angular_rate(
    roll: ImuAngularRateRoll,
    pitch: ImuAngularRatePitch,
    yaw: ImuAngularRateYaw,
) -> ImuAngularRate {
    ImuAngularRate::from_axes(roll, pitch, yaw)
}
use vescpkg_rs::{cos, sin};
