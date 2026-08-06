//! Checked access to the optional FOC audio subsystem.
#![allow(
    clippy::missing_errors_doc,
    reason = "error variants document failures"
)]

use core::ffi::c_int;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::{AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate, AudioVoltage};

static AUDIO_SAMPLE_TABLE_LEASE: AtomicBool = AtomicBool::new(false);

/// Failure returned by a FOC audio operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FocAudioError {
    /// The loaded firmware does not expose this audio slot.
    Unavailable,
    /// Firmware rejected an otherwise well-formed command.
    Rejected,
    /// A typed value or sample payload is not valid for the ABI.
    InvalidParameter,
    /// A sample buffer length cannot be represented by the C ABI.
    BufferTooLong,
    /// Another package-owned lease already owns the firmware audio table.
    Busy,
}

impl_error!(FocAudioError {
    Unavailable => "FOC audio capability is unavailable",
    Rejected => "firmware rejected the FOC audio command",
    InvalidParameter => "FOC audio parameter is invalid",
    BufferTooLong => "FOC audio buffer exceeds the firmware ABI limit",
    Busy => "FOC audio sample table is already owned",
});

/// Select whether stopping FOC audio also resets firmware audio state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocAudioStopMode {
    /// Stop output while preserving the firmware's audio state.
    Preserve,
    /// Stop output and reset the firmware's audio state.
    Reset,
}

impl FocAudioStopMode {
    /// Return the raw reset flag expected by the firmware ABI.
    #[must_use]
    pub const fn resets(self) -> bool {
        matches!(self, Self::Reset)
    }
}

/// Handle for the optional FOC audio entrypoints.
#[derive(Debug, Clone, Copy, Default)]
pub struct FocAudio;

/// Lease keeping a sample table's backing slice borrowed while firmware owns
/// its pointer.
pub struct FocAudioSampleTable<'a> {
    audio: FocAudio,
    _samples: &'a [f32],
}

impl FocAudio {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Trigger a short audio beep.
    pub fn beep(
        &self,
        frequency: AudioFrequency,
        duration: AudioDuration,
        voltage: AudioVoltage,
    ) -> Result<(), FocAudioError> {
        let frequency = positive(frequency.frequency().as_hertz())?;
        let duration = positive(duration.duration().as_seconds())?;
        let voltage = nonnegative(voltage.voltage().as_volts())?;
        command_result(unsafe { crate::ffi::foc_beep(frequency, duration, voltage) })
    }

    /// Play a continuous tone on one of the firmware's audio channels.
    pub fn play_tone(
        &self,
        channel: AudioChannel,
        frequency: AudioFrequency,
        voltage: AudioVoltage,
    ) -> Result<(), FocAudioError> {
        let frequency = positive(frequency.frequency().as_hertz())?;
        let voltage = nonnegative(voltage.voltage().as_volts())?;
        command_result(unsafe {
            crate::ffi::foc_play_tone(c_int::from(channel.as_u8()), frequency, voltage)
        })
    }

    /// Stop active FOC audio output with an explicit reset policy.
    pub fn stop(&self, mode: FocAudioStopMode) -> Result<(), FocAudioError> {
        unsafe { crate::ffi::foc_stop_audio(mode.resets()) }
            .then_some(())
            .ok_or(FocAudioError::Unavailable)
    }

    /// Play signed 8-bit samples at a checked sample rate.
    pub fn play_samples(
        &self,
        samples: &[i8],
        sample_rate: AudioSampleRate,
        voltage: AudioVoltage,
    ) -> Result<(), FocAudioError> {
        let length = c_int_length(samples.len())?;
        if samples.is_empty() {
            return Err(FocAudioError::InvalidParameter);
        }
        let sample_rate = positive(sample_rate.sample_rate().as_hertz())?;
        let voltage = nonnegative(voltage.voltage().as_volts())?;
        command_result(unsafe {
            crate::ffi::foc_play_audio_samples(samples.as_ptr(), length, sample_rate, voltage)
        })
    }

    /// Install a sample table and hold its backing slice borrowed until the
    /// returned lease is dropped.
    pub fn set_sample_table<'a>(
        &self,
        channel: AudioChannel,
        samples: &'a [f32],
    ) -> Result<FocAudioSampleTable<'a>, FocAudioError> {
        let length = c_int_length(samples.len())?;
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
            return Err(FocAudioError::InvalidParameter);
        }
        if !unsafe { crate::ffi::foc_stop_audio_available() } {
            return Err(FocAudioError::Unavailable);
        }
        if AUDIO_SAMPLE_TABLE_LEASE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(FocAudioError::Busy);
        }
        match unsafe {
            crate::ffi::foc_set_audio_sample_table(
                c_int::from(channel.as_u8()),
                samples.as_ptr(),
                length,
            )
        } {
            None => {
                release_sample_table();
                Err(FocAudioError::Unavailable)
            }
            Some(false) => {
                release_sample_table();
                Err(FocAudioError::Rejected)
            }
            Some(true) => Ok(FocAudioSampleTable {
                audio: *self,
                _samples: samples,
            }),
        }
    }

    /// Return the firmware-owned table pointer for inspection without erasing
    /// the firmware's `const` contract.
    ///
    /// # Safety
    ///
    /// The returned pointer has no length metadata and is only valid for as
    /// long as the firmware retains the corresponding table. The caller must
    /// not dereference it after its lease is dropped or turn it into a slice
    /// without separately knowing the table length.
    #[must_use]
    pub unsafe fn sample_table_ptr(&self, channel: AudioChannel) -> Option<*const f32> {
        unsafe { crate::ffi::foc_get_audio_sample_table(c_int::from(channel.as_u8())) }
    }
}

impl Drop for FocAudioSampleTable<'_> {
    fn drop(&mut self) {
        let _ = self.audio.stop(FocAudioStopMode::Reset);
        release_sample_table();
    }
}

impl crate::Firmware {
    /// Return the optional FOC audio capability handle.
    #[must_use]
    pub fn audio(&self) -> FocAudio {
        FocAudio::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional FOC audio capability handle.
    #[must_use]
    pub fn audio(&self) -> FocAudio {
        FocAudio::new()
    }
}

fn positive(value: f32) -> Result<f32, FocAudioError> {
    (value.is_finite() && value > 0.0)
        .then_some(value)
        .ok_or(FocAudioError::InvalidParameter)
}

fn nonnegative(value: f32) -> Result<f32, FocAudioError> {
    (value.is_finite() && value >= 0.0)
        .then_some(value)
        .ok_or(FocAudioError::InvalidParameter)
}

fn c_int_length(length: usize) -> Result<c_int, FocAudioError> {
    c_int::try_from(length).map_err(|_| FocAudioError::BufferTooLong)
}

fn release_sample_table() {
    AUDIO_SAMPLE_TABLE_LEASE.store(false, Ordering::Release);
}

fn command_result(result: Option<bool>) -> Result<(), FocAudioError> {
    match result {
        None => Err(FocAudioError::Unavailable),
        Some(true) => Ok(()),
        Some(false) => Err(FocAudioError::Rejected),
    }
}
