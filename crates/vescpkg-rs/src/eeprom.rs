//! Typed access to the package custom-EEPROM range.

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
        i32::try_from(index).ok().map(Self)
    }

    pub(crate) const fn get(self) -> i32 {
        self.0
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

/// Firmware-backed package custom-EEPROM capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct CustomEeprom;

impl CustomEeprom {
    /// Construct the zero-sized firmware capability.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Read one word, returning `None` when the address has no stored value.
    #[must_use]
    pub fn read(self, address: CustomEepromAddress) -> Option<EepromWord> {
        let mut word = 0_u32;
        unsafe { crate::ffi::read_eeprom_word(&mut word, address.get()) }
            .then(|| EepromWord::from_u32(word))
    }

    /// Store one word and report firmware success.
    pub fn write(self, address: CustomEepromAddress, word: EepromWord) -> bool {
        let mut word = word.to_u32();
        unsafe { crate::ffi::store_eeprom_word(&mut word, address.get()) }
    }

    /// Read a serialized byte image from consecutive custom-EEPROM words.
    ///
    /// Returns `false` when any required word is absent or its address cannot
    /// be represented. Bytes read before a failure remain in `bytes`.
    pub fn read_bytes(self, bytes: &mut [u8]) -> bool {
        bytes
            .chunks_mut(EepromWord::BYTE_LEN)
            .enumerate()
            .all(|(index, bytes)| {
                let Some(address) = CustomEepromAddress::from_index(index) else {
                    return false;
                };
                let Some(word) = self.read(address) else {
                    return false;
                };
                bytes.copy_from_slice(&word.to_ne_bytes()[..bytes.len()]);
                true
            })
    }

    /// Store a serialized byte image in consecutive custom-EEPROM words.
    ///
    /// A final partial word is padded with zeroes. Returns `false` after the
    /// first address or firmware write failure.
    pub fn write_bytes(self, bytes: &[u8]) -> bool {
        bytes
            .chunks(EepromWord::BYTE_LEN)
            .enumerate()
            .all(|(index, bytes)| {
                let Some(address) = CustomEepromAddress::from_index(index) else {
                    return false;
                };
                let mut word = [0; EepromWord::BYTE_LEN];
                word[..bytes.len()].copy_from_slice(bytes);
                self.write(address, EepromWord::from_ne_bytes(word))
            })
    }
}
