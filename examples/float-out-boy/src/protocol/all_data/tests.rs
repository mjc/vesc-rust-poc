use super::{
    FirmwareFaultWireCode, FloatOutBoyAllDataPayloads, FloatOutBoyRideState,
    encode_float_out_boy_all_data_fault_response,
};
use crate::FloatOutBoyRunState;

#[test]
fn fault_response_reports_its_wire_length() {
    let response =
        encode_float_out_boy_all_data_fault_response(FirmwareFaultWireCode::from_wire_code(5));

    assert_eq!(response.len(), 4);
}

#[test]
fn telemetry_snapshot_updates_typed_ride_state_directly() {
    let ride_state = FloatOutBoyRideState::default().with_run_state(FloatOutBoyRunState::Ready);
    let payloads = FloatOutBoyAllDataPayloads::source_startup().with_ride_state(ride_state);

    assert_eq!(payloads.ride_state(), ride_state);
}
