//! Validated descriptor for Refloat's optional recorder firmware buffer.

const DATA_RECORDER_MAGIC: u32 = 0xcafe_1111;
const DATA_RECORDER_RAM_START: u32 = 0x1000_0000;
const DATA_RECORDER_RAM_END: u32 = 0x1000_f800;
const DATA_RECORDER_ALIGNMENT: u32 = 4;

/// A validated recorder-buffer descriptor from Refloat's special VESC 6.05 firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareDataRecorderDescriptor {
    start_address: u32,
    len: u32,
}

impl FirmwareDataRecorderDescriptor {
    /// Validate the descriptor words stored immediately after the VESC interface.
    ///
    /// # Errors
    ///
    /// Returns the specific descriptor invariant that failed.
    pub fn try_from_words(
        [magic, start_address, len]: [u32; 3],
    ) -> Result<Self, FirmwareDataRecorderDescriptorError> {
        if magic != DATA_RECORDER_MAGIC {
            return Err(FirmwareDataRecorderDescriptorError::BadMagic);
        }
        if start_address % DATA_RECORDER_ALIGNMENT != 0 || len % DATA_RECORDER_ALIGNMENT != 0 {
            return Err(FirmwareDataRecorderDescriptorError::Misaligned);
        }
        if len < DATA_RECORDER_ALIGNMENT {
            return Err(FirmwareDataRecorderDescriptorError::Undersized);
        }
        let end = start_address
            .checked_add(len)
            .ok_or(FirmwareDataRecorderDescriptorError::AddressOverflow)?;
        if start_address < DATA_RECORDER_RAM_START || end > DATA_RECORDER_RAM_END {
            return Err(FirmwareDataRecorderDescriptorError::OutsideRecorderRam);
        }
        Ok(Self { start_address, len })
    }

    /// Return the validated native start address.
    #[must_use]
    pub const fn start_address(self) -> u32 {
        self.start_address
    }

    /// Return the validated byte length.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Return whether the validated region is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Reason a recorder-firmware descriptor was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FirmwareDataRecorderDescriptorError {
    /// The firmware magic did not identify Refloat's recorder build.
    BadMagic,
    /// The buffer start or length did not preserve ARM word alignment.
    Misaligned,
    /// The buffer cannot hold even one aligned firmware word.
    Undersized,
    /// Adding the byte length overflowed the 32-bit firmware address.
    AddressOverflow,
    /// The buffer was not wholly inside the recorder firmware's reserved CCM region.
    OutsideRecorderRam,
}

impl core::fmt::Display for FirmwareDataRecorderDescriptorError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::BadMagic => "bad recorder firmware magic",
            Self::Misaligned => "misaligned recorder buffer",
            Self::Undersized => "undersized recorder buffer",
            Self::AddressOverflow => "recorder buffer address overflow",
            Self::OutsideRecorderRam => "recorder buffer outside reserved CCM RAM",
        })
    }
}

impl core::error::Error for FirmwareDataRecorderDescriptorError {}
