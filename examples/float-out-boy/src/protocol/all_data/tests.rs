use super::{FirmwareFaultWireCode, encode_float_out_boy_all_data_fault_response};

#[test]
fn fault_response_reports_its_wire_length() {
    let response =
        encode_float_out_boy_all_data_fault_response(FirmwareFaultWireCode::from_wire_code(5));

    assert_eq!(response.len(), 4);
}
