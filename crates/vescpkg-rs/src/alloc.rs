//! Explicit firmware allocation handles for VESC native packages.
//!
//! VESC native packages can allocate from firmware-provided allocator slots.
//! This module wraps those slots with explicit RAII handles; it is not Rust's
//! default allocator, and it does not make `alloc` collections available.
//!
//! Memory returned by firmware `malloc` is uninitialized. The handle frees via
//! firmware `free` on [`Drop`], and callers should use raw pointers until they
//! have explicitly initialized the allocation.
//!
//! The optional `alloc` feature exposes [`VescAllocator`] for package crates
//! that deliberately opt into Rust's `alloc` collections.

#[cfg(feature = "alloc")]
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::mem::{ManuallyDrop, size_of};
#[cfg(feature = "alloc")]
use core::ptr;
use core::ptr::NonNull;

#[cfg(feature = "alloc")]
const ALLOC_HEADER_BYTES: usize = size_of::<*mut c_void>();

/// Firmware-backed global allocator for package-local `alloc` collections.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, Default)]
pub struct VescAllocator;

#[cfg(feature = "alloc")]
unsafe impl GlobalAlloc for VescAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return NonNull::<u8>::dangling().as_ptr();
        }

        let Some(bytes) = layout
            .size()
            .checked_add(ALLOC_HEADER_BYTES)
            .and_then(|bytes| bytes.checked_add(layout.align().saturating_sub(1)))
        else {
            return ptr::null_mut();
        };
        let raw = unsafe { vescpkg_rs_sys::raw::vesc_malloc(bytes) }.cast::<u8>();
        let Some(raw) = NonNull::new(raw) else {
            return ptr::null_mut();
        };
        let start = raw.as_ptr().wrapping_add(ALLOC_HEADER_BYTES);
        let offset = start.align_offset(layout.align());
        if offset == usize::MAX {
            unsafe { vescpkg_rs_sys::raw::vesc_free(raw.as_ptr().cast()) };
            return ptr::null_mut();
        }
        let user = start.wrapping_add(offset);
        unsafe {
            ptr::copy_nonoverlapping(
                (&raw.as_ptr().cast::<c_void>() as *const *mut c_void).cast::<u8>(),
                user.sub(ALLOC_HEADER_BYTES),
                ALLOC_HEADER_BYTES,
            );
        }
        user
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }
        let mut raw = ptr::null_mut::<c_void>();
        unsafe {
            ptr::copy_nonoverlapping(
                ptr.sub(ALLOC_HEADER_BYTES),
                (&mut raw as *mut *mut c_void).cast::<u8>(),
                ALLOC_HEADER_BYTES,
            );
            vescpkg_rs_sys::raw::vesc_free(raw);
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            unsafe { ptr::write_bytes(ptr, 0, layout.size()) };
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            let Ok(layout) = Layout::from_size_align(new_size, layout.align()) else {
                return ptr::null_mut();
            };
            return unsafe { self.alloc(layout) };
        }
        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };
        let replacement = unsafe { self.alloc(new_layout) };
        if !replacement.is_null() {
            unsafe { ptr::copy_nonoverlapping(ptr, replacement, layout.size().min(new_size)) };
            unsafe { self.dealloc(ptr, layout) };
        }
        replacement
    }
}

/// Firmware allocation failures reported by [`FirmwareAllocator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    /// Zero-byte allocations are rejected.
    ZeroLength,
    /// Zero-sized element allocations are rejected.
    ZeroSizedType,
    /// Element size multiplied by count overflowed `usize`.
    SizeOverflow,
    /// Firmware allocator returned null for the requested byte count.
    OutOfMemory {
        /// Requested byte count.
        bytes: usize,
    },
}

/// Firmware allocation and free calls used by the SDK allocator wrapper.
pub trait AllocBindings {
    /// # Safety
    ///
    /// The returned pointer, if non-null, must be freed by [`AllocBindings::free`]
    /// exactly once.
    unsafe fn malloc(&self, bytes: usize) -> *mut c_void;

    /// # Safety
    ///
    /// `ptr` must be null or a pointer returned by this allocator that has not
    /// already been freed.
    unsafe fn free(&self, ptr: *mut c_void);
}

#[cfg(not(test))]
impl AllocBindings for crate::RealBindings {
    unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
        unsafe { vescpkg_rs_sys::raw::vesc_malloc(bytes) }
    }

    unsafe fn free(&self, ptr: *mut c_void) {
        unsafe { vescpkg_rs_sys::raw::vesc_free(ptr) }
    }
}

/// Explicit firmware allocator backed by an [`AllocBindings`] implementation.
#[derive(Debug, Clone, Copy)]
pub struct FirmwareAllocator<'a, B> {
    bindings: &'a B,
}

impl<'a, B: AllocBindings> FirmwareAllocator<'a, B> {
    /// Create an allocator wrapper around firmware allocation bindings.
    pub const fn new(bindings: &'a B) -> Self {
        Self { bindings }
    }

    /// Allocate `len` uninitialized bytes from the firmware allocator.
    pub fn allocate_bytes(&self, len: usize) -> Result<FirmwareAllocation<'a, u8, B>, AllocError> {
        self.allocate_for::<u8>(len)
    }

    /// Allocate space for `count` uninitialized values of `T`.
    pub fn allocate_for<T>(&self, count: usize) -> Result<FirmwareAllocation<'a, T, B>, AllocError>
    where
        T: Sized,
    {
        let elem_size = size_of::<T>();
        if elem_size == 0 {
            return Err(AllocError::ZeroSizedType);
        }
        if count == 0 {
            return Err(AllocError::ZeroLength);
        }

        let bytes = elem_size
            .checked_mul(count)
            .ok_or(AllocError::SizeOverflow)?;
        let ptr = unsafe { self.bindings.malloc(bytes) };
        let ptr = NonNull::new(ptr.cast::<T>()).ok_or(AllocError::OutOfMemory { bytes })?;

        Ok(FirmwareAllocation {
            ptr,
            len: count,
            bindings: self.bindings,
        })
    }
}

/// Owned firmware allocation pointer freed on drop.

#[derive(Debug)]
pub struct FirmwareAllocation<'a, T, B: AllocBindings> {
    ptr: NonNull<T>,
    len: usize,
    bindings: &'a B,
}

impl<T, B: AllocBindings> FirmwareAllocation<'_, T, B> {
    /// Return the allocation pointer as const.
    pub const fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Return the allocation pointer as mutable.
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Number of `T` elements requested for this allocation.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether this allocation has zero elements.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the owned non-null firmware pointer.
    pub const fn as_non_null(&self) -> NonNull<T> {
        self.ptr
    }

    /// Transfer ownership of the firmware pointer out of this RAII handle.
    ///
    /// The caller becomes responsible for freeing the pointer exactly once with
    /// the same firmware allocator.
    pub fn into_raw(self) -> NonNull<T> {
        let allocation = ManuallyDrop::new(self);
        allocation.ptr
    }
}

impl<'a, T, B: AllocBindings> FirmwareAllocation<'a, T, B> {
    /// Build an owned firmware allocation handle from raw parts.
    ///
    /// # Safety
    ///
    /// `ptr` must have been returned by `bindings.malloc`, represent at least
    /// `len * size_of::<T>()` bytes, and be uniquely owned by this handle.
    pub unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize, bindings: &'a B) -> Self {
        Self { ptr, len, bindings }
    }
}

impl<T, B: AllocBindings> Drop for FirmwareAllocation<'_, T, B> {
    fn drop(&mut self) {
        unsafe { self.bindings.free(self.ptr.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{AllocBindings, AllocError, FirmwareAllocation, FirmwareAllocator};
    use core::cell::Cell;
    use core::ffi::c_void;
    use core::ptr::NonNull;
    use std::vec;

    #[derive(Debug)]
    struct FakeAllocBindings {
        malloc_calls: Cell<usize>,
        free_calls: Cell<usize>,
        last_requested_len: Cell<usize>,
        next_ptr: Cell<*mut c_void>,
        last_freed: Cell<usize>,
    }

    impl FakeAllocBindings {
        fn new(ptr: *mut c_void) -> Self {
            Self {
                malloc_calls: Cell::new(0),
                free_calls: Cell::new(0),
                last_requested_len: Cell::new(0),
                next_ptr: Cell::new(ptr),
                last_freed: Cell::new(0),
            }
        }

        fn failing() -> Self {
            Self::new(core::ptr::null_mut())
        }
    }

    impl AllocBindings for FakeAllocBindings {
        unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
            self.malloc_calls.set(self.malloc_calls.get() + 1);
            self.last_requested_len.set(bytes);
            self.next_ptr.get()
        }

        unsafe fn free(&self, ptr: *mut c_void) {
            self.free_calls.set(self.free_calls.get() + 1);
            self.last_freed.set(ptr as usize);
        }
    }

    #[test]
    fn allocate_bytes_calls_firmware_malloc_with_requested_len() {
        let mut backing = vec![0_u8; 8];
        let bindings = FakeAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&bindings);

        let allocation = allocator.allocate_bytes(8).expect("allocation");

        assert_eq!(bindings.malloc_calls.get(), 1);
        assert_eq!(bindings.last_requested_len.get(), 8);
        assert_eq!(allocation.len(), 8);
        assert!(!allocation.is_empty());
        assert_eq!(allocation.as_ptr(), backing.as_ptr());
        assert_eq!(allocation.as_non_null().as_ptr(), backing.as_mut_ptr());
    }

    #[test]
    fn allocation_drop_calls_firmware_free_once() {
        let mut backing = vec![0_u8; 4];
        let ptr = backing.as_mut_ptr();
        let bindings = FakeAllocBindings::new(ptr.cast());
        let allocator = FirmwareAllocator::new(&bindings);

        {
            let mut allocation = allocator.allocate_bytes(4).expect("allocation");
            assert_eq!(allocation.as_mut_ptr(), ptr);
        }

        assert_eq!(bindings.free_calls.get(), 1);
        assert_eq!(bindings.last_freed.get(), ptr as usize);
    }

    #[test]

    fn malloc_null_maps_to_out_of_memory() {
        let bindings = FakeAllocBindings::failing();
        let allocator = FirmwareAllocator::new(&bindings);

        let error = allocator.allocate_bytes(7).unwrap_err();

        assert_eq!(error, AllocError::OutOfMemory { bytes: 7 });
    }

    #[test]
    fn allocate_for_uses_checked_type_size_times_count() {
        let mut backing = vec![0_u32; 3];
        let bindings = FakeAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&bindings);

        let allocation = allocator.allocate_for::<u32>(3).expect("allocation");

        assert_eq!(bindings.last_requested_len.get(), 12);
        assert_eq!(allocation.len(), 3);
    }

    #[test]

    fn allocate_for_rejects_size_overflow() {
        let mut backing = vec![0_u8; 1];
        let bindings = FakeAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&bindings);

        let error = allocator.allocate_for::<u16>(usize::MAX).unwrap_err();

        assert_eq!(error, AllocError::SizeOverflow);
        assert_eq!(bindings.malloc_calls.get(), 0);
    }

    #[test]

    fn allocate_for_rejects_zero_sized_types() {
        let mut backing = vec![0_u8; 1];
        let bindings = FakeAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&bindings);

        assert_eq!(
            allocator.allocate_for::<()>(1).unwrap_err(),
            AllocError::ZeroSizedType
        );
        assert_eq!(
            allocator.allocate_bytes(0).unwrap_err(),
            AllocError::ZeroLength
        );
        assert_eq!(
            allocator.allocate_for::<u8>(0).unwrap_err(),
            AllocError::ZeroLength
        );
        assert_eq!(bindings.malloc_calls.get(), 0);
    }

    #[test]
    fn into_raw_prevents_drop_from_freeing() {
        let mut backing = vec![0_u8; 4];
        let ptr = backing.as_mut_ptr();
        let bindings = FakeAllocBindings::new(ptr.cast());
        let allocator = FirmwareAllocator::new(&bindings);

        let allocation = allocator.allocate_bytes(4).expect("allocation");
        let raw = allocation.into_raw();

        assert_eq!(raw.as_ptr(), ptr);
        assert_eq!(bindings.free_calls.get(), 0);
    }

    #[test]

    fn from_raw_parts_frees_on_drop() {
        let mut backing = vec![0_u16; 2];
        let ptr = NonNull::new(backing.as_mut_ptr()).expect("nonnull");
        let bindings = FakeAllocBindings::new(ptr.as_ptr().cast());

        {
            let allocation = unsafe { FirmwareAllocation::from_raw_parts(ptr, 2, &bindings) };
            assert_eq!(allocation.len(), 2);
        }

        assert_eq!(bindings.free_calls.get(), 1);
        assert_eq!(bindings.last_freed.get(), ptr.as_ptr() as usize);
    }
}
