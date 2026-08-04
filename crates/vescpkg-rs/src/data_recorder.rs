//! Validated descriptor for Refloat's optional recorder firmware buffer.

use core::ptr::NonNull;

const DATA_RECORDER_MAGIC: u32 = 0xcafe_1111;
const DATA_RECORDER_RAM_START: u32 = 0x1000_0000;
const DATA_RECORDER_RAM_END: u32 = 0x1000_f800;
const DATA_RECORDER_ALIGNMENT: u32 = 4;
#[cfg(target_arch = "arm")]
const DATA_RECORDER_DESCRIPTOR_ADDRESS: usize = 0x1000_fff4;

/// Storage-agnostic cursor for a fixed-capacity circular buffer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RingCursor {
    next: usize,
    len: usize,
}

fn advance_ring_index(index: usize, capacity: usize) -> usize {
    index
        .checked_add(1)
        .filter(|next| *next < capacity)
        .unwrap_or(0)
}

impl RingCursor {
    /// Forget all committed slots.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Return the slot for the next write, or `None` when capacity is zero.
    #[must_use]
    pub fn write_slot(self, capacity: usize) -> Option<usize> {
        (capacity > 0).then_some(self.next)
    }

    /// Commit a successful write and advance the cursor.
    pub fn commit_write(&mut self, capacity: usize) {
        self.next = advance_ring_index(self.next, capacity);
        self.len = self.len.saturating_add(1).min(capacity);
    }

    /// Return the number of committed slots available at this capacity.
    #[must_use]
    pub fn len(self, capacity: usize) -> usize {
        self.len.min(capacity)
    }

    /// Return whether no committed slots are available at this capacity.
    #[must_use]
    pub fn is_empty(self, capacity: usize) -> bool {
        self.len(capacity) == 0
    }

    /// Resolve an oldest-first logical index to a physical storage slot.
    #[must_use]
    pub fn slot_at(self, index: usize, capacity: usize) -> Option<usize> {
        let len = self.len(capacity);
        if index >= len || capacity == 0 {
            return None;
        }
        let oldest = self
            .next
            .checked_add(capacity)?
            .checked_sub(len)?
            .checked_rem(capacity)?;
        oldest.checked_add(index)?.checked_rem(capacity)
    }
}

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

/// Exclusive package handle to a validated recorder-firmware RAM region.
pub struct FirmwareDataRecorderBuffer {
    address: NonNull<u8>,
    len: usize,
}

impl core::fmt::Debug for FirmwareDataRecorderBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FirmwareDataRecorderBuffer")
            .field("address", &self.address)
            .field("len", &self.len)
            .finish()
    }
}

#[cfg(target_arch = "arm")]
// SAFETY: the special recorder firmware dedicates this static CCM region to
// one recorder package. Moving its unique handle between package threads does
// not change the region's lifetime or introduce another owner.
unsafe impl Send for FirmwareDataRecorderBuffer {}

impl FirmwareDataRecorderBuffer {
    /// Build a handle from a validated descriptor and its live native address.
    ///
    /// # Safety
    ///
    /// `address` must remain readable and writable for `descriptor.len()`
    /// bytes for the handle's lifetime, and no other code may access that
    /// region concurrently.
    #[cfg(any(test, target_arch = "arm"))]
    unsafe fn from_descriptor_and_address(
        descriptor: FirmwareDataRecorderDescriptor,
        address: *mut u8,
    ) -> Self {
        Self {
            // SAFETY: the caller guarantees a live non-null region matching
            // the already validated descriptor.
            address: unsafe { NonNull::new_unchecked(address) },
            len: usize::try_from(descriptor.len()).unwrap_or(0),
        }
    }

    #[cfg(target_arch = "arm")]
    pub(crate) fn discover() -> Option<Self> {
        let words = core::array::from_fn(|index| {
            // SAFETY: native ARM VESC packages use the fixed F407 VESC_IF
            // region. The special firmware places three aligned words at
            // `0x1000fff4`; standard F40x firmware leaves that mapped trailer
            // unused, so its non-magic contents fail closed.
            unsafe {
                (DATA_RECORDER_DESCRIPTOR_ADDRESS as *const u32)
                    .add(index)
                    .read_volatile()
            }
        });
        let descriptor = FirmwareDataRecorderDescriptor::try_from_words(words).ok()?;
        let address = usize::try_from(descriptor.start_address()).ok()? as *mut u8;
        // SAFETY: descriptor validation proves a non-null, aligned range wholly
        // inside the special firmware's reserved CCM region. The package-start
        // coordinator issues at most one handle for this lifecycle.
        Some(unsafe { Self::from_descriptor_and_address(descriptor, address) })
    }

    /// Return the firmware-reserved byte length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether the reserved region is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Copy bytes into the reserved region when the complete range is valid.
    pub fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };
        if end > self.len {
            return false;
        }
        // SAFETY: construction establishes exclusive access to `self.len`
        // bytes, and the checked range is wholly inside that region.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.address.as_ptr().add(offset),
                bytes.len(),
            );
        }
        true
    }

    /// Copy bytes from the reserved region when the complete range is valid.
    pub fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };
        if end > self.len {
            return false;
        }
        // SAFETY: construction establishes readable access to `self.len`
        // bytes, and the checked range is wholly inside that region.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.address.as_ptr().add(offset),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ring_cursor_is_empty() {
        assert_eq!(RingCursor::default().len(24), 0);
    }

    #[test]
    fn ring_cursor_preserves_newest_slots_across_capacities_and_wraps() {
        for capacity in 0_usize..=32 {
            for writes in 0..=capacity.saturating_mul(3).saturating_add(1) {
                let mut ring = RingCursor::default();
                for _ in 0..writes {
                    if ring.write_slot(capacity).is_some() {
                        ring.commit_write(capacity);
                    }
                }

                let len = writes.min(capacity);
                assert_eq!(ring.len(capacity), len);
                for index in 0..len {
                    let expected = writes.saturating_sub(len).saturating_add(index) % capacity;
                    assert_eq!(ring.slot_at(index, capacity), Some(expected));
                }
                assert_eq!(ring.slot_at(len, capacity), None);
            }
        }
    }

    #[test]
    fn recorder_buffer_reads_and_writes_only_checked_ranges() {
        let mut storage = [0_u8; 32];
        let descriptor =
            FirmwareDataRecorderDescriptor::try_from_words([0xcafe_1111, 0x1000_0000, 32])
                .expect("test descriptor");
        // SAFETY: `storage` remains alive and exclusively borrowed for the
        // handle's complete use below.
        let mut buffer = unsafe {
            FirmwareDataRecorderBuffer::from_descriptor_and_address(
                descriptor,
                storage.as_mut_ptr(),
            )
        };

        assert!(buffer.write(4, &[1, 2, 3, 4]));
        let mut copied = [0; 4];
        assert!(buffer.read(4, &mut copied));
        assert_eq!(copied, [1, 2, 3, 4]);
        assert!(!buffer.write(31, &[5, 6]));
        assert!(!buffer.read(usize::MAX, &mut copied));
        assert_eq!(&storage[4..8], &[1, 2, 3, 4]);
    }
}
