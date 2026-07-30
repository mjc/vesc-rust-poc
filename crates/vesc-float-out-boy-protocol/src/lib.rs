//! Float Out Boy-specific `no_std` host/device protocol.
//!
//! This crate deliberately owns only FOB wire IDs, compatibility mappings,
//! payload types, and fixed-buffer encoding. It is not a generic VESC protocol
//! and does not own balance control, LEDs, package lifecycle, or allocation.

#![no_std]
#![deny(warnings, clippy::pedantic)]
#![forbid(unused_extern_crates)]
#![cfg_attr(
    not(test),
    deny(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::mem_forget,
        clippy::missing_safety_doc,
        clippy::multiple_unsafe_ops_per_block,
        clippy::panic,
        clippy::todo,
        clippy::undocumented_unsafe_blocks,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

#[cfg(test)]
extern crate std;

mod all_data;
mod all_data_wire;
mod app_data;
mod footpad;
mod metadata;
mod packet;
mod realtime;
mod realtime_encoder;
mod ride_state;
mod state;

pub use self::all_data::{
    FloatOutBoyAllDataAttitude, FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataMode2Payload,
    FloatOutBoyAllDataMode3Payload, FloatOutBoyAllDataMode4Payload, FloatOutBoyAllDataMotorPayload,
    FloatOutBoyAllDataPayloads, FloatOutBoyAllDataResponse, FloatOutBoyAllDataStatus,
    encode_float_out_boy_all_data_fault_response,
};
pub use self::app_data::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAllDataRequest,
    FloatOutBoyAllDataRequestError, FloatOutBoyAppDataCommand,
};
pub use self::footpad::{FloatOutBoyFootpadSample, FloatOutBoyFootpadState};
pub use self::metadata::{
    FLOAT_OUT_BOY_INFO_RESPONSE_V2_LEN, FLOAT_OUT_BOY_REALTIME_DATA_IDS_RESPONSE_LEN,
    FloatOutBoyInfoResponse, encode_float_out_boy_info_response,
    encode_float_out_boy_realtime_data_ids_response,
};
#[cfg(test)]
pub use self::packet::saturating_usize_to_u8;
pub use self::packet::{
    FloatOutBoyPacket, degrees, saturating_trunc_f32_to_i16, saturating_trunc_f32_to_u8,
    saturating_trunc_f32_to_u32, truncating_u64_to_u32,
};
pub use self::realtime::{
    FLOAT_OUT_BOY_REALTIME_DATA_ITEMS, FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS,
    FLOAT_OUT_BOY_REALTIME_RUNTIME_ITEMS, FloatOutBoyRealtimeBalanceCurrent,
    FloatOutBoyRealtimeBalancePitch, FloatOutBoyRealtimeBoosterTorque,
    FloatOutBoyRealtimeDataHeader, FloatOutBoyRealtimeDataItem,
    FloatOutBoyRealtimeFilteredMotorCurrent, FloatOutBoyRealtimeMotorCurrents,
    FloatOutBoyRealtimeMotorTemperatures, FloatOutBoyRealtimeRemoteInput,
    FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
    FloatOutBoyRealtimeTail, realtime_value,
};
pub use self::realtime_encoder::{
    FloatOutBoyRealtimeDataResponse, encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_realtime_data_response_with_runtime,
};
pub use self::ride_state::FloatOutBoyRideState;
pub use self::state::{
    FloatOutBoyBeepReason, FloatOutBoyChargingState, FloatOutBoyDarkRideState,
    FloatOutBoyDataRecorderFlags, FloatOutBoyFatalErrorState, FloatOutBoyMode, FloatOutBoyRunState,
    FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition, FloatOutBoyWheelSlipState,
};
