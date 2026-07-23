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

            /// Return the absolute current while preserving the domain wrapper.
            pub const fn abs(self) -> Self {
                Self(self.0.abs())
            }

            /// Return true when this current is greater than zero.
            pub const fn is_positive(self) -> bool {
                self.0.is_positive()
            }

            /// Return true when this current is less than zero.
            pub const fn is_negative(self) -> bool {
                self.0.is_negative()
            }

            /// Return true when this current is exactly zero.
            pub const fn is_zero(self) -> bool {
                self.0.is_zero()
            }

            /// Return true when the wrapped current is finite.
            pub const fn is_finite(self) -> bool {
                self.0.as_amps().is_finite()
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

/// Known active motor-fault identifiers from the pinned `mc_fault_code` ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FirmwareFaultId {
    /// Controller over-voltage.
    OverVoltage,
    /// Controller under-voltage.
    UnderVoltage,
    /// Gate-driver fault.
    Drv,
    /// Absolute over-current.
    AbsoluteOverCurrent,
    /// FET over-temperature.
    OverTemperatureFet,
    /// Motor over-temperature.
    OverTemperatureMotor,
    /// Gate-driver over-voltage.
    GateDriverOverVoltage,
    /// Gate-driver under-voltage.
    GateDriverUnderVoltage,
    /// MCU under-voltage.
    McuUnderVoltage,
    /// Booting after a watchdog reset.
    BootingFromWatchdogReset,
    /// SPI encoder fault.
    EncoderSpi,
    /// Sin/cos encoder amplitude below minimum.
    EncoderSincosBelowMinAmplitude,
    /// Sin/cos encoder amplitude above maximum.
    EncoderSincosAboveMaxAmplitude,
    /// Main flash corruption.
    FlashCorruption,
    /// Current-sensor-one offset too high.
    HighOffsetCurrentSensor1,
    /// Current-sensor-two offset too high.
    HighOffsetCurrentSensor2,
    /// Current-sensor-three offset too high.
    HighOffsetCurrentSensor3,
    /// Phase currents are unbalanced.
    UnbalancedCurrents,
    /// Brake fault.
    Brk,
    /// Resolver loss of tracking.
    ResolverLot,
    /// Resolver DOS fault.
    ResolverDos,
    /// Resolver loss of signal.
    ResolverLos,
    /// Application configuration flash corruption.
    FlashCorruptionAppConfig,
    /// Motor configuration flash corruption.
    FlashCorruptionMcConfig,
    /// Encoder magnet not detected.
    EncoderNoMagnet,
    /// Encoder magnet is too strong.
    EncoderMagnetTooStrong,
    /// Phase filter fault.
    PhaseFilter,
}

impl FirmwareFaultId {
    /// Convert the known ABI identifier to the app-data compatibility byte.
    pub const fn wire_code(self) -> FirmwareFaultWireCode {
        FirmwareFaultWireCode(self as u8 + 1)
    }

    pub(crate) const fn from_raw_code(code: i32) -> Option<Self> {
        Some(match code {
            1 => Self::OverVoltage,
            2 => Self::UnderVoltage,
            3 => Self::Drv,
            4 => Self::AbsoluteOverCurrent,
            5 => Self::OverTemperatureFet,
            6 => Self::OverTemperatureMotor,
            7 => Self::GateDriverOverVoltage,
            8 => Self::GateDriverUnderVoltage,
            9 => Self::McuUnderVoltage,
            10 => Self::BootingFromWatchdogReset,
            11 => Self::EncoderSpi,
            12 => Self::EncoderSincosBelowMinAmplitude,
            13 => Self::EncoderSincosAboveMaxAmplitude,
            14 => Self::FlashCorruption,
            15 => Self::HighOffsetCurrentSensor1,
            16 => Self::HighOffsetCurrentSensor2,
            17 => Self::HighOffsetCurrentSensor3,
            18 => Self::UnbalancedCurrents,
            19 => Self::Brk,
            20 => Self::ResolverLot,
            21 => Self::ResolverDos,
            22 => Self::ResolverLos,
            23 => Self::FlashCorruptionAppConfig,
            24 => Self::FlashCorruptionMcConfig,
            25 => Self::EncoderNoMagnet,
            26 => Self::EncoderMagnetTooStrong,
            27 => Self::PhaseFilter,
            _ => return None,
        })
    }
}

/// Semantic result of reading the firmware motor-fault slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareFault {
    /// No motor fault is active.
    None,
    /// A known active firmware fault.
    Active(FirmwareFaultId),
    /// Firmware returned a value this SDK does not understand.
    Unknown,
}

impl FirmwareFault {
    pub(crate) const fn from_raw_code(code: i32) -> Self {
        match code {
            0 => Self::None,
            code => match FirmwareFaultId::from_raw_code(code) {
                Some(fault) => Self::Active(fault),
                None => Self::Unknown,
            },
        }
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
        if requested.abs().is_greater_than(self.0) {
            MotorCurrent::new(self.0.scaled_by(requested.signum()))
        } else {
            current
        }
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

    #[test]
    fn motor_current_predicates_preserve_the_domain_wrapper() {
        let current = MotorCurrent::new(Current::from_amps(-4.0));

        assert!(current.is_negative());
        assert!(!current.is_positive());
        assert!(!current.is_zero());
        assert_eq!(current.abs(), MotorCurrent::new(Current::from_amps(4.0)));
        assert!(current.is_finite());
        assert!(!MotorCurrent::new(Current::from_amps(f32::NAN)).is_finite());
    }
}
