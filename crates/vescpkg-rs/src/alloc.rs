//! Firmware allocator support for VESC native packages.
//!
//! With the `alloc` feature enabled, package crates may install
//! [`VescAllocator`] as their package-local `#[global_allocator]` and then use
//! Rust `alloc` collections such as `Vec`, `Box`, and `String`. The adapter
//! over-allocates and stores the original firmware pointer before the aligned
//! user pointer so Rust allocation layouts can request alignments larger than
//! the firmware `malloc` API exposes directly. Out-of-memory is reported by
//! returning null from `GlobalAlloc::alloc`; `alloc` collection methods that
//! panic or abort on allocation failure keep their normal behavior, while
//! `try_reserve` reports the failure to the package.

#[cfg(feature = "alloc")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(feature = "alloc")]
use core::ffi::c_void;
#[cfg(feature = "alloc")]
use core::mem::size_of;
#[cfg(feature = "alloc")]
use core::ptr;
#[cfg(feature = "alloc")]
use core::ptr::NonNull;

#[cfg(feature = "alloc")]
const HEADER_BYTES: usize = size_of::<*mut c_void>();
#[cfg(feature = "alloc")]
const HEADER_ALIGN: usize = core::mem::align_of::<*mut c_void>();

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy)]
struct AllocationHeader;

#[cfg(feature = "alloc")]
impl AllocationHeader {
    fn request_bytes(layout: Layout) -> Result<usize, AllocationSizeOverflow> {
        let align = effective_align(layout.align());
        layout
            .size()
            .checked_add(HEADER_BYTES)
            .and_then(|bytes| bytes.checked_add(align - 1))
            .ok_or(AllocationSizeOverflow)
    }

    unsafe fn write_before(user: NonNull<u8>, original: NonNull<u8>) {
        let original = original.as_ptr().cast::<c_void>();
        let header = user.as_ptr().wrapping_sub(HEADER_BYTES);
        unsafe {
            ptr::copy_nonoverlapping((&raw const original).cast::<u8>(), header, HEADER_BYTES)
        };
    }

    unsafe fn read_before(user: NonNull<u8>) -> *mut c_void {
        let mut original = ptr::null_mut::<c_void>();
        let header = user.as_ptr().wrapping_sub(HEADER_BYTES);
        unsafe { ptr::copy_nonoverlapping(header, (&raw mut original).cast::<u8>(), HEADER_BYTES) };
        original
    }
}

/// VESC firmware allocator adapter for package-local Rust `alloc` use.
///
/// Install this type with `#[global_allocator]` in a package crate that
/// intentionally enables `vescpkg-rs/alloc` and wants `alloc` collections to
/// consume firmware heap.
///
/// ```ignore
/// use vescpkg_rs::VescAllocator;
///
/// #[global_allocator]
/// static ALLOCATOR: VescAllocator = VescAllocator;
/// ```
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, Default)]
pub struct VescAllocator;

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AllocationSizeOverflow;

#[cfg(feature = "alloc")]
unsafe impl GlobalAlloc for VescAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return NonNull::<u8>::dangling().as_ptr().with_addr(layout.align());
        }

        match AllocationHeader::request_bytes(layout) {
            Ok(request) => {
                let raw = unsafe { crate::ffi::vesc_malloc(request) }.cast::<u8>();
                let Some(user) = aligned_user_ptr(raw, layout.align()) else {
                    if !raw.is_null() {
                        unsafe { crate::ffi::vesc_free(raw.cast()) };
                    }
                    return ptr::null_mut();
                };
                user.as_ptr()
            }
            Err(AllocationSizeOverflow) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }

        let user = unsafe { NonNull::new_unchecked(ptr) };
        let original = unsafe { AllocationHeader::read_before(user) };
        unsafe { crate::ffi::vesc_free(original) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            unsafe { zero_allocation_bytes(ptr, layout.size()) };
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size == 0 {
            unsafe { self.dealloc(ptr, layout) };
            return ptr::null_mut();
        }

        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };
        let new_ptr = unsafe { self.alloc(new_layout) };
        if !new_ptr.is_null() {
            let bytes_to_copy = layout.size().min(new_size);
            unsafe { copy_allocation_bytes(ptr, new_ptr, bytes_to_copy) };
            unsafe { self.dealloc(ptr, layout) };
        }
        new_ptr
    }
}

#[cfg(feature = "alloc")]
unsafe fn zero_allocation_bytes(dst: *mut u8, len: usize) {
    unsafe { ptr::write_bytes(dst, 0, len) };
}

#[cfg(feature = "alloc")]
unsafe fn copy_allocation_bytes(src: *const u8, dst: *mut u8, len: usize) {
    unsafe { ptr::copy_nonoverlapping(src, dst, len) };
}

#[cfg(feature = "alloc")]
fn aligned_user_ptr(raw: *mut u8, align: usize) -> Option<NonNull<u8>> {
    let raw = NonNull::new(raw)?;
    let align = effective_align(align);
    let start = raw.as_ptr().wrapping_add(HEADER_BYTES);
    let offset = start.align_offset(align);
    if offset == usize::MAX {
        return None;
    }
    let user = NonNull::new(start.wrapping_add(offset))?;

    unsafe {
        AllocationHeader::write_before(user, raw);
    }

    Some(user)
}

#[cfg(feature = "alloc")]
const fn effective_align(requested: usize) -> usize {
    if requested > HEADER_ALIGN {
        requested
    } else {
        HEADER_ALIGN
    }
}

#[cfg(all(test, feature = "alloc"))]
unsafe fn stored_original_ptr(user: NonNull<u8>) -> *mut c_void {
    unsafe { AllocationHeader::read_before(user) }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use super::{
        AllocationHeader, HEADER_ALIGN, HEADER_BYTES, VescAllocator, aligned_user_ptr,
        copy_allocation_bytes, stored_original_ptr, zero_allocation_bytes,
    };
    #[cfg(feature = "alloc")]
    use core::alloc::{GlobalAlloc, Layout};
    #[cfg(feature = "alloc")]
    use core::ptr::NonNull;
    #[cfg(feature = "alloc")]
    #[test]
    fn allocation_request_includes_alignment_and_header_space() {
        let layout = Layout::from_size_align(7, 32).expect("valid layout");

        assert_eq!(
            AllocationHeader::request_bytes(layout),
            Ok(7 + HEADER_BYTES + 31)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn allocation_request_keeps_pointer_alignment_for_low_alignment_layouts() {
        let layout = Layout::from_size_align(1, 1).expect("valid layout");

        assert_eq!(
            AllocationHeader::request_bytes(layout),
            Ok(1 + HEADER_BYTES + HEADER_ALIGN - 1)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn global_allocator_returns_aligned_dangling_pointer_for_zero_sized_layouts() {
        let layout = Layout::from_size_align(0, 32).expect("valid layout");

        let ptr = unsafe { VescAllocator.alloc(layout) };

        assert!(!ptr.is_null());
        assert_eq!(ptr.addr() % layout.align(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn global_allocator_dealloc_ignores_zero_sized_layouts() {
        let layout = Layout::from_size_align(0, 32).expect("valid layout");
        let ptr = NonNull::<u8>::dangling().as_ptr().with_addr(layout.align());

        unsafe { VescAllocator.dealloc(ptr, layout) };
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn aligned_user_ptr_returns_requested_alignment_and_preserves_original_pointer() {
        let mut backing = [0_u8; 128];
        let raw = backing.as_mut_ptr();
        let user = aligned_user_ptr(raw, 64).expect("aligned pointer");

        assert_eq!(user.as_ptr().addr() % 64, 0);
        assert_eq!(unsafe { stored_original_ptr(user) }, raw.cast());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn aligned_user_ptr_aligns_unaligned_firmware_pointer_without_losing_original() {
        let mut backing = [0_u8; 128];
        let raw = backing.as_mut_ptr().wrapping_add(1);
        let user = aligned_user_ptr(raw, 16).expect("aligned pointer");

        assert_eq!(user.as_ptr().addr() % 16, 0);
        assert_eq!(unsafe { stored_original_ptr(user) }, raw.cast());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn aligned_user_ptr_preserves_original_pointer_for_low_alignment_layouts() {
        let mut backing = [0_u8; 128];
        let raw = backing.as_mut_ptr();
        let user = aligned_user_ptr(raw, 1).expect("aligned pointer");

        assert_eq!(unsafe { stored_original_ptr(user) }, raw.cast());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn aligned_user_ptr_maps_null_firmware_allocation_to_none() {
        assert_eq!(aligned_user_ptr(core::ptr::null_mut(), 4), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn allocation_byte_copy_preserves_dynamic_realloc_contents() {
        let src = [1_u8, 2, 3, 4, 5];
        let mut dst = [0_u8; 8];

        unsafe { copy_allocation_bytes(src.as_ptr(), dst.as_mut_ptr(), src.len()) };

        assert_eq!(&dst[..src.len()], src);
        assert_eq!(&dst[src.len()..], &[0, 0, 0]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn allocation_zeroing_clears_dynamic_alloc_zeroed_contents() {
        let mut dst = [1_u8; 8];

        unsafe { zero_allocation_bytes(dst.as_mut_ptr(), dst.len()) };

        assert_eq!(dst, [0; 8]);
    }
}
