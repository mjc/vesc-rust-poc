//! Explicitly owned memory from the Express firmware allocator.

use super::ExpressCallError;
use super::ExpressRuntime;

/// Error returned while creating an Express allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressAllocationError {
    /// The firmware allocator slot is absent.
    Unavailable(ExpressCallError),
    /// Zero-byte allocation was requested.
    ZeroSize,
    /// Firmware returned a null pointer.
    NullPointer,
}

/// An owned byte allocation obtained from Express firmware.
///
/// The allocation is never copied into a host allocator and is released with
/// the matching firmware `free` slot when dropped.
#[derive(Debug)]
pub struct ExpressAllocation<'a> {
    runtime: ExpressRuntime<'a>,
    pointer: *mut u8,
    len: usize,
}

impl<'a> ExpressAllocation<'a> {
    /// Allocate `len` bytes from the Express firmware allocator.
    pub fn new(runtime: ExpressRuntime<'a>, len: usize) -> Result<Self, ExpressAllocationError> {
        if len == 0 {
            return Err(ExpressAllocationError::ZeroSize);
        }
        let pointer = runtime
            .malloc(len)
            .map_err(ExpressAllocationError::Unavailable)?
            .cast::<u8>();
        if pointer.is_null() {
            return Err(ExpressAllocationError::NullPointer);
        }
        Ok(Self {
            runtime,
            pointer,
            len,
        })
    }

    /// Return the allocation length.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether the allocation length is zero.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrow the initialized allocation bytes.
    ///
    /// The caller must ensure firmware initialized the bytes before reading
    /// them; `malloc` itself does not promise zeroed memory.
    ///
    /// # Safety
    ///
    /// The bytes must have been initialized by the caller or firmware before
    /// they are read, and the allocation must remain exclusively owned by this
    /// handle for the returned borrow.
    pub unsafe fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.pointer, self.len) }
    }

    /// Borrow the allocation bytes for initialization or mutation.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.pointer, self.len) }
    }
}

impl Drop for ExpressAllocation<'_> {
    fn drop(&mut self) {
        let _ = self.runtime.free(self.pointer.cast());
    }
}
