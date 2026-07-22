//! Motor-domain semantic wrappers.

use crate::units::{Current, Frequency, SampleRate, VescSeconds, Voltage};

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

        impl core::ops::Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl core::ops::Mul<f32> for $name {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl core::ops::Div<f32> for $name {
            type Output = Self;

            fn div(self, rhs: f32) -> Self::Output {
                Self(self.0 / rhs)
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self(-self.0)
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
        pub struct $name(VescSeconds);

        impl $name {
            /// Wrap VESC float seconds with motor-domain meaning.
            pub const fn new(duration: VescSeconds) -> Self {
                Self(duration)
            }

            /// Return the typed duration without erasing it to a primitive.
            pub const fn duration(self) -> VescSeconds {
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

    /// First FOC audio channel.
    pub const FIRST: Self = Self(Self::MIN);

    /// Create a checked FOC audio channel.
    pub const fn try_new(channel: u8) -> Result<Self, AudioChannelError> {
        if channel <= Self::MAX {
            Ok(Self(channel))
        } else {
            Err(AudioChannelError { value: channel })
        }
    }

    /// Encode the channel index for the audio boundary.
    pub const fn as_u8(self) -> u8 {
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

impl core::fmt::Display for AudioChannelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "audio channel {} is outside 0..=3", self.value)
    }
}

impl core::error::Error for AudioChannelError {}

/// Firmware motor fault code token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FirmwareFaultCode(i32);

impl FirmwareFaultCode {
    /// Build an internal fault-code token from the firmware enum value.
    #[cfg(any(not(test), feature = "test-support"))]
    pub(crate) const fn from_raw_code(code: i32) -> Self {
        Self(code)
    }

    /// Build a firmware fault-code token from its byte wire representation.
    pub const fn from_wire_code(code: u8) -> Self {
        Self(code as i32)
    }

    /// Return true when the firmware reports no active fault.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

/// Firmware fault code encoded in the app-data byte format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FirmwareFaultWireCode(u8);

impl FirmwareFaultWireCode {
    /// Build a token from an app-data fault-code byte.
    pub const fn from_wire_code(code: u8) -> Self {
        Self(code)
    }

    /// Return the app-data fault-code byte.
    pub const fn wire_code(self) -> u8 {
        self.0
    }
}

impl TryFrom<FirmwareFaultCode> for FirmwareFaultWireCode {
    type Error = core::num::TryFromIntError;

    fn try_from(code: FirmwareFaultCode) -> Result<Self, Self::Error> {
        u8::try_from(code.0).map(Self)
    }
}

/// Positive motor-current limit magnitude.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MotorCurrentLimit(Current);

impl MotorCurrentLimit {
    /// Normalize a configured motor-current limit to its positive magnitude.
    pub const fn new(current: Current) -> Self {
        Self(current.abs())
    }

    /// Preserve a firmware value whose positive-limit contract is already established.
    #[cfg(not(test))]
    pub(crate) const fn from_positive_current(current: Current) -> Self {
        Self(current)
    }

    /// Return the positive current-limit magnitude.
    pub const fn current(self) -> Current {
        self.0
    }

    /// Clamp a signed motor current to this positive magnitude.
    ///
    /// This follows VESC's comparison semantics: a zero limit clamps nonzero
    /// current to signed zero, while NaN operands leave the current unchanged.
    pub const fn clamp(self, current: MotorCurrent) -> MotorCurrent {
        let requested = current.current();
        if requested.abs().as_amps() > self.0.as_amps() {
            MotorCurrent::new(Current::from_amps(self.0.as_amps() * requested.signum()))
        } else {
            current
        }
    }
}

/// Positive battery/input-current limit magnitude.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct InputCurrentLimit(Current);

impl InputCurrentLimit {
    /// Normalize a configured input-current limit to its positive magnitude.
    pub const fn new(current: Current) -> Self {
        Self(current.abs())
    }

    /// Return the positive input-current-limit magnitude.
    pub const fn current(self) -> Current {
        self.0
    }
}

current_type!(MotorCurrent, "Motor phase/current-control current.");
current_type!(BrakeCurrent, "Motor braking current.");
current_type!(HandbrakeCurrent, "Handbrake current command.");
current_type!(PhaseCurrent, "Measured motor phase current.");
current_type!(TotalMotorCurrent, "Total motor current.");
current_type!(DirectionalMotorCurrent, "Signed/directional motor current.");
current_type!(AverageMotorCurrent, "Average motor current statistic.");
current_type!(PeakMotorCurrent, "Peak motor current statistic.");
current_type!(DCurrent, "FOC d-axis current.");
current_type!(QCurrent, "FOC q-axis current.");
current_type!(OpenLoopCurrent, "Open-loop motor current command.");
voltage_type!(DVoltage, "FOC d-axis voltage.");
voltage_type!(QVoltage, "FOC q-axis voltage.");
voltage_type!(AudioVoltage, "Audio/haptic voltage command.");

/// Explicit motor-control thread selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MotorSelection(u8);

impl MotorSelection {
    /// Select a motor-control thread by its firmware index.
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    /// Return the firmware motor-control thread index.
    pub const fn index(self) -> u8 {
        self.0
    }
}
frequency_type!(AudioFrequency, "Audio/haptic frequency command.");
sample_rate_type!(AudioSampleRate, "Sample rate for audio sample playback.");
seconds_type!(AudioDuration, "Audio/haptic playback duration.");

#[cfg(test)]
mod tests {
    use super::{MotorCurrent, MotorCurrentLimit};
    use crate::Current;

    #[test]
    fn current_limit_normalizes_firmware_sign() {
        let limit = MotorCurrentLimit::new(Current::from_amps(-40.0));

        assert_eq!(limit.current(), Current::from_amps(40.0));
    }

    #[test]
    fn current_limit_clamps_magnitude_and_preserves_direction() {
        let limit = MotorCurrentLimit::new(Current::from_amps(40.0));

        assert_eq!(
            limit.clamp(MotorCurrent::new(Current::from_amps(25.0))),
            MotorCurrent::new(Current::from_amps(25.0))
        );
        assert_eq!(
            limit.clamp(MotorCurrent::new(Current::from_amps(50.0))),
            MotorCurrent::new(Current::from_amps(40.0))
        );
        assert_eq!(
            limit.clamp(MotorCurrent::new(Current::from_amps(-50.0))),
            MotorCurrent::new(Current::from_amps(-40.0))
        );
    }

    #[test]
    fn zero_current_limit_clamps_nonzero_current_to_signed_zero() {
        let limit = MotorCurrentLimit::new(Current::from_amps(0.0));

        assert_eq!(
            limit
                .clamp(MotorCurrent::new(Current::from_amps(4.0)))
                .current()
                .as_amps()
                .to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(
            limit
                .clamp(MotorCurrent::new(Current::from_amps(-4.0)))
                .current()
                .as_amps()
                .to_bits(),
            (-0.0_f32).to_bits()
        );
    }

    #[test]
    fn nan_current_or_limit_follows_vesc_comparison_semantics() {
        let nan_current = MotorCurrent::new(Current::from_amps(f32::NAN));
        let finite_limit = MotorCurrentLimit::new(Current::from_amps(40.0));
        assert!(finite_limit.clamp(nan_current).current().as_amps().is_nan());

        let nan_limit = MotorCurrentLimit::new(Current::from_amps(f32::NAN));
        let finite_current = MotorCurrent::new(Current::from_amps(50.0));
        assert_eq!(nan_limit.clamp(finite_current), finite_current);
    }
}
