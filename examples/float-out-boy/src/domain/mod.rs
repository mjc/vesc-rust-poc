//! Float Out Boy-specific ride-domain types.
//!
//! These types compose the reusable `vescpkg-rs` package-author units and
//! semantic wrappers into Float Out Boy concepts. Raw firmware/app-data primitives
//! should stay at explicit boundary conversions.
//!
//! Source anchors for the compatibility surface below are Float Out Boy `v1.2.1`
//! (`0ef6e99d8701`):
//! - `third_party/float-out-boy/src/main.c:1313-1399` defines `COMMAND_GET_ALLDATA` response layout.
//! - `third_party/float-out-boy/src/main.c:1876-1901` defines realtime-data ID-list packet layout.
//! - `third_party/float-out-boy/src/main.c:1190-1205` defines startup `Data` initialization order.

mod all_data;
mod app_data;
mod motor_command;
mod realtime;
mod ride_state;
mod state;
mod wire;

pub use self::all_data::{
    FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMode2Payload,
    FloatOutBoyAllDataMode3Payload, FloatOutBoyAllDataMode4Payload, FloatOutBoyAllDataMotorPayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataResponse, FloatOutBoyAllDataStatus,
};
pub use self::app_data::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAllDataRequest,
    FloatOutBoyAllDataRequestError, FloatOutBoyAppDataCommand, FloatOutBoyAppDataCommandError,
    FloatOutBoyAppDataPackageId,
};
pub use self::motor_command::FloatOutBoyMotorCommand;
pub use self::realtime::{
    FLOAT_OUT_BOY_REALTIME_DATA_ITEMS, FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyRealtimeAtrAccelerationDiff,
    FloatOutBoyRealtimeAtrSpeedBoost, FloatOutBoyRealtimeBalanceCurrent,
    FloatOutBoyRealtimeBalancePitch, FloatOutBoyRealtimeBoosterCurrent,
    FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeFilteredMotorCurrent, FloatOutBoyRealtimeMotorCurrents,
    FloatOutBoyRealtimeMotorTemperatures, FloatOutBoyRealtimeRemoteInput,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyRealtimeTail,
};
pub use self::ride_state::FloatOutBoyRideState;
pub use self::state::{
    FloatOutBoyBeepReason, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState, FloatOutBoyMode, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition, FloatOutBoyTractionControlState,
    FloatOutBoyWheelSlipState,
};
pub use crate::footpad::{FloatOutBoyFootpadSample, FloatOutBoyFootpadState};
