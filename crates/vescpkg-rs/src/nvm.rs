//! Typed access to firmware byte-addressed nonvolatile memory.

use core::num::NonZeroU32;

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

/// Discovered or package-provided byte capacity for firmware NVM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NvmCapacity(NonZeroU32);

impl NvmCapacity {
    /// Construct a non-zero NVM capacity in bytes.
    #[must_use]
    pub const fn new(bytes: u32) -> Option<Self> {
        match NonZeroU32::new(bytes) {
            Some(bytes) => Some(Self(bytes)),
            None => None,
        }
    }

    /// Convert a host-discovered capacity when it fits the firmware width.
    #[must_use]
    pub fn from_usize(bytes: usize) -> Option<Self> {
        u32::try_from(bytes).ok().and_then(Self::new)
    }

    /// Convert the capacity to its firmware byte representation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
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
    /// A bounded handle was asked to access bytes outside its capacity.
    OutOfBounds,
    /// Firmware exposed NVM but rejected the operation.
    FirmwareFailure,
}

/// Firmware-backed byte-addressed NVM capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nvm {
    capacity: Option<NvmCapacity>,
}

impl Nvm {
    /// Construct the zero-sized firmware capability.
    #[must_use]
    pub const fn new() -> Self {
        Self { capacity: None }
    }

    /// Construct a handle with a discovered or explicitly configured capacity.
    #[must_use]
    pub const fn with_capacity(capacity: NvmCapacity) -> Self {
        Self {
            capacity: Some(capacity),
        }
    }

    /// Return the capacity carried by this handle, when one is known.
    #[must_use]
    pub const fn capacity(self) -> Option<NvmCapacity> {
        self.capacity
    }

    /// Read bytes beginning at `offset` into an owned caller buffer.
    pub fn read(self, offset: NvmOffset, bytes: &mut [u8]) -> Result<(), NvmError> {
        let len = checked_len(offset, bytes.len(), self.capacity)?;
        operation_result(unsafe { crate::ffi::read_nvm(bytes.as_mut_ptr(), len, offset.get()) })
    }

    /// Write bytes beginning at `offset` from a caller buffer.
    pub fn write(self, offset: NvmOffset, bytes: &[u8]) -> Result<(), NvmError> {
        let len = checked_len(offset, bytes.len(), self.capacity)?;
        operation_result(unsafe {
            crate::ffi::write_nvm(bytes.as_ptr().cast_mut(), len, offset.get())
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

fn checked_len(
    offset: NvmOffset,
    len: usize,
    capacity: Option<NvmCapacity>,
) -> Result<u32, NvmError> {
    let len = u32::try_from(len).map_err(|_| NvmError::InvalidRange)?;
    let end = offset
        .get()
        .checked_add(len)
        .ok_or(NvmError::InvalidRange)?;
    if let Some(capacity) = capacity {
        if end > capacity.get() {
            return Err(NvmError::OutOfBounds);
        }
    }
    Ok(len)
}
