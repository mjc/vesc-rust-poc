//! Restrained, fixed-output FOC-audio hardware seam.

use vesc_protocol::audio_smoke::{
    BEEP_COMMAND, BEEP_RESPONSE_BYTES, BeepStatus, encode_beep_response,
};
use vescpkg_rs::{
    AudioDuration, AudioFrequency, AudioVoltage, FocAudio, FocAudioError, Frequency, VescSeconds,
    Voltage,
};

/// Error returned before the fixed audio request reaches firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSmokeCommandError {
    /// The recognized command carried unexpected payload bytes.
    InvalidLength,
    /// The caller's response storage cannot hold the fixed acknowledgement.
    ResponseTooShort,
}

/// Handle the fixed short-beep command, or return `None` for another protocol.
///
/// # Errors
///
/// Returns an error when the recognized request has the wrong length or the
/// response buffer is too short.
pub fn handle_audio_smoke_command(
    audio: FocAudio,
    packet: &[u8],
    response: &mut [u8],
) -> Result<Option<usize>, AudioSmokeCommandError> {
    if packet.first().copied() != Some(BEEP_COMMAND) {
        return Ok(None);
    }
    if packet.len() != 1 {
        return Err(AudioSmokeCommandError::InvalidLength);
    }
    let output = response
        .get_mut(..BEEP_RESPONSE_BYTES)
        .ok_or(AudioSmokeCommandError::ResponseTooShort)?;
    let status = match audio.beep(
        AudioFrequency::new(Frequency::from_hertz(440.0)),
        AudioDuration::new(VescSeconds::from_seconds(0.05)),
        AudioVoltage::new(Voltage::from_volts(0.5)),
    ) {
        Ok(()) => BeepStatus::Played,
        Err(FocAudioError::Unavailable) => BeepStatus::Unavailable,
        Err(_) => BeepStatus::Rejected,
    };
    output.copy_from_slice(&encode_beep_response(status));
    Ok(Some(BEEP_RESPONSE_BYTES))
}

#[cfg(test)]
mod tests {
    use vesc_protocol::audio_smoke::{BEEP_COMMAND, BeepResponse, BeepStatus, encode_beep_command};
    use vescpkg_rs::test_support::FirmwareTest;

    use super::{AudioSmokeCommandError, handle_audio_smoke_command};

    #[test]
    fn fixed_beep_reports_played_or_unavailable() {
        let firmware = FirmwareTest::new();
        let mut response = [0_u8; 2];

        let len =
            handle_audio_smoke_command(firmware.audio(), &encode_beep_command(), &mut response)
                .expect("valid command")
                .expect("audio command");
        assert_eq!(len, response.len());
        assert_eq!(
            BeepResponse::decode(&response).map(BeepResponse::status),
            Ok(BeepStatus::Played)
        );

        firmware.set_audio_available(false);
        handle_audio_smoke_command(firmware.audio(), &encode_beep_command(), &mut response)
            .expect("valid command")
            .expect("audio command");
        assert_eq!(
            BeepResponse::decode(&response).map(BeepResponse::status),
            Ok(BeepStatus::Unavailable)
        );
    }

    #[test]
    fn fixed_beep_rejects_malformed_requests_without_claiming_other_packets() {
        let firmware = FirmwareTest::new();
        let mut response = [0_u8; 2];

        assert_eq!(
            handle_audio_smoke_command(firmware.audio(), &[BEEP_COMMAND, 0], &mut response),
            Err(AudioSmokeCommandError::InvalidLength)
        );
        assert_eq!(
            handle_audio_smoke_command(firmware.audio(), &[0], &mut response),
            Ok(None)
        );
        assert_eq!(
            handle_audio_smoke_command(firmware.audio(), &[BEEP_COMMAND], &mut []),
            Err(AudioSmokeCommandError::ResponseTooShort)
        );
    }
}
