//! Express interface-table shape and slot classification.

use super::types::{EXPRESS_C_IF_VERSION, EXPRESS_IF_SLOT_COUNT, ExpressAddress, ExpressWord};

/// Whether a pinned Express slot is a scalar word or a nullable function slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressSlotKind {
    /// Interface version or LispBM symbol constant stored inline.
    Scalar,
    /// Function pointer represented as a target word and allowed to be null on
    /// older firmware.
    Function,
}

/// Return the pinned kind of an Express slot, if it is in the v1 table.
pub const fn express_slot_kind(index: usize) -> Option<ExpressSlotKind> {
    match index {
        0 | 38..=42 => Some(ExpressSlotKind::Scalar),
        1..=37 | 43..=79 => Some(ExpressSlotKind::Function),
        _ => None,
    }
}

/// Borrowed Express table after its breaking layout version has been checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressTable<'a> {
    words: &'a [u32],
}

impl<'a> ExpressTable<'a> {
    /// Validate the first slot and borrow all words supplied by firmware.
    ///
    /// A shorter table is valid for an older Express firmware: appended
    /// function slots are absent rather than shifted or reinterpreted.
    pub const fn load(words: &'a [u32]) -> Result<Self, ExpressTableError> {
        if words.is_empty() {
            return Err(ExpressTableError::Empty);
        }
        if words[0] != EXPRESS_C_IF_VERSION {
            return Err(ExpressTableError::VersionMismatch {
                expected: EXPRESS_C_IF_VERSION,
                found: words[0],
            });
        }
        Ok(Self { words })
    }

    /// Return the validated interface version.
    pub const fn version(self) -> u32 {
        self.words[0]
    }

    /// Return the number of words exposed by this firmware table.
    pub const fn len(self) -> usize {
        self.words.len()
    }

    /// Return whether the firmware exposed no table words.
    pub const fn is_empty(self) -> bool {
        self.words.is_empty()
    }

    /// Return a raw word when the firmware exposes that appended slot.
    pub fn word(self, index: usize) -> Option<ExpressWord> {
        self.words.get(index).map(|word| ExpressWord::new(*word))
    }

    /// Return a non-null function address without converting it to a host
    /// pointer or making a call through an unverified ABI.
    pub fn function_address(self, index: usize) -> Option<ExpressAddress> {
        if !matches!(express_slot_kind(index), Some(ExpressSlotKind::Function)) {
            return None;
        }
        match self.words.get(index) {
            Some(0) | None => None,
            Some(word) => Some(ExpressAddress::new(*word)),
        }
    }

    /// Return whether all slots in the pinned v1 table are present.
    pub const fn is_complete(self) -> bool {
        self.words.len() >= EXPRESS_IF_SLOT_COUNT
    }
}

/// Error returned before any Express table slot is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressTableError {
    /// The firmware did not provide the version slot.
    Empty,
    /// The breaking interface version is not supported by this crate.
    VersionMismatch {
        /// Version expected by this table loader.
        expected: u32,
        /// Version found in the firmware table.
        found: u32,
    },
}
