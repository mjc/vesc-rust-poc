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
            .then(|| EepromWord::from_ne_bytes(word.to_ne_bytes()))
    }

    /// Store one word and report firmware success.
    pub fn write(self, address: CustomEepromAddress, word: EepromWord) -> bool {
        let mut word = u32::from_ne_bytes(word.to_ne_bytes());
        unsafe { crate::ffi::store_eeprom_word(&mut word, address.get()) }
    }
}
