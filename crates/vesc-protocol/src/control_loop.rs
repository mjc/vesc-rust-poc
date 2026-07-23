//! Allocation-free wire helpers for the no-actuation control-loop example.

/// Command byte that changes the signed setpoint.
pub const SETPOINT_COMMAND: u8 = 1;
/// Command byte that reads the shared control-loop state.
pub const STATUS_COMMAND: u8 = 2;
/// Encoded response size for [`ControlLoopStatus`].
pub const STATUS_BYTES: usize = 11;

/// Error returned by the bounded control-loop wire codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The command byte is not part of this example protocol.
    UnknownCommand,
    /// The command or response did not contain exactly the required bytes.
    InvalidLength,
    /// The response buffer is too short for the requested response.
    ResponseTooShort,
    /// A response did not carry the expected command byte.
    UnexpectedResponse,
}

/// Owned status returned by the control-loop example's wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLoopStatus {
    setpoint: i16,
    sampled_input: i16,
    output: i16,
    tick_count: u32,
}

impl ControlLoopStatus {
    /// Decode one status response without allocating or retaining firmware data.
    pub fn decode(response: &[u8]) -> Result<Self, CommandError> {
        if response.len() != STATUS_BYTES {
            return Err(CommandError::InvalidLength);
        }
        if response[0] != STATUS_COMMAND {
            return Err(CommandError::UnexpectedResponse);
        }
        Ok(Self {
            setpoint: i16::from_le_bytes([response[1], response[2]]),
            sampled_input: i16::from_le_bytes([response[3], response[4]]),
            output: i16::from_le_bytes([response[5], response[6]]),
            tick_count: u32::from_le_bytes([response[7], response[8], response[9], response[10]]),
        })
    }

    /// Return the requested setpoint.
    pub const fn setpoint(self) -> i16 {
        self.setpoint
    }

    /// Return the synthetic sampled input.
    pub const fn sampled_input(self) -> i16 {
        self.sampled_input
    }

    /// Return the computed, non-actuating output.
    pub const fn output(self) -> i16 {
        self.output
    }

    /// Return the number of completed loop ticks reported by firmware.
    pub const fn tick_count(self) -> u32 {
        self.tick_count
    }
}

/// Encode a setpoint command for the control-loop callback.
pub const fn encode_setpoint_command(setpoint: i16) -> [u8; 3] {
    let [low, high] = setpoint.to_le_bytes();
    [SETPOINT_COMMAND, low, high]
}

/// Encode a status request for the control-loop callback.
pub const fn encode_status_command() -> [u8; 1] {
    [STATUS_COMMAND]
}

#[cfg(test)]
mod tests {
    use super::{
        CommandError, ControlLoopStatus, SETPOINT_COMMAND, STATUS_BYTES, STATUS_COMMAND,
        encode_setpoint_command, encode_status_command,
    };

    #[test]
    fn codec_round_trips_commands_and_status() {
        assert_eq!(encode_setpoint_command(-100), [SETPOINT_COMMAND, 156, 255]);
        assert_eq!(encode_status_command(), [STATUS_COMMAND]);
        let mut response = [0_u8; STATUS_BYTES];
        response[0] = STATUS_COMMAND;
        response[1..3].copy_from_slice(&(-100_i16).to_le_bytes());
        response[3..5].copy_from_slice(&50_i16.to_le_bytes());
        response[5..7].copy_from_slice(&(-150_i16).to_le_bytes());
        response[7..11].copy_from_slice(&42_u32.to_le_bytes());
        let status = ControlLoopStatus::decode(&response).expect("status");
        assert_eq!(status.setpoint(), -100);
        assert_eq!(status.sampled_input(), 50);
        assert_eq!(status.output(), -150);
        assert_eq!(status.tick_count(), 42);
    }

    #[test]
    fn codec_rejects_invalid_responses() {
        assert_eq!(
            ControlLoopStatus::decode(&[STATUS_COMMAND]),
            Err(CommandError::InvalidLength)
        );
        let mut response = [0_u8; STATUS_BYTES];
        response[0] = SETPOINT_COMMAND;
        assert_eq!(
            ControlLoopStatus::decode(&response),
            Err(CommandError::UnexpectedResponse)
        );
    }
}
