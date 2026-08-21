use super::*;
use vescpkg_rs::prelude::{FirmwareFaultWireCode, TimestampTicks};

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_get_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
) -> [u8; FLOAT_OUT_BOY_GET_REALTIME_DATA_RESPONSE_LEN] {
    encode_float_out_boy_get_realtime_data_response_with_remote(
        payloads,
        crate::domain::FloatOutBoyRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
    )
}

#[inline(never)]
pub(in crate::package) fn encode_float_out_boy_realtime_data_response(
    payloads: &FloatOutBoyAllDataPayloads,
    system_timestamp: TimestampTicks,
) -> FloatOutBoyRealtimeDataResponse {
    encode_float_out_boy_realtime_data_response_with_runtime(
        payloads,
        FloatOutBoyRealtimeDataHeader::new(
            system_timestamp,
            payloads.base().status().ride_state(),
            payloads.base().footpad().state(),
            payloads.base().status().beep_reason(),
        ),
        FloatOutBoyRealtimeTail::new(false, FirmwareFaultWireCode::from_wire_code(0)),
        crate::domain::FloatOutBoyRealtimeRemoteInput::new(
            vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
        ),
        FloatOutBoyRealtimeAtrAccelerationDiff::from_erpm_delta(0.0),
        FloatOutBoyRealtimeAtrSpeedBoost::from_units(0.0),
    )
}
