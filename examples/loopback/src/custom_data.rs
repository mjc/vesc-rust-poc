//! Usage-shaped port of VESC's official
//! [`custom_data_comm`](https://github.com/vedderb/vesc_pkg/blob/ddf1e162d5b7d01d848263af317cc7f8f14c0d14/c_libs/examples/custom_data_comm/code.c)
//! wire state.

use vesc_protocol::buffer::{append_float32_auto, append_i32, read_float32_auto};

/// State exchanged by the official custom application-data example.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomDataCommState {
    message_count: i32,
    last_value: f32,
}

impl CustomDataCommState {
    /// Construct an empty custom-data state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message_count: 0,
            last_value: 0.0,
        }
    }

    /// Consume one received `float32_auto` payload.
    ///
    /// The state is unchanged when the payload is shorter than one encoded float.
    pub fn receive(&mut self, payload: &[u8]) -> bool {
        let mut index = 0;
        read_float32_auto(payload, &mut index)
            .map(|value| {
                self.last_value = value;
                self.message_count = self.message_count.wrapping_add(1);
            })
            .is_some()
    }

    /// Encode the counter and last received value into a caller-owned buffer.
    pub fn encode_response(&self, output: &mut [u8]) -> Option<usize> {
        let mut index = 0;
        append_i32(output, &mut index, self.message_count)?;
        append_float32_auto(output, &mut index, self.last_value)?;
        Some(index)
    }

    /// Return the number of accepted messages.
    #[must_use]
    pub const fn message_count(self) -> i32 {
        self.message_count
    }

    /// Return the last accepted value.
    #[must_use]
    pub const fn last_value(self) -> f32 {
        self.last_value
    }
}

impl Default for CustomDataCommState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::CustomDataCommState;
    use vesc_protocol::buffer::{append_float32_auto, read_float32_auto, read_i32};

    #[test]
    fn custom_data_state_decodes_and_encodes_the_official_shape() {
        let mut state = CustomDataCommState::new();
        let mut incoming = [0; 4];
        let mut incoming_index = 0;
        append_float32_auto(&mut incoming, &mut incoming_index, 1.25).expect("four bytes");

        assert!(state.receive(&incoming));
        assert_eq!(state.message_count(), 1);
        assert_eq!(state.last_value().to_bits(), 1.25_f32.to_bits());

        let mut response = [0; 8];
        assert_eq!(state.encode_response(&mut response), Some(8));
        let mut response_index = 0;
        assert_eq!(read_i32(&response, &mut response_index), Some(1));
        assert_eq!(
            read_float32_auto(&response, &mut response_index),
            Some(1.25)
        );
        assert_eq!(response_index, 8);
    }

    #[test]
    fn custom_data_state_rejects_short_payloads_without_mutation() {
        let mut state = CustomDataCommState::new();
        assert!(!state.receive(&[0, 1, 2]));
        assert_eq!(state, CustomDataCommState::new());
    }
}
