//! Bounded, allocation-free firmware logging.

use core::fmt;

/// Failure reported when a bounded log cannot be sent to firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogError {
    /// The fixed buffer overflowed; no partial message was sent.
    Truncated,
    /// The payload contains a NUL byte and cannot be sent as a C string.
    InteriorNul,
    /// The target firmware does not expose its logging slot.
    Unsupported,
}

/// A fixed-capacity log message that never allocates.
pub struct FirmwareLog<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    len: usize,
    truncated: bool,
    interior_nul: bool,
}

impl<const CAPACITY: usize> FirmwareLog<CAPACITY> {
    /// Create an empty log. One byte is reserved for the firmware C terminator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            len: 0,
            truncated: false,
            interior_nul: false,
        }
    }

    /// Return the bytes currently buffered, excluding the C terminator.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return whether input exceeded the fixed message capacity.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Append bytes, retaining the prefix that fits in the fixed buffer.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        let limit = CAPACITY.saturating_sub(1);
        for &byte in bytes {
            if byte == 0 {
                self.interior_nul = true;
                continue;
            }
            if self.len < limit {
                self.bytes[self.len] = byte;
                self.len += 1;
            } else {
                self.truncated = true;
            }
        }
        if CAPACITY != 0 {
            self.bytes[self.len.min(CAPACITY - 1)] = 0;
        }
    }

    /// Send the complete message through firmware's constant `%s` path.
    pub fn flush(&self) -> Result<usize, LogError> {
        if self.truncated {
            return Err(LogError::Truncated);
        }
        if self.interior_nul {
            return Err(LogError::InteriorNul);
        }
        let sent = unsafe { crate::ffi::printf_data(self.bytes.as_ptr().cast()) };
        if sent {
            Ok(self.len)
        } else {
            Err(LogError::Unsupported)
        }
    }
}

impl<const CAPACITY: usize> Default for FirmwareLog<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> fmt::Write for FirmwareLog<CAPACITY> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.write_bytes(value.as_bytes());
        Ok(())
    }
}
