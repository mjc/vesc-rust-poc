//! Allocation-free wire helpers for the restrained FOC-audio smoke test.

/// Command byte requesting one fixed, short FOC-audio beep.
pub const BEEP_COMMAND: u8 = 0xa0;
/// Encoded response size for [`BeepResponse`].
pub const BEEP_RESPONSE_BYTES: usize = 2;

/// Device result for the fixed short-beep request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeepStatus {
    /// Firmware accepted the beep request.
    Played,
    /// The loaded firmware does not expose FOC audio.
    Unavailable,
    /// Firmware rejected the checked request.
    Rejected,
}

/// Owned response returned by the audio smoke command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeepResponse {
    status: BeepStatus,
}

impl BeepResponse {
    /// Decode one fixed-size audio smoke response.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong length, command byte, or status byte.
    pub fn decode(response: &[u8]) -> Result<Self, BeepResponseError> {
        let [command, status]: [u8; BEEP_RESPONSE_BYTES] = response
            .try_into()
            .map_err(|_| BeepResponseError::InvalidLength)?;
        if command != BEEP_COMMAND {
            return Err(BeepResponseError::UnexpectedCommand);
        }
        let status = match status {
            0 => BeepStatus::Played,
            1 => BeepStatus::Unavailable,
            2 => BeepStatus::Rejected,
            _ => return Err(BeepResponseError::UnknownStatus),
        };
        Ok(Self { status })
    }

    /// Return the reported device result.
    #[must_use]
    pub const fn status(self) -> BeepStatus {
        self.status
    }
}

/// Error returned for malformed audio smoke responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeepResponseError {
    /// The response did not contain exactly two bytes.
    InvalidLength,
    /// The response used another command byte.
    UnexpectedCommand,
    /// The response carried an unknown status byte.
    UnknownStatus,
}

/// Encode the fixed short-beep request.
#[must_use]
pub const fn encode_beep_command() -> [u8; 1] {
    [BEEP_COMMAND]
}

/// Encode one device result for the fixed short-beep request.
#[must_use]
pub const fn encode_beep_response(status: BeepStatus) -> [u8; BEEP_RESPONSE_BYTES] {
    let status = match status {
        BeepStatus::Played => 0,
        BeepStatus::Unavailable => 1,
        BeepStatus::Rejected => 2,
    };
    [BEEP_COMMAND, status]
}

#[cfg(test)]
mod tests {
    use super::{BEEP_COMMAND, BeepResponse, BeepResponseError, BeepStatus, encode_beep_command};

    #[test]
    fn codec_round_trips_the_fixed_beep_command_and_statuses() {
        assert_eq!(encode_beep_command(), [BEEP_COMMAND]);
        assert_eq!(
            BeepResponse::decode(&[BEEP_COMMAND, 0]).map(BeepResponse::status),
            Ok(BeepStatus::Played)
        );
        assert_eq!(
            BeepResponse::decode(&[BEEP_COMMAND, 1]).map(BeepResponse::status),
            Ok(BeepStatus::Unavailable)
        );
        assert_eq!(
            BeepResponse::decode(&[BEEP_COMMAND, 2]).map(BeepResponse::status),
            Ok(BeepStatus::Rejected)
        );
    }

    #[test]
    fn codec_rejects_malformed_responses() {
        assert_eq!(
            BeepResponse::decode(&[BEEP_COMMAND]),
            Err(BeepResponseError::InvalidLength)
        );
        assert_eq!(
            BeepResponse::decode(&[0, 0]),
            Err(BeepResponseError::UnexpectedCommand)
        );
        assert_eq!(
            BeepResponse::decode(&[BEEP_COMMAND, 3]),
            Err(BeepResponseError::UnknownStatus)
        );
    }
}
