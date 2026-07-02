//! Motor-domain semantic wrappers.

use crate::units::{AbiSeconds, Current, Frequency, SampleRate, Voltage};

macro_rules! current_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Current);

        impl $name {
            /// Wrap a generic current with VESC-domain meaning.
            pub const fn new(current: Current) -> Self {
                Self(current)
            }

            /// Return the typed current without erasing it to a primitive.
            pub const fn current(self) -> Current {
                self.0
            }
        }
    };
}

macro_rules! voltage_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Voltage);

        impl $name {
            /// Wrap a generic voltage with VESC-domain meaning.
            pub const fn new(voltage: Voltage) -> Self {
                Self(voltage)
            }

            /// Return the typed voltage without erasing it to a primitive.
            pub const fn voltage(self) -> Voltage {
                self.0
            }
        }
    };
}

macro_rules! frequency_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(Frequency);

        impl $name {
            /// Wrap a generic frequency with VESC-domain meaning.
            pub const fn new(frequency: Frequency) -> Self {
                Self(frequency)
            }

            /// Return the typed frequency without erasing it to a primitive.
            pub const fn frequency(self) -> Frequency {
                self.0
            }
        }
    };
}

macro_rules! sample_rate_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(SampleRate);

        impl $name {
            /// Wrap a generic sample rate with VESC-domain meaning.
            pub const fn new(sample_rate: SampleRate) -> Self {
                Self(sample_rate)
            }

            /// Return the typed sample rate without erasing it to a primitive.
            pub const fn sample_rate(self) -> SampleRate {
                self.0
            }
        }
    };
}

macro_rules! seconds_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(AbiSeconds);

        impl $name {
            /// Wrap package ABI seconds with VESC-domain meaning.
            pub const fn new(duration: AbiSeconds) -> Self {
                Self(duration)
            }

            /// Return the typed duration without erasing it to a primitive.
            pub const fn duration(self) -> AbiSeconds {
                self.0
            }
        }
    };
}

/// Number of FOC audio channels exposed by BLDC firmware.
pub const AUDIO_CHANNEL_COUNT: u8 = 4;

/// FOC audio channel token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct AudioChannel(u8);

impl AudioChannel {
    /// Lowest accepted FOC audio channel.
    pub const MIN: u8 = 0;

    /// Highest accepted FOC audio channel.
    pub const MAX: u8 = AUDIO_CHANNEL_COUNT - 1;

    /// Create a checked FOC audio channel.
    pub const fn try_new(channel: u8) -> Result<Self, AudioChannelError> {
        if channel <= Self::MAX {
            Ok(Self(channel))
        } else {
            Err(AudioChannelError { value: channel })
        }
    }

    /// Explicitly extract the raw channel index.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Error returned when an audio channel is outside the firmware range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioChannelError {
    value: u8,
}

impl AudioChannelError {
    /// Return the rejected channel.
    pub const fn value(self) -> u8 {
        self.value
    }
}

/// Firmware motor fault code token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FirmwareFaultCode(u8);

impl FirmwareFaultCode {
    /// Build a firmware fault-code token from the app-data compatible byte.
    pub const fn from_compat_code(code: u8) -> Self {
        Self(code)
    }

    /// Return the app-data compatible fault code byte.
    pub const fn compat_code(self) -> u8 {
        self.0
    }

    /// Return true when the firmware reports no active fault.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

current_type!(MotorCurrent, "Motor phase/current-control current.");
current_type!(BrakeCurrent, "Motor braking current.");
current_type!(HandbrakeCurrent, "Handbrake current command.");
current_type!(PhaseCurrent, "Measured motor phase current.");
current_type!(TotalMotorCurrent, "Total motor current.");
current_type!(DirectionalMotorCurrent, "Signed/directional motor current.");
current_type!(DCurrent, "FOC d-axis current.");
current_type!(QCurrent, "FOC q-axis current.");
current_type!(OpenLoopCurrent, "Open-loop motor current command.");
voltage_type!(DVoltage, "FOC d-axis voltage.");
voltage_type!(QVoltage, "FOC q-axis voltage.");
voltage_type!(AudioVoltage, "Audio/haptic voltage command.");
frequency_type!(AudioFrequency, "Audio/haptic frequency command.");
sample_rate_type!(AudioSampleRate, "Sample rate for audio sample playback.");
seconds_type!(AudioDuration, "Audio/haptic playback duration.");
