//! Ownership-scoped flattened LispBM values for Express.

use super::functions::{
    FB, FCons, FFloat, FI, FI32, FI64, FLbmArray, FSym, FU32, FU64, Free, LbmFinishFlatten,
    LbmStartFlatten,
};
use super::{ExpressCallError, ExpressInterface, ExpressSlot};

/// Error returned while constructing or appending to a flat LispBM value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressFlatValueError {
    /// The firmware did not expose the requested flat-value slot.
    Unavailable(ExpressCallError),
    /// Firmware rejected the operation or the input could not fit its ABI.
    Rejected,
}

/// A flat-value message error returned when unblocking a LispBM context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressLispMessageError {
    /// The firmware did not expose the unblock slot.
    Unavailable(ExpressCallError),
    /// Firmware rejected the message or flattening failed.
    Rejected,
}

/// Firmware-owned flattened LispBM value under construction.
///
/// The buffer is released through the Express `free` slot if the value is
/// dropped before ownership is transferred to LispBM.
#[must_use]
pub struct ExpressFlatValue<'a> {
    interface: ExpressInterface<'a>,
    pub(crate) raw: super::types::ExpressFlatValueRaw,
    finished: bool,
}

impl<'a> ExpressFlatValue<'a> {
    pub(crate) fn start(
        interface: ExpressInterface<'a>,
        buffer_size: usize,
    ) -> Result<Self, ExpressFlatValueError> {
        let start: LbmStartFlatten = unsafe { interface.function(ExpressSlot::LbmStartFlatten) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        let mut raw = super::types::ExpressFlatValueRaw::empty();
        if unsafe { start(&mut raw, buffer_size) } {
            Ok(Self {
                interface,
                raw,
                finished: false,
            })
        } else {
            Err(ExpressFlatValueError::Rejected)
        }
    }

    /// Return whether firmware has finished this flat value.
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// Append a cons marker.
    pub fn push_cons(&mut self) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FCons = unsafe { self.interface.function(ExpressSlot::FCons) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw) })
    }

    /// Append a symbol identifier.
    pub fn push_symbol(&mut self, symbol: u32) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FSym = unsafe { self.interface.function(ExpressSlot::FSym) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, symbol) })
    }

    /// Append an immediate LispBM integer.
    pub fn push_i(&mut self, value: i32) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FI = unsafe { self.interface.function(ExpressSlot::FI) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append a byte value.
    pub fn push_byte(&mut self, value: u8) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FB = unsafe { self.interface.function(ExpressSlot::FB) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append a signed 32-bit value.
    pub fn push_i32(&mut self, value: i32) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FI32 = unsafe { self.interface.function(ExpressSlot::FI32) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append an unsigned 32-bit value.
    pub fn push_u32(&mut self, value: u32) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FU32 = unsafe { self.interface.function(ExpressSlot::FU32) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append an `f32` value.
    pub fn push_float(&mut self, value: f32) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FFloat = unsafe { self.interface.function(ExpressSlot::FFloat) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append a signed 64-bit value.
    pub fn push_i64(&mut self, value: i64) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FI64 = unsafe { self.interface.function(ExpressSlot::FI64) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append an unsigned 64-bit value.
    pub fn push_u64(&mut self, value: u64) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let push: FU64 = unsafe { self.interface.function(ExpressSlot::FU64) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, value) })
    }

    /// Append a copied byte array.
    pub fn push_byte_array(&mut self, bytes: &[u8]) -> Result<bool, ExpressFlatValueError> {
        self.ensure_open()?;
        let count = u32::try_from(bytes.len()).map_err(|_| ExpressFlatValueError::Rejected)?;
        let push: FLbmArray = unsafe { self.interface.function(ExpressSlot::FLbmArray) }
            .map_err(ExpressFlatValueError::Unavailable)?;
        Ok(unsafe { push(&mut self.raw, count, bytes.as_ptr().cast_mut()) })
    }

    /// Finish this value before transferring it to LispBM.
    pub fn finish(&mut self) -> Result<bool, ExpressFlatValueError> {
        if self.finished {
            return Ok(true);
        }
        let finish: LbmFinishFlatten =
            unsafe { self.interface.function(ExpressSlot::LbmFinishFlatten) }
                .map_err(ExpressFlatValueError::Unavailable)?;
        let accepted = unsafe { finish(&mut self.raw) };
        self.finished = accepted;
        Ok(accepted)
    }

    pub(crate) fn relinquish(&mut self) {
        self.raw.buf = core::ptr::null_mut();
    }

    fn ensure_open(&self) -> Result<(), ExpressFlatValueError> {
        (!self.finished)
            .then_some(())
            .ok_or(ExpressFlatValueError::Rejected)
    }
}

impl Drop for ExpressFlatValue<'_> {
    fn drop(&mut self) {
        if self.raw.buf.is_null() {
            return;
        }
        let Ok(free) = (unsafe { self.interface.function::<Free>(ExpressSlot::Free) }) else {
            return;
        };
        unsafe { free(self.raw.buf.cast()) };
        self.raw.buf = core::ptr::null_mut();
    }
}
