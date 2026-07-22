//! Configuration semantic wrappers.

use core::marker::PhantomData;

/// One protocol byte kept typed while mapping wire values into semantic units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct WireByte(u8);

impl WireByte {
    /// Wrap one byte received from a validated protocol payload.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Return the validated protocol byte.
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Apply a wire scale and offset directly through a semantic constructor.
    pub fn scaled<T>(self, scale: f32, offset: f32, constructor: fn(f32) -> T) -> T {
        constructor(f32::from(self.0) * scale + offset)
    }

    /// Apply a rational wire scale in protocol operation order.
    ///
    /// # Panics
    ///
    /// Panics when `denominator` is zero or non-finite.
    pub fn scaled_ratio<T>(
        self,
        numerator: f32,
        denominator: f32,
        offset: f32,
        constructor: fn(f32) -> T,
    ) -> T {
        assert!(
            denominator.is_finite() && denominator != 0.0,
            "wire scale denominator must be finite and non-zero"
        );
        constructor((f32::from(self.0) * numerator) / denominator + offset)
    }
}

pub use crate::units::{BatteryCellCount, BatteryCellCountError};
use crate::units::{Distance, FluxLinkage, Inductance, Resistance};

/// Battery chemistry selector used by the VESC motor configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BatteryChemistry {
    /// Lithium-ion chemistry with a 3.0–4.2 V cell range.
    LithiumIon,
    /// Lithium-iron-phosphate chemistry with a 2.6–3.6 V cell range.
    LithiumIronPhosphate,
    /// Lead-acid chemistry.
    LeadAcid,
}

impl BatteryChemistry {
    /// Decode the VESC `BATTERY_TYPE` enum value.
    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::LithiumIon),
            1 => Some(Self::LithiumIronPhosphate),
            2 => Some(Self::LeadAcid),
            _ => None,
        }
    }

    /// Encode the VESC `BATTERY_TYPE` enum value.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::LithiumIon => 0,
            Self::LithiumIronPhosphate => 1,
            Self::LeadAcid => 2,
        }
    }
}

/// CAN baud-rate selector used by the VESC application configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanBaudRate {
    /// 125 kbit/s (`CAN_BAUD_125K`).
    Kbps125,
    /// 250 kbit/s (`CAN_BAUD_250K`).
    Kbps250,
    /// 500 kbit/s (`CAN_BAUD_500K`).
    Kbps500,
    /// 1 Mbit/s (`CAN_BAUD_1M`).
    Mbps1,
    /// 10 kbit/s (`CAN_BAUD_10K`).
    Kbps10,
    /// 20 kbit/s (`CAN_BAUD_20K`).
    Kbps20,
    /// 50 kbit/s (`CAN_BAUD_50K`).
    Kbps50,
    /// 75 kbit/s (`CAN_BAUD_75K`).
    Kbps75,
    /// 100 kbit/s (`CAN_BAUD_100K`).
    Kbps100,
}

impl CanBaudRate {
    /// Decode the VESC `CAN_BAUD` enum value.
    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Kbps125),
            1 => Some(Self::Kbps250),
            2 => Some(Self::Kbps500),
            3 => Some(Self::Mbps1),
            4 => Some(Self::Kbps10),
            5 => Some(Self::Kbps20),
            6 => Some(Self::Kbps50),
            7 => Some(Self::Kbps75),
            8 => Some(Self::Kbps100),
            _ => None,
        }
    }

    /// Encode the VESC `CAN_BAUD` enum value.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Kbps125 => 0,
            Self::Kbps250 => 1,
            Self::Kbps500 => 2,
            Self::Mbps1 => 3,
            Self::Kbps10 => 4,
            Self::Kbps20 => 5,
            Self::Kbps50 => 6,
            Self::Kbps75 => 7,
            Self::Kbps100 => 8,
        }
    }
}

macro_rules! positive_count_type {
    ($name:ident, $error:ident, $doc:literal, $error_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(u16);

        impl $name {
            /// Create a checked non-zero count.
            pub const fn try_new(count: u16) -> Result<Self, $error> {
                if count == 0 {
                    Err($error { value: count })
                } else {
                    Ok(Self(count))
                }
            }

            /// Encode the count for the firmware boundary.
            pub const fn as_u16(self) -> u16 {
                self.0
            }
        }

        #[doc = $error_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $error {
            value: u16,
        }

        impl $error {
            /// Return the rejected count.
            pub const fn value(self) -> u16 {
                self.value
            }
        }
    };
}

macro_rules! unit_type {
    ($name:ident, $inner:ty, $new_arg:ident, $accessor:ident, $doc:literal, $accessor_doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Wrap a generic unit with config meaning.
            pub const fn new($new_arg: $inner) -> Self {
                Self($new_arg)
            }

            #[doc = $accessor_doc]
            pub const fn $accessor(self) -> $inner {
                self.0
            }
        }
    };
}

/// Fixed-size serialized VESC custom-config bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigImage<const LEN: usize>([u8; LEN]);

impl<const LEN: usize> CustomConfigImage<LEN> {
    /// Wrap already validated custom-config bytes.
    pub const fn new(bytes: [u8; LEN]) -> Self {
        Self(bytes)
    }

    /// Parse bytes with the generated 32-bit config signature.
    pub fn from_serialized(bytes: &[u8], signature: [u8; 4]) -> Option<Self> {
        let bytes = <&[u8; LEN]>::try_from(bytes).ok()?;
        bytes.starts_with(&signature).then_some(Self(*bytes))
    }

    /// Return the serialized config bytes.
    pub const fn as_bytes(&self) -> &[u8; LEN] {
        &self.0
    }

    /// Mutably borrow the serialized config bytes.
    pub fn as_mut_bytes(&mut self) -> &mut [u8; LEN] {
        &mut self.0
    }

    /// Read one big-endian `u16`, returning `None` for out-of-range generated offsets.
    pub(crate) fn be_u16_at(&self, offset: usize) -> Option<u16> {
        let end = offset.checked_add(2)?;
        let bytes = <&[u8; 2]>::try_from(self.0.get(offset..end)?).ok()?;
        Some(u16::from_be_bytes(*bytes))
    }

    /// Read a generated boolean flag.
    fn flag_at(&self, offset: usize) -> Option<bool> {
        self.0.get(offset).map(|value| *value != 0)
    }

    /// Edit this config image in place.
    pub fn editor(&mut self) -> CustomConfigEditor<'_, LEN> {
        CustomConfigEditor(&mut self.0)
    }
}

/// In-place editor for fixed-size serialized VESC custom-config bytes.
pub struct CustomConfigEditor<'a, const LEN: usize>(&'a mut [u8; LEN]);

impl<const LEN: usize> CustomConfigEditor<'_, LEN> {
    /// Write one byte for crate-internal field descriptors.
    pub(crate) fn set_byte_at(&mut self, offset: usize, value: u8) -> Option<()> {
        let byte = self.0.get_mut(offset)?;
        *byte = value;
        Some(())
    }

    /// Write one big-endian `u16`, returning `None` when the field is out of range.
    pub(crate) fn set_be_u16_at(&mut self, offset: usize, value: u16) -> Option<()> {
        let bytes = offset
            .checked_add(2)
            .and_then(|end| self.0.get_mut(offset..end))
            .and_then(|bytes| <&mut [u8; 2]>::try_from(bytes).ok())?;
        *bytes = value.to_be_bytes();
        Some(())
    }
}

/// Byte offset into generated custom-config bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct CustomConfigOffset(usize);

impl CustomConfigOffset {
    /// Create a generated config offset.
    const fn new(offset: usize) -> Self {
        Self(offset)
    }

    /// Explicitly extract the raw byte offset.
    const fn get(self) -> usize {
        self.0
    }
}

/// Generated custom-config millisecond-duration field descriptor.
///
/// C map: Float Out Boy decodes generated unsigned 16-bit values in big-endian
/// order at `third_party/float-out-boy/src/conf/buffer.c:188-191`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigDurationField(CustomConfigOffset);

impl CustomConfigDurationField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        Self(CustomConfigOffset::new(offset))
    }

    /// Decode the field directly into a duration.
    #[inline(always)]
    pub fn read<const LEN: usize>(
        self,
        image: &CustomConfigImage<LEN>,
    ) -> Option<crate::VescSeconds> {
        image
            .be_u16_at(self.0.get())
            .map(|value| crate::VescSeconds::from_seconds(f32::from(value) / 1000.0))
    }

    /// Encode a semantic duration into its generated millisecond field.
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        duration: crate::VescSeconds,
    ) -> Option<()> {
        editor.set_be_u16_at(self.0.get(), finite_u16(duration.as_seconds() * 1000.0)?)
    }
}

/// Generated custom-config voltage field descriptor.
///
/// C map: Float Out Boy decodes generated unsigned 16-bit values in big-endian
/// order at `third_party/float-out-boy/src/conf/buffer.c:188-191`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigVoltageField(CustomConfigOffset);

impl CustomConfigVoltageField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        Self(CustomConfigOffset::new(offset))
    }

    /// Decode the field directly into voltage.
    #[inline(always)]
    pub fn read<const LEN: usize>(
        self,
        image: &CustomConfigImage<LEN>,
    ) -> Option<crate::units::Voltage> {
        image
            .be_u16_at(self.0.get())
            .map(|value| crate::units::Voltage::from_volts(f32::from(value) / 1000.0))
    }

    /// Encode semantic voltage into its generated millivolt field.
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        voltage: crate::units::Voltage,
    ) -> Option<()> {
        editor.set_be_u16_at(self.0.get(), finite_u16(voltage.as_volts() * 1000.0)?)
    }
}

/// Generated custom-config electrical-speed field descriptor.
///
/// C map: Float Out Boy decodes generated unsigned 16-bit values in big-endian
/// order at `third_party/float-out-boy/src/conf/buffer.c:188-191`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigElectricalSpeedField(CustomConfigOffset);

impl CustomConfigElectricalSpeedField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        Self(CustomConfigOffset::new(offset))
    }

    /// Decode the field directly into electrical RPM.
    #[inline(always)]
    pub fn read<const LEN: usize>(
        self,
        image: &CustomConfigImage<LEN>,
    ) -> Option<super::motion::ElectricalSpeed> {
        image.be_u16_at(self.0.get()).map(|value| {
            super::motion::ElectricalSpeed::new(crate::units::Rpm::from_revolutions_per_minute(
                f32::from(value),
            ))
        })
    }

    /// Encode semantic electrical speed into its generated ERPM field.
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        speed: super::motion::ElectricalSpeed,
    ) -> Option<()> {
        editor.set_be_u16_at(
            self.0.get(),
            finite_u16(speed.rpm().as_revolutions_per_minute())?,
        )
    }
}

/// Generated custom-config sample-rate field descriptor.
///
/// C map: Float Out Boy decodes generated unsigned 16-bit values in big-endian
/// order at `third_party/float-out-boy/src/conf/buffer.c:188-191`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigSampleRateField(CustomConfigOffset);

impl CustomConfigSampleRateField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        Self(CustomConfigOffset::new(offset))
    }

    /// Decode the field directly into its semantic sample-rate value.
    #[inline(always)]
    pub fn read<const LEN: usize>(
        self,
        image: &CustomConfigImage<LEN>,
    ) -> Option<crate::SampleRate> {
        image
            .be_u16_at(self.0.get())
            .map(|value| crate::SampleRate::from_hertz(f32::from(value)))
    }

    /// Encode a semantic sample rate into its generated unsigned field.
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        sample_rate: crate::SampleRate,
    ) -> Option<()> {
        editor.set_be_u16_at(self.0.get(), finite_u16(sample_rate.as_hertz())?)
    }
}

/// Generated boolean custom-config field descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigFlagField(CustomConfigOffset);

impl CustomConfigFlagField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(offset < LEN, "generated config field is out of bounds");
        Self(CustomConfigOffset::new(offset))
    }

    /// Read the field, returning `None` when its generated offset is invalid.
    #[inline(always)]
    pub fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<bool> {
        image.flag_at(self.0.get())
    }

    /// Write the field, returning `None` when its generated offset is invalid.
    #[inline(always)]
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        value: bool,
    ) -> Option<()> {
        editor.set_byte_at(self.0.get(), u8::from(value))
    }
}

/// Generated raw-byte custom-config field descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigWireByteField(CustomConfigOffset);

impl CustomConfigWireByteField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(offset < LEN, "generated config field is out of bounds");
        Self(CustomConfigOffset::new(offset))
    }

    /// Read the field without erasing its wire type.
    #[inline(always)]
    pub fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<WireByte> {
        image.0.get(self.0.get()).copied().map(WireByte::new)
    }

    /// Write the field without exposing primitive conversion to package code.
    #[inline(always)]
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        value: WireByte,
    ) -> Option<()> {
        editor.set_byte_at(self.0.get(), value.0)
    }
}

/// Generated enum custom-config field descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigEnumField<T>(CustomConfigOffset, PhantomData<fn() -> T>);

impl<T> CustomConfigEnumField<T> {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(offset < LEN, "generated config field is out of bounds");
        Self(CustomConfigOffset::new(offset), PhantomData)
    }

    /// Decode the field into its Rust enum.
    #[inline(always)]
    pub fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<T>
    where
        T: From<u8>,
    {
        image.0.get(self.0.get()).copied().map(T::from)
    }
}

/// Generated two-byte field that a safety policy resets to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomConfigResetField(CustomConfigOffset);

impl CustomConfigResetField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        Self(CustomConfigOffset::new(offset))
    }

    /// Reset the generated field without exposing its storage representation.
    pub fn clear<const LEN: usize>(self, editor: &mut CustomConfigEditor<'_, LEN>) -> Option<()> {
        editor.set_be_u16_at(self.0.get(), 0)
    }
}

// Float Out Boy's generated float16 fields store a signed big-endian integer and a
// generated scale. C map: `third_party/float-out-boy/src/conf/buffer.c:182-210`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CustomConfigScaledField {
    offset: CustomConfigOffset,
    scale: f32,
}

impl CustomConfigScaledField {
    const fn new(offset: usize, scale: f32) -> Option<Self> {
        if scale.is_finite() && scale > 0.0 {
            Some(Self {
                offset: CustomConfigOffset::new(offset),
                scale,
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<f32> {
        image
            .be_u16_at(self.offset.get())
            .map(|value| f32::from(value as i16) / self.scale)
    }

    fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        value: f32,
    ) -> Option<()> {
        editor.set_be_u16_at(self.offset.get(), finite_i16(value * self.scale)? as u16)
    }
}

fn finite_u16(value: f32) -> Option<u16> {
    (value.is_finite() && value >= 0.0 && value <= f32::from(u16::MAX)).then_some(value as u16)
}

fn finite_i16(value: f32) -> Option<i16> {
    (value.is_finite() && value >= f32::from(i16::MIN) && value <= f32::from(i16::MAX))
        .then_some(value as i16)
}

macro_rules! semantic_scaled_config_field {
    ($name:ident, $value:ty, $decode:expr, $encode:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(transparent)]
        pub struct $name(CustomConfigScaledField);

        impl $name {
            #[doc(hidden)]
            pub const fn __from_generated<const LEN: usize>(offset: usize, scale: f32) -> Self {
                assert!(
                    LEN >= 2 && offset <= LEN - 2,
                    "generated config field is out of bounds"
                );
                match CustomConfigScaledField::new(offset, scale) {
                    Some(field) => Self(field),
                    None => panic!("generated config field scale must be finite and positive"),
                }
            }

            /// Decode the field directly into its semantic value.
            #[inline(always)]
            pub fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<$value> {
                self.0.read(image).map($decode)
            }

            /// Encode a semantic value into its generated scaled field.
            pub fn write<const LEN: usize>(
                self,
                editor: &mut CustomConfigEditor<'_, LEN>,
                value: $value,
            ) -> Option<()> {
                self.0.write(editor, ($encode)(value))
            }
        }
    };
}

/// Define a package-local descriptor from generated custom-config metadata.
///
/// The image length and field metadata are checked during constant evaluation,
/// keeping raw offsets and scales at the generated-layout boundary.
///
/// ```compile_fail
/// use vescpkg_rs::{CustomConfigFlagField, generated_custom_config_field};
///
/// const BAD: CustomConfigFlagField = generated_custom_config_field!(
///     CustomConfigFlagField,
///     len: 1,
///     offset: 1
/// );
/// ```
#[macro_export]
macro_rules! generated_custom_config_field {
    ($field:ty, len: $len:expr, offset: $offset:expr) => {
        <$field>::__from_generated::<{ $len }>($offset)
    };
    ($field:ty, len: $len:expr, offset: $offset:expr, scale: $scale:expr) => {
        <$field>::__from_generated::<{ $len }>($offset, $scale)
    };
}

semantic_scaled_config_field!(
    CustomConfigMotorCurrentField,
    crate::MotorCurrent,
    |value| crate::MotorCurrent::new(crate::Current::from_amps(value)),
    |value: crate::MotorCurrent| value.current().as_amps(),
    "Generated scaled motor-current field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigAngleField,
    crate::AngleDegrees,
    crate::AngleDegrees::from_degrees,
    crate::AngleDegrees::as_degrees,
    "Generated scaled angle field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigAngularVelocityField,
    crate::AngularVelocity,
    crate::AngularVelocity::from_degrees_per_second,
    crate::AngularVelocity::as_degrees_per_second,
    "Generated scaled angular-velocity field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigSecondsField,
    crate::VescSeconds,
    crate::VescSeconds::from_seconds,
    crate::VescSeconds::as_seconds,
    "Generated scaled-seconds field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigScaledVoltageField,
    crate::Voltage,
    crate::Voltage::from_volts,
    crate::Voltage::as_volts,
    "Generated scaled-voltage field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigFrequencyField,
    crate::Frequency,
    crate::Frequency::from_hertz,
    crate::Frequency::as_hertz,
    "Generated scaled-frequency field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigMahonyPitchGainField,
    crate::MahonyPitchGain,
    crate::MahonyPitchGain::new,
    crate::MahonyPitchGain::value,
    "Generated Mahony pitch-gain field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigMahonyRollGainField,
    crate::MahonyRollGain,
    crate::MahonyRollGain::new,
    crate::MahonyRollGain::value,
    "Generated Mahony roll-gain field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigAngleCurrentGainField,
    crate::AngleCurrentGain,
    crate::AngleCurrentGain::new,
    crate::AngleCurrentGain::as_amps_per_degree,
    "Generated angle-current-gain field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigRateCurrentGainField,
    crate::RateCurrentGain,
    crate::RateCurrentGain::new,
    crate::RateCurrentGain::as_amps_per_degree_per_second,
    "Generated rate-current-gain field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigIntegralCurrentGainField,
    crate::IntegralCurrentGain,
    crate::IntegralCurrentGain::new,
    crate::IntegralCurrentGain::as_amps_per_degree_per_tick,
    "Generated integral-current-gain field descriptor."
);
semantic_scaled_config_field!(
    CustomConfigPidScaleField,
    crate::PidScale,
    crate::PidScale::new,
    crate::PidScale::value,
    "Generated PID-scale field descriptor."
);

/// Generated unsigned-ratio field descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct CustomConfigRatioField(CustomConfigScaledField);

impl CustomConfigRatioField {
    #[doc(hidden)]
    pub const fn __from_generated<const LEN: usize>(offset: usize, scale: f32) -> Self {
        assert!(
            LEN >= 2 && offset <= LEN - 2,
            "generated config field is out of bounds"
        );
        match CustomConfigScaledField::new(offset, scale) {
            Some(field) => Self(field),
            None => panic!("generated config field scale must be finite and positive"),
        }
    }

    /// Decode the field directly into an unsigned ratio.
    #[inline(always)]
    pub fn read<const LEN: usize>(self, image: &CustomConfigImage<LEN>) -> Option<crate::Ratio> {
        self.0
            .read(image)
            .and_then(|value| crate::Ratio::from_ratio(value).ok())
    }

    /// Encode an unsigned ratio into its generated scaled field.
    pub fn write<const LEN: usize>(
        self,
        editor: &mut CustomConfigEditor<'_, LEN>,
        value: crate::Ratio,
    ) -> Option<()> {
        self.0.write(editor, value.as_ratio())
    }
}

/// Gear reduction ratio configured for speed/distance calculations.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GearRatio(f32);

impl GearRatio {
    /// Create a checked positive gear ratio.
    pub const fn try_new(ratio: f32) -> Result<Self, GearRatioError> {
        if ratio.is_finite() && ratio > 0.0 {
            Ok(Self(ratio))
        } else {
            Err(GearRatioError { value: ratio })
        }
    }

    /// Return the configured ratio for typed calculations.
    pub const fn as_f32(self) -> f32 {
        self.0
    }
}

/// Error returned when a gear ratio is not finite and positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GearRatioError {
    value: f32,
}

impl GearRatioError {
    /// Return the rejected ratio.
    pub const fn value(self) -> f32 {
        self.value
    }
}

impl core::fmt::Display for GearRatioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} is not a finite positive gear ratio", self.value)
    }
}

impl core::error::Error for GearRatioError {}

positive_count_type!(
    MotorPoleCount,
    MotorPoleCountError,
    "Configured motor pole count.",
    "Error returned when the motor pole count is zero."
);
unit_type!(
    WheelDiameter,
    Distance,
    distance,
    distance,
    "Configured wheel diameter.",
    "Return the typed wheel diameter without erasing it to a primitive."
);
unit_type!(
    FocMotorResistance,
    Resistance,
    resistance,
    resistance,
    "Configured FOC motor resistance.",
    "Return the typed motor resistance without erasing it to a primitive."
);
unit_type!(
    FocMotorInductance,
    Inductance,
    inductance,
    inductance,
    "Configured FOC motor inductance.",
    "Return the typed motor inductance without erasing it to a primitive."
);
unit_type!(
    FocMotorFluxLinkage,
    FluxLinkage,
    flux_linkage,
    flux_linkage,
    "Configured FOC motor flux linkage.",
    "Return the typed motor flux linkage without erasing it to a primitive."
);

#[cfg(test)]
mod tests {
    use super::{
        CustomConfigAngleCurrentGainField, CustomConfigAngleField,
        CustomConfigAngularVelocityField, CustomConfigDurationField,
        CustomConfigElectricalSpeedField, CustomConfigEnumField, CustomConfigFlagField,
        CustomConfigFrequencyField, CustomConfigImage, CustomConfigIntegralCurrentGainField,
        CustomConfigMahonyPitchGainField, CustomConfigMahonyRollGainField,
        CustomConfigMotorCurrentField, CustomConfigPidScaleField, CustomConfigRateCurrentGainField,
        CustomConfigRatioField, CustomConfigResetField, CustomConfigSampleRateField,
        CustomConfigScaledVoltageField, CustomConfigSecondsField, CustomConfigVoltageField,
        CustomConfigWireByteField, WireByte,
    };
    use crate::{ElectricalSpeed, Frequency, Rpm, SampleRate, VescSeconds, Voltage};

    const SIGNATURE: [u8; 4] = [0x90, 0xb7, 0xa9, 0xba];

    #[test]
    fn wire_byte_field_round_trips_without_erasing_its_type() {
        let field = crate::generated_custom_config_field!(
            CustomConfigWireByteField,
            len: 1,
            offset: 0
        );
        let mut image = CustomConfigImage::new([7]);

        assert_eq!(field.read(&image), Some(WireByte::new(7)));
        assert_eq!(
            field.write(&mut image.editor(), WireByte::new(42)),
            Some(())
        );
        assert_eq!(field.read(&image), Some(WireByte::new(42)));
    }

    #[test]
    fn custom_config_image_rejects_wrong_length_or_signature() {
        assert!(CustomConfigImage::<6>::from_serialized(&SIGNATURE, SIGNATURE).is_none());

        let mut bytes = [0x90, 0xb7, 0xa9, 0xba, 0x00, 0x01];
        assert!(CustomConfigImage::<6>::from_serialized(&bytes, SIGNATURE).is_some());

        bytes[0] = 0;
        assert!(CustomConfigImage::<6>::from_serialized(&bytes, SIGNATURE).is_none());
    }

    #[test]
    fn sample_rate_field_decodes_big_endian_hertz() {
        let field = crate::generated_custom_config_field!(
            CustomConfigSampleRateField,
            len: 2,
            offset: 0
        );
        let mut image = CustomConfigImage::new([0x00, 0xc8]);

        assert_eq!(field.read(&image), Some(SampleRate::from_hertz(200.0)));
        assert_eq!(
            field.write(&mut image.editor(), SampleRate::from_hertz(500.0)),
            Some(()),
        );
        assert_eq!(image.as_bytes(), &[0x01, 0xf4]);
    }

    #[test]
    fn scaled_frequency_field_round_trips_hertz() {
        let field = crate::generated_custom_config_field!(
            CustomConfigFrequencyField,
            len: 2,
            offset: 0,
            scale: 100.0
        );
        let mut image = CustomConfigImage::new([0x01, 0xf4]);

        assert_eq!(field.read(&image), Some(Frequency::from_hertz(5.0)));
        assert_eq!(
            field.write(&mut image.editor(), Frequency::from_hertz(7.5)),
            Some(()),
        );
        assert_eq!(image.as_bytes(), &[0x02, 0xee]);
    }

    #[test]
    fn electrical_speed_field_decodes_big_endian_erpm() {
        let field = crate::generated_custom_config_field!(
            CustomConfigElectricalSpeedField,
            len: 2,
            offset: 0
        );
        let mut image = CustomConfigImage::new([0x30, 0x39]);

        assert_eq!(
            field.read(&image),
            Some(ElectricalSpeed::new(Rpm::from_revolutions_per_minute(
                12_345.0,
            ))),
        );
        let speed = ElectricalSpeed::new(Rpm::from_revolutions_per_minute(5432.0));
        assert_eq!(field.write(&mut image.editor(), speed), Some(()));
        assert_eq!(field.read(&image), Some(speed));
    }

    #[test]
    fn voltage_field_decodes_big_endian_millivolts() {
        let field = crate::generated_custom_config_field!(
            CustomConfigVoltageField,
            len: 2,
            offset: 0
        );
        let mut image = CustomConfigImage::new([0x0c, 0xe4]);

        assert_eq!(field.read(&image), Some(Voltage::from_volts(3.3)));
        assert_eq!(
            field.write(&mut image.editor(), Voltage::from_volts(4.2)),
            Some(()),
        );
        assert_eq!(field.read(&image), Some(Voltage::from_volts(4.2)));
    }

    #[test]
    fn duration_field_decodes_big_endian_milliseconds() {
        let field = crate::generated_custom_config_field!(
            CustomConfigDurationField,
            len: 2,
            offset: 0
        );
        let mut image = CustomConfigImage::new([0x04, 0xe2]);

        assert_eq!(field.read(&image), Some(VescSeconds::from_seconds(1.25)));
        assert_eq!(
            field.write(&mut image.editor(), VescSeconds::from_seconds(2.5)),
            Some(()),
        );
        assert_eq!(field.read(&image), Some(VescSeconds::from_seconds(2.5)));
    }

    #[test]
    fn flag_field_reads_and_writes_boolean_values() {
        let mut image = CustomConfigImage::new([0, 1]);
        let flag = crate::generated_custom_config_field!(
            CustomConfigFlagField,
            len: 2,
            offset: 1
        );

        assert_eq!(flag.read(&image), Some(true));
        assert_eq!(flag.write(&mut image.editor(), false), Some(()));
        assert_eq!(flag.read(&image), Some(false));
    }

    #[test]
    fn scaled_fields_decode_and_encode_semantic_values() {
        macro_rules! field {
            ($field:ty) => {
                crate::generated_custom_config_field!(
                    $field,
                    len: 2,
                    offset: 0,
                    scale: 10.0
                )
            };
        }

        let mut image = CustomConfigImage::new([0x00, 0x7b]);

        assert_eq!(
            field!(CustomConfigMotorCurrentField).read(&image),
            Some(crate::MotorCurrent::new(crate::Current::from_amps(12.3)))
        );
        assert_eq!(
            field!(CustomConfigAngleField).read(&image),
            Some(crate::AngleDegrees::from_degrees(12.3))
        );
        assert_eq!(
            field!(CustomConfigAngularVelocityField).read(&image),
            Some(crate::AngularVelocity::from_degrees_per_second(12.3))
        );
        assert_eq!(
            field!(CustomConfigSecondsField).read(&image),
            Some(crate::VescSeconds::from_seconds(12.3))
        );
        assert_eq!(
            field!(CustomConfigScaledVoltageField).read(&image),
            Some(crate::Voltage::from_volts(12.3))
        );
        assert_eq!(
            field!(CustomConfigMahonyPitchGainField).read(&image),
            Some(crate::MahonyPitchGain::new(12.3))
        );
        assert_eq!(
            field!(CustomConfigMahonyRollGainField).read(&image),
            Some(crate::MahonyRollGain::new(12.3))
        );
        assert_eq!(
            field!(CustomConfigAngleCurrentGainField).read(&image),
            Some(crate::AngleCurrentGain::new(12.3))
        );
        assert_eq!(
            field!(CustomConfigRateCurrentGainField).read(&image),
            Some(crate::RateCurrentGain::new(12.3))
        );
        assert_eq!(
            field!(CustomConfigIntegralCurrentGainField).read(&image),
            Some(crate::IntegralCurrentGain::new(12.3))
        );
        assert_eq!(
            field!(CustomConfigPidScaleField).read(&image),
            Some(crate::PidScale::new(12.3))
        );

        image = CustomConfigImage::new([0x00, 0x08]);
        assert_eq!(
            field!(CustomConfigRatioField).read(&image),
            Some(crate::Ratio::from_ratio_const(0.8))
        );
        assert_eq!(
            field!(CustomConfigRatioField)
                .write(&mut image.editor(), crate::Ratio::from_ratio_const(0.5),),
            Some(())
        );
        assert_eq!(image.as_bytes(), &[0x00, 0x05]);

        field!(CustomConfigAngleField)
            .write(&mut image.editor(), crate::AngleDegrees::from_degrees(-4.2))
            .expect("valid generated field");
        assert_eq!(image.as_bytes(), &[0xff, 0xd6]);
    }

    #[test]
    fn config_fields_reject_incomplete_or_unrepresentable_values() {
        let image = CustomConfigImage::new([0x12]);
        assert_eq!(image.be_u16_at(0), None);
        assert_eq!(image.be_u16_at(usize::MAX), None);

        let mut image = CustomConfigImage::new([0_u8; 2]);
        assert!(
            crate::generated_custom_config_field!(
                CustomConfigDurationField,
                len: 2,
                offset: 0
            )
            .write(&mut image.editor(), crate::VescSeconds::from_seconds(-1.0))
            .is_none()
        );
        assert!(
            crate::generated_custom_config_field!(CustomConfigVoltageField, len: 2, offset: 0)
                .write(
                    &mut image.editor(),
                    crate::units::Voltage::from_volts(f32::INFINITY)
                )
                .is_none()
        );
        assert!(
            crate::generated_custom_config_field!(
                CustomConfigSampleRateField,
                len: 2,
                offset: 0
            )
            .write(&mut image.editor(), crate::SampleRate::from_hertz(f32::NAN))
            .is_none()
        );
        assert!(
            crate::generated_custom_config_field!(
                CustomConfigAngleField,
                len: 2,
                offset: 0,
                scale: 10.0
            )
            .write(
                &mut image.editor(),
                crate::AngleDegrees::from_degrees(4_000.0)
            )
            .is_none()
        );
    }

    #[test]
    fn wire_byte_maps_directly_into_semantic_scaled_values() {
        let byte = super::WireByte::new(42);

        assert_eq!(
            byte.scaled(0.5, -1.0, crate::AngleDegrees::from_degrees),
            crate::AngleDegrees::from_degrees(20.0)
        );
        assert_eq!(
            byte.scaled_ratio(1.0, 2.0, -1.0, crate::AngleDegrees::from_degrees),
            crate::AngleDegrees::from_degrees(20.0)
        );
    }

    #[test]
    fn wire_byte_rejects_invalid_ratio_denominators() {
        for denominator in [0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            assert!(
                std::panic::catch_unwind(|| {
                    super::WireByte::new(42).scaled_ratio(
                        1.0,
                        denominator,
                        0.0,
                        crate::AngleDegrees::from_degrees,
                    )
                })
                .is_err()
            );
        }
    }

    #[test]
    fn enum_and_reset_fields_hide_generated_storage() {
        #[derive(Debug, PartialEq, Eq)]
        enum Mode {
            Known,
            Unknown(u8),
        }

        impl From<u8> for Mode {
            fn from(value: u8) -> Self {
                if value == 1 {
                    Self::Known
                } else {
                    Self::Unknown(value)
                }
            }
        }

        let mut image = CustomConfigImage::new([7, 0x12, 0x34]);
        assert_eq!(
            crate::generated_custom_config_field!(
                CustomConfigEnumField<Mode>,
                len: 3,
                offset: 0
            )
            .read(&image),
            Some(Mode::Unknown(7))
        );

        crate::generated_custom_config_field!(CustomConfigResetField, len: 3, offset: 1)
            .clear(&mut image.editor())
            .expect("valid generated field");
        assert_eq!(image.as_bytes(), &[7, 0, 0]);
    }

    #[test]
    fn gear_ratio_rejects_non_finite_values() {
        assert!(super::GearRatio::try_new(f32::INFINITY).is_err());
        assert!(super::GearRatio::try_new(f32::NEG_INFINITY).is_err());
        assert!(super::GearRatio::try_new(f32::NAN).is_err());
    }
}
