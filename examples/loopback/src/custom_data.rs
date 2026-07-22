//! Usage-shaped port of VESC's `custom_data_comm` wire state.

use vesc_protocol::buffer::{append_float32_auto, append_i32, read_float32_auto};

/// State exchanged by the official custom application-data example.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomDataCommState {
    message_count: i32,
    last_value: f32,
}

impl CustomDataCommState {
    /// Construct an empty custom-data state.
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
        let Some(value) = read_float32_auto(payload, &mut index) else {
            return false;
        };
        self.last_value = value;
        self.message_count = self.message_count.wrapping_add(1);
        true
    }

    /// Encode the counter and last received value into a caller-owned buffer.
    pub fn encode_response(&self, output: &mut [u8]) -> Option<usize> {
        let mut index = 0;
        append_i32(output, &mut index, self.message_count)?;
        append_float32_auto(output, &mut index, self.last_value)?;
        Some(index)
    }

    /// Return the number of accepted messages.
    pub const fn message_count(self) -> i32 {
        self.message_count
    }

    /// Return the last accepted value.
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
        assert_eq!(state.last_value(), 1.25);

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
