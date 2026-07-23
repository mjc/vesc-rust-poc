//! Exclusive, checked access to the optional VESC UART peripheral.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::BaudRate;

static UART_OWNED: AtomicBool = AtomicBool::new(false);

/// Failure returned by a UART operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UartError {
    /// The firmware table does not expose the requested UART slot.
    Unavailable,
    /// Another package capability currently owns the global UART.
    Busy,
    /// Firmware rejected the configuration or write.
    Rejected,
    /// A write length cannot be represented by the C ABI.
    BufferTooLong,
    /// Firmware returned a value outside the byte/no-byte contract.
    InvalidRead,
}

/// Optional UART capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Uart;

/// Select the UART wiring mode used when opening a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartDuplexMode {
    /// Use separate transmit and receive lines.
    FullDuplex,
    /// Share one line for transmit and receive.
    HalfDuplex,
}

impl UartDuplexMode {
    /// Return the raw half-duplex flag expected by the firmware ABI.
    pub const fn is_half_duplex(self) -> bool {
        matches!(self, Self::HalfDuplex)
    }
}

/// Exclusive UART ownership lease.
pub struct UartLease {
    _private: (),
}

impl Uart {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Acquire the UART and configure its baud and duplex mode.
    pub fn open(&self, baud: BaudRate, mode: UartDuplexMode) -> Result<UartLease, UartError> {
        if UART_OWNED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(UartError::Busy);
        }
        let started = unsafe { crate::ffi::uart_start(baud.as_u32(), mode.is_half_duplex()) };
        match started {
            Some(true) => Ok(UartLease { _private: () }),
            Some(false) => {
                UART_OWNED.store(false, Ordering::Release);
                Err(UartError::Rejected)
            }
            None => {
                UART_OWNED.store(false, Ordering::Release);
                Err(UartError::Unavailable)
            }
        }
    }
}

impl UartLease {
    /// Write a bounded byte slice and return the number of bytes accepted.
    pub fn write(&self, data: &[u8]) -> Result<usize, UartError> {
        let size = u32::try_from(data.len()).map_err(|_| UartError::BufferTooLong)?;
        if data.is_empty() {
            return Ok(0);
        }
        match unsafe { crate::ffi::uart_write(data.as_ptr(), size) } {
            None => Err(UartError::Unavailable),
            Some(true) => Ok(data.len()),
            Some(false) => Err(UartError::Rejected),
        }
    }

    /// Read one byte, returning `None` when the UART has no byte ready.
    pub fn read(&self) -> Result<Option<u8>, UartError> {
        match unsafe { crate::ffi::uart_read() } {
            None => Err(UartError::Unavailable),
            Some(-1) => Ok(None),
            Some(value) => u8::try_from(value)
                .map(Some)
                .map_err(|_| UartError::InvalidRead),
        }
    }
}

impl Drop for UartLease {
    fn drop(&mut self) {
        UART_OWNED.store(false, Ordering::Release);
    }
}

impl crate::Firmware {
    /// Return the optional UART capability handle.
    pub fn uart(&self) -> Uart {
        Uart::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional UART capability handle.
    pub fn uart(&self) -> Uart {
        Uart::new()
    }
}
