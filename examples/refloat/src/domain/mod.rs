//! Refloat-specific ride-domain types.
//!
//! These types compose the reusable `vescpkg-rs` package-author units and
//! semantic wrappers into Refloat concepts. Raw firmware/app-data primitives
//! should stay at explicit boundary conversions.
//!
//! Source anchors for the compatibility surface below are Refloat `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/refloat/src/main.c:1313-1399` defines `COMMAND_GET_ALLDATA` response layout.
//! - `third_party/refloat/src/main.c:1876-1901` defines realtime-data ID-list packet layout.
//! - `third_party/refloat/src/main.c:1190-1205` defines startup `Data` initialization order.

mod all_data;
mod app_data;
mod motor_command;
mod realtime;
mod ride_state;
mod state;
mod wire;

pub use self::all_data::{
    RefloatAllDataAttitude, RefloatAllDataBasePayload, RefloatAllDataBatteryTemperature,
    RefloatAllDataMode2Payload, RefloatAllDataMode3Payload, RefloatAllDataMode4Payload,
    RefloatAllDataMotorPayload, RefloatAllDataPayloads, RefloatAllDataResponse,
    RefloatAllDataStatus, RefloatFocIdCurrent,
};
pub use self::app_data::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataMode, RefloatAllDataRequest,
    RefloatAllDataRequestError, RefloatAppDataCommand, RefloatAppDataCommandError,
    RefloatAppDataPackageId,
};
pub use self::motor_command::RefloatMotorCommand;
pub use self::realtime::{
    REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RECORDED_ITEMS, REFLOAT_REALTIME_RUNTIME_ITEMS,
    RefloatAlertId, RefloatRealtimeAlertMask, RefloatRealtimeAlwaysPayload,
    RefloatRealtimeAtrAccelerationDiff, RefloatRealtimeAtrSpeedBoost,
    RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
    RefloatRealtimeChargingCurrent, RefloatRealtimeChargingPayload, RefloatRealtimeChargingVoltage,
    RefloatRealtimeDataHeader, RefloatRealtimeDataItem, RefloatRealtimeDataItemGroup,
    RefloatRealtimeDataRecordPolicy, RefloatRealtimeFilteredMotorCurrent,
    RefloatRealtimeImuPayload, RefloatRealtimeMotorCurrents, RefloatRealtimeMotorPayload,
    RefloatRealtimeMotorTemperatures, RefloatRealtimeRemoteInput, RefloatRealtimeReservedFlags,
    RefloatRealtimeRuntimeAtrPayload, RefloatRealtimeRuntimePayload,
    RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints, RefloatRealtimeTail,
};
pub use self::ride_state::RefloatRideState;
pub use self::state::{
    RefloatBeepReason, RefloatChargingState, RefloatDarkRideState, RefloatDataRecorderFlags,
    RefloatFatalErrorState, RefloatMode, RefloatRunState, RefloatSetpointAdjustment,
    RefloatStopCondition, RefloatWheelSlipState,
};
pub use crate::footpad::{RefloatFootpadSample, RefloatFootpadState};
