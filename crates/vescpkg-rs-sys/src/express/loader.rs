//! Express table loading and capability inspection.

use super::table::{ExpressSlot, ExpressTable, ExpressTableError};
use super::types::{EXPRESS_IF_SLOT_COUNT, ExpressAddress, ExpressTarget, ExpressWord};

/// Error returned while loading an Express interface table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressLoadError {
    /// The table failed its version check.
    Table(ExpressTableError),
}

/// Error returned when a callable Express slot is absent or null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressCallError {
    /// The named function slot that was unavailable.
    pub slot: ExpressSlot,
}

/// A validated Express v1 interface table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressInterface<'a> {
    table: ExpressTable<'a>,
}

impl<'a> ExpressInterface<'a> {
    /// Validate and borrow a table supplied by a host fixture or caller.
    pub const fn from_words(words: &'a [u32]) -> Result<Self, ExpressLoadError> {
        match ExpressTable::load(words) {
            Ok(table) => Ok(Self { table }),
            Err(error) => Err(ExpressLoadError::Table(error)),
        }
    }

    /// Borrow the validated table.
    pub const fn table(self) -> ExpressTable<'a> {
        self.table
    }

    /// Return whether a named slot is present in this firmware table.
    pub const fn has_slot(self, slot: ExpressSlot) -> bool {
        self.table.len() > slot.index()
    }

    /// Return a named scalar or raw slot word when present.
    pub fn word(self, slot: ExpressSlot) -> Option<ExpressWord> {
        self.table.word_at(slot)
    }

    /// Return a non-null named function address when present.
    pub fn function_address(self, slot: ExpressSlot) -> Option<ExpressAddress> {
        self.table.function_address_at(slot)
    }

    /// Resolve a raw Express function pointer with its caller-selected C ABI.
    ///
    /// # Safety
    ///
    /// `F` must be the exact `unsafe extern "C" fn` signature declared for
    /// `slot` in the pinned Express header. The returned function points into
    /// firmware at a target address and may only be called while that firmware
    /// table remains valid on the matching 32-bit target. This method performs
    /// no host-pointer, target, or signature validation.
    pub unsafe fn function<F: Copy>(self, slot: ExpressSlot) -> Result<F, ExpressCallError> {
        let address = self
            .function_address(slot)
            .ok_or(ExpressCallError { slot })?;
        let raw = address.get() as usize;
        Ok(unsafe { core::mem::transmute_copy(&raw) })
    }

    /// Construct a view over the fixed firmware table for an Express target.
    ///
    /// # Safety
    ///
    /// The caller must be executing on the matching Express target and the
    /// firmware must keep the target's interface table mapped and readable for
    /// the returned lifetime. The fixed address is never valid as a host
    /// pointer and must not be used on a different architecture or target.
    pub unsafe fn from_target(target: ExpressTarget) -> Result<Self, ExpressLoadError> {
        let words = unsafe {
            core::slice::from_raw_parts(
                target.interface_address() as *const u32,
                EXPRESS_IF_SLOT_COUNT,
            )
        };
        Self::from_words(words)
    }
}
