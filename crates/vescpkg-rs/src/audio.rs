//! Checked access to the optional FOC audio subsystem.

use core::ffi::c_int;
use core::ptr::NonNull;

use crate::{AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate, AudioVoltage};

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
        match unsafe { crate::ffi::foc_beep(frequency, duration, voltage) } {
            None => Err(FocAudioError::Unavailable),
            Some(true) => Ok(()),
            Some(false) => Err(FocAudioError::Rejected),
        }
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
        match unsafe { crate::ffi::foc_play_tone(channel.as_u8() as c_int, frequency, voltage) } {
            None => Err(FocAudioError::Unavailable),
            Some(true) => Ok(()),
            Some(false) => Err(FocAudioError::Rejected),
        }
    }

    /// Stop active FOC audio output.
    pub fn stop(&self, reset: bool) -> Result<(), FocAudioError> {
        unsafe { crate::ffi::foc_stop_audio(reset) }
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
        match unsafe {
            crate::ffi::foc_play_audio_samples(samples.as_ptr(), length, sample_rate, voltage)
        } {
            None => Err(FocAudioError::Unavailable),
            Some(true) => Ok(()),
            Some(false) => Err(FocAudioError::Rejected),
        }
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
        match unsafe {
            crate::ffi::foc_set_audio_sample_table(
                channel.as_u8() as c_int,
                samples.as_ptr(),
                length,
            )
        } {
            None => Err(FocAudioError::Unavailable),
            Some(false) => Err(FocAudioError::Rejected),
            Some(true) => Ok(FocAudioSampleTable {
                audio: *self,
                _samples: samples,
            }),
        }
    }

    /// Return the firmware-owned table pointer for inspection.
    ///
    /// # Safety
    ///
    /// The returned pointer has no length metadata and is only valid for as
    /// long as the firmware retains the corresponding table. The caller must
    /// not dereference it after its lease is dropped or turn it into a slice
    /// without separately knowing the table length.
    pub unsafe fn sample_table_ptr(&self, channel: AudioChannel) -> Option<NonNull<f32>> {
        unsafe { crate::ffi::foc_get_audio_sample_table(channel.as_u8() as c_int) }
            .and_then(|pointer| NonNull::new(pointer as *mut f32))
    }
}

impl Drop for FocAudioSampleTable<'_> {
    fn drop(&mut self) {
        let _ = self.audio.stop(true);
    }
}

impl crate::Firmware {
    /// Return the optional FOC audio capability handle.
    pub fn audio(&self) -> FocAudio {
        FocAudio::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional FOC audio capability handle.
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
    (length <= i32::MAX as usize)
        .then_some(length as c_int)
        .ok_or(FocAudioError::BufferTooLong)
}
