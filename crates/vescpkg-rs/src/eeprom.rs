//! Typed access to the package custom-EEPROM range.
#![allow(
    clippy::missing_errors_doc,
    reason = "error variants document failures"
)]

/// Word address passed to the firmware custom-EEPROM interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CustomEepromAddress(i32);

impl CustomEepromAddress {
    /// Convert a word index to the signed address representation used by firmware.
    ///
    /// Returns `None` when `index` does not fit in an `i32`. Firmware-specific
    /// address limits are not validated here.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        EepromWordOffset::from_index(index).checked_address().ok()
    }

    pub(crate) const fn get(self) -> i32 {
        self.0
    }
}

/// Zero-based word offset within the package custom-EEPROM range.
///
/// This remains distinct from [`CustomEepromAddress`], which is the signed
/// representation passed across the firmware ABI.  Keeping the offset in
/// `usize` lets callers describe an image layout without performing primitive
/// address conversions themselves; conversion to the ABI is checked when an
/// operation is dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EepromWordOffset(usize);

impl EepromWordOffset {
    /// Construct a zero-based word offset.
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn checked_address(self) -> Result<CustomEepromAddress, EepromError> {
        i32::try_from(self.0)
            .map(CustomEepromAddress)
            .map_err(|_| EepromError::AddressOverflow)
    }

    fn checked_add(self, words: usize) -> Result<Self, EepromError> {
        self.0
            .checked_add(words)
            .map(Self)
            .ok_or(EepromError::AddressOverflow)
    }
}

/// One EEPROM word preserving the serialized byte order in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EepromWord([u8; 4]);

impl EepromWord {
    /// Number of serialized bytes in one EEPROM word.
    pub const BYTE_LEN: usize = 4;

    /// Construct one word from a native-endian unsigned integer.
    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        Self::from_ne_bytes(value.to_ne_bytes())
    }

    /// Recover the native-endian unsigned integer represented by this word.
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        u32::from_ne_bytes(self.0)
    }

    /// Construct one word from a native-endian signed integer.
    #[must_use]
    pub const fn from_i32(value: i32) -> Self {
        Self::from_ne_bytes(value.to_ne_bytes())
    }

    /// Recover the native-endian signed integer represented by this word.
    #[must_use]
    pub const fn to_i32(self) -> i32 {
        i32::from_ne_bytes(self.0)
    }

    /// Construct one word from a native-endian `f32` bit pattern.
    #[must_use]
    pub const fn from_f32(value: f32) -> Self {
        Self::from_ne_bytes(value.to_ne_bytes())
    }

    /// Recover the native-endian `f32` bit pattern represented by this word.
    #[must_use]
    pub const fn to_f32(self) -> f32 {
        f32::from_ne_bytes(self.0)
    }

    /// Construct one word from four serialized bytes.
    #[must_use]
    pub const fn from_ne_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Recover the four serialized bytes.
    #[must_use]
    pub const fn to_ne_bytes(self) -> [u8; 4] {
        self.0
    }
}

/// Failure returned by a custom-EEPROM word or image operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EepromError {
    /// A required word was not available to read.
    Missing,
    /// A signature-committed image did not contain a complete first word.
    ImageTooShort,
    /// The requested consecutive word address cannot be represented by the ABI.
    AddressOverflow,
    /// Firmware rejected a word write.
    FirmwareRejected,
}

impl core::fmt::Display for EepromError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "custom EEPROM word is unavailable",
            Self::ImageTooShort => "custom EEPROM image has no complete signature word",
            Self::AddressOverflow => "custom EEPROM address range overflows the ABI",
            Self::FirmwareRejected => "firmware rejected the custom EEPROM write",
        })
    }
}

impl core::error::Error for EepromError {}

/// Firmware-backed package custom-EEPROM capability.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CustomEeprom;

impl CustomEeprom {
    /// Construct the zero-sized firmware capability.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Read one word, returning `None` when the address has no stored value.
    #[must_use]
    pub fn read(
        self,
        _effects: &crate::FirmwareEffects,
        address: CustomEepromAddress,
    ) -> Option<EepromWord> {
        let mut word = 0_u32;
        call_vesc_ffi!(read_eeprom_word(&raw mut word, address.get()))
            .then(|| EepromWord::from_u32(word))
    }

    /// Read one word at a typed word offset.
    #[must_use]
    pub fn read_at(
        self,
        effects: &crate::FirmwareEffects,
        offset: EepromWordOffset,
    ) -> Option<EepromWord> {
        offset
            .checked_address()
            .ok()
            .and_then(|address| self.read(effects, address))
    }

    /// Store one word, reporting a firmware rejection explicitly.
    pub fn write(
        self,
        _effects: &crate::FirmwareEffects,
        address: CustomEepromAddress,
        word: EepromWord,
    ) -> Result<(), EepromError> {
        let mut word = word.to_u32();
        call_vesc_ffi!(store_eeprom_word(&raw mut word, address.get()))
            .then_some(())
            .ok_or(EepromError::FirmwareRejected)
    }

    /// Store one word at a typed word offset.
    pub fn write_at(
        self,
        effects: &crate::FirmwareEffects,
        offset: EepromWordOffset,
        word: EepromWord,
    ) -> Result<(), EepromError> {
        offset
            .checked_address()
            .and_then(|address| self.write(effects, address, word))
    }

    /// Read a serialized byte image from consecutive custom-EEPROM words.
    ///
    /// Returns a typed error when any required word is absent or its address
    /// cannot be represented. Bytes read before a failure remain in `bytes`.
    pub fn read_bytes(
        self,
        effects: &crate::FirmwareEffects,
        bytes: &mut [u8],
    ) -> Result<(), EepromError> {
        let address = CustomEepromAddress::from_index(0).ok_or(EepromError::AddressOverflow)?;
        self.read_bytes_at(effects, address, bytes)
    }

    /// Read a serialized byte image from consecutive custom-EEPROM words at
    /// an explicit starting address.
    ///
    /// Bytes read before a missing word or address failure remain in `bytes`.
    pub fn read_bytes_at(
        self,
        effects: &crate::FirmwareEffects,
        start: CustomEepromAddress,
        bytes: &mut [u8],
    ) -> Result<(), EepromError> {
        let start = usize::try_from(start.get()).map_err(|_| EepromError::AddressOverflow)?;
        self.read_bytes_at_offset(effects, EepromWordOffset::from_index(start), bytes)
    }

    /// Read a serialized byte image at a typed word offset.
    pub fn read_bytes_at_offset(
        self,
        effects: &crate::FirmwareEffects,
        start: EepromWordOffset,
        bytes: &mut [u8],
    ) -> Result<(), EepromError> {
        bytes
            .chunks_mut(EepromWord::BYTE_LEN)
            .enumerate()
            .try_for_each(|(index, bytes)| {
                let offset = start.checked_add(index)?;
                let word = offset
                    .checked_address()
                    .and_then(|address| self.read(effects, address).ok_or(EepromError::Missing))?;
                bytes.copy_from_slice(&word.to_ne_bytes()[..bytes.len()]);
                Ok(())
            })
    }

    /// Read an owned fixed-size byte image from offset zero.
    pub fn read_image<const N: usize>(
        self,
        effects: &crate::FirmwareEffects,
    ) -> Result<[u8; N], EepromError> {
        self.read_image_at(effects, EepromWordOffset::from_index(0))
    }

    /// Read an owned fixed-size byte image at a typed word offset.
    pub fn read_image_at<const N: usize>(
        self,
        effects: &crate::FirmwareEffects,
        start: EepromWordOffset,
    ) -> Result<[u8; N], EepromError> {
        let mut image = [0; N];
        self.read_bytes_at_offset(effects, start, &mut image)?;
        Ok(image)
    }

    /// Store a serialized byte image in consecutive custom-EEPROM words.
    ///
    /// A final partial word is padded with zeroes. Returns a typed error after
    /// the first address or firmware write failure.
    pub fn write_bytes(
        self,
        effects: &crate::FirmwareEffects,
        bytes: &[u8],
    ) -> Result<(), EepromError> {
        let address = CustomEepromAddress::from_index(0).ok_or(EepromError::AddressOverflow)?;
        self.write_bytes_at(effects, address, bytes)
    }

    /// Store a serialized byte image in consecutive words at an explicit
    /// starting address.
    ///
    /// A final partial word is padded with zeroes. Returns a typed error after
    /// the first address or firmware write failure.
    pub fn write_bytes_at(
        self,
        effects: &crate::FirmwareEffects,
        start: CustomEepromAddress,
        bytes: &[u8],
    ) -> Result<(), EepromError> {
        let start = usize::try_from(start.get()).map_err(|_| EepromError::AddressOverflow)?;
        self.write_bytes_at_offset(effects, EepromWordOffset::from_index(start), bytes)
    }

    /// Store a serialized byte image at a typed word offset.
    pub fn write_bytes_at_offset(
        self,
        effects: &crate::FirmwareEffects,
        start: EepromWordOffset,
        bytes: &[u8],
    ) -> Result<(), EepromError> {
        bytes
            .chunks(EepromWord::BYTE_LEN)
            .enumerate()
            .try_for_each(|(index, bytes)| {
                let offset = start.checked_add(index)?;
                let mut word = [0; EepromWord::BYTE_LEN];
                let word_bytes = word
                    .get_mut(..bytes.len())
                    .ok_or(EepromError::AddressOverflow)?;
                word_bytes.copy_from_slice(bytes);
                self.write_at(effects, offset, EepromWord::from_ne_bytes(word))
            })
    }

    /// Store an owned fixed-size byte image at offset zero.
    pub fn write_image<const N: usize>(
        self,
        effects: &crate::FirmwareEffects,
        image: &[u8; N],
    ) -> Result<(), EepromError> {
        self.write_image_at(effects, EepromWordOffset::from_index(0), image)
    }

    /// Store an image whose first word is its validity signature.
    ///
    /// The signature is cleared before writing the payload and restored only
    /// after every payload word succeeds, so interrupted writes remain invalid.
    pub fn write_signature_committed_image<const N: usize>(
        self,
        effects: &crate::FirmwareEffects,
        image: &[u8; N],
    ) -> Result<(), EepromError> {
        let signature = <[u8; EepromWord::BYTE_LEN]>::try_from(
            image
                .get(..EepromWord::BYTE_LEN)
                .ok_or(EepromError::ImageTooShort)?,
        )
        .map_err(|_| EepromError::ImageTooShort)?;
        let payload = image
            .get(EepromWord::BYTE_LEN..)
            .ok_or(EepromError::ImageTooShort)?;
        let signature_offset = EepromWordOffset::from_index(0);
        self.write_at(effects, signature_offset, EepromWord::from_u32(0))?;
        self.write_bytes_at_offset(effects, EepromWordOffset::from_index(1), payload)?;
        self.write_at(
            effects,
            signature_offset,
            EepromWord::from_ne_bytes(signature),
        )
    }

    /// Store an owned fixed-size byte image at a typed word offset.
    pub fn write_image_at<const N: usize>(
        self,
        effects: &crate::FirmwareEffects,
        start: EepromWordOffset,
        image: &[u8; N],
    ) -> Result<(), EepromError> {
        self.write_bytes_at_offset(effects, start, image)
    }
}
