pub(in crate::package) use vesc_float_out_boy_protocol::realtime_value;
pub(super) use vesc_float_out_boy_protocol::{
    encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_realtime_data_response_with_runtime,
    encode_float_out_boy_realtime_selected_response,
};

#[path = "realtime/tests.rs"]
mod tests;
