//! Refloat app-data packet processing.

use crate::domain::{RefloatAllDataPayloads, RefloatAllDataRequest, RefloatAllDataResponse};

/// Process one Refloat app-data packet from a typed all-data payload snapshot.
pub fn process_refloat_app_data(
    payloads: RefloatAllDataPayloads,
    bytes: &[u8],
) -> Option<RefloatAllDataResponse> {
    let request = RefloatAllDataRequest::parse(bytes).ok()?;
    Some(payloads.encode_response(request))
}

#[cfg(test)]
mod tests {
    use super::process_refloat_app_data;
    use crate::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature,
        RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
        RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataStatus,
        RefloatAppDataCommand, RefloatBeepReason, RefloatFocIdCurrent, RefloatMode,
        RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
        RefloatRealtimeChargingCurrent, RefloatRealtimeChargingVoltage,
        RefloatRealtimeMotorTemperatures, RefloatRealtimeRuntimeSetpoint,
        RefloatRealtimeRuntimeSetpoints, RefloatRideState, RefloatRunState,
        RefloatSetpointAdjustment, RefloatStopCondition,
    };
    use vescpkg_rs::prelude::*;

    #[test]
    fn app_data_processes_all_data_requests_from_payload_snapshot() {
        let response = process_refloat_app_data(
            sample_all_data_payloads(),
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::GetAllData.id(),
                4,
            ],
        )
        .expect("all-data request should produce a response");

        assert_eq!(response.as_bytes().len(), 58);
        assert_eq!(&response.as_bytes()[..3], &[101, 10, 4]);
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::GetAllData.id(),
                ]
            ),
            None
        );
        assert_eq!(
            process_refloat_app_data(
                sample_all_data_payloads(),
                &[
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                    RefloatAppDataCommand::PrintInfo.id(),
                    4,
                ]
            ),
            None
        );
    }

    fn sample_all_data_payloads() -> RefloatAllDataPayloads {
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        );
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.60)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.40)),
            FootpadSensorState::Both,
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
}
