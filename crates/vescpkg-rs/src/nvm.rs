//! Typed access to firmware byte-addressed nonvolatile memory.

/// Byte offset passed to the firmware NVM interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NvmOffset(u32);

impl NvmOffset {
    /// Construct an NVM offset.
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }

    /// Convert a host byte offset when it fits the firmware representation.
    #[must_use]
    pub fn from_usize(offset: usize) -> Option<Self> {
        u32::try_from(offset).ok().map(Self)
    }

    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// Failure returned by a typed NVM operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NvmError {
    /// The firmware does not expose the optional NVM capability.
    Unsupported,
    /// The requested offset and byte length cannot be represented safely.
    InvalidRange,
    /// Firmware exposed NVM but rejected the operation.
    FirmwareFailure,
}

/// Firmware-backed byte-addressed NVM capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nvm;

impl Nvm {
    /// Construct the zero-sized firmware capability.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Read bytes beginning at `offset` into an owned caller buffer.
    pub fn read(self, offset: NvmOffset, bytes: &mut [u8]) -> Result<(), NvmError> {
        let len = checked_len(offset, bytes.len())?;
        operation_result(unsafe { crate::ffi::read_nvm(bytes.as_mut_ptr(), offset.get(), len) })
    }

    /// Write bytes beginning at `offset` from a caller buffer.
    pub fn write(self, offset: NvmOffset, bytes: &[u8]) -> Result<(), NvmError> {
        let len = checked_len(offset, bytes.len())?;
        operation_result(unsafe {
            crate::ffi::write_nvm(bytes.as_ptr().cast_mut(), offset.get(), len)
        })
    }

    /// Erase the complete firmware NVM region.
    pub fn wipe(self) -> Result<(), NvmError> {
        operation_result(unsafe { crate::ffi::wipe_nvm() })
    }
}

fn operation_result(result: Option<bool>) -> Result<(), NvmError> {
    match result {
        None => Err(NvmError::Unsupported),
        Some(true) => Ok(()),
        Some(false) => Err(NvmError::FirmwareFailure),
    }
}

fn checked_len(offset: NvmOffset, len: usize) -> Result<u32, NvmError> {
    let len = u32::try_from(len).map_err(|_| NvmError::InvalidRange)?;
    offset
        .get()
        .checked_add(len)
        .ok_or(NvmError::InvalidRange)?;
    Ok(len)
}
