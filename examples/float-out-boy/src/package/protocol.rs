#[cfg(test)]
mod realtime;

#[cfg(any(test, target_arch = "arm"))]
pub(in crate::package) use vesc_float_out_boy_protocol::realtime_value;
pub(super) use vesc_float_out_boy_protocol::{
    encode_float_out_boy_get_realtime_data_response_with_remote,
    encode_float_out_boy_realtime_data_response_with_runtime,
};
pub(in crate::package) use vesc_float_out_boy_protocol::{
    encode_float_out_boy_info_response, encode_float_out_boy_realtime_data_ids_response,
};
