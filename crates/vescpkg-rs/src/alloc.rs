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
use core::ops::{Deref, DerefMut};
#[cfg(feature = "alloc")]
use core::ptr;
#[cfg(feature = "alloc")]
use core::ptr::NonNull;
#[cfg(feature = "alloc")]
use rust_alloc::alloc::{alloc, dealloc};

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
            .and_then(|bytes| bytes.checked_add(align.saturating_sub(1)))
            .ok_or(AllocationSizeOverflow)
    }

    unsafe fn write_before(user: NonNull<u8>, original: NonNull<u8>) {
        let original = original.as_ptr().cast::<c_void>();
        let header = user.as_ptr().wrapping_sub(HEADER_BYTES);
        // SAFETY: `aligned_user_ptr` reserves `HEADER_BYTES` immediately before
        // `user`; the source is a live pointer value and the regions do not overlap.
        unsafe {
            ptr::copy_nonoverlapping((&raw const original).cast::<u8>(), header, HEADER_BYTES);
        };
    }

    unsafe fn read_before(user: NonNull<u8>) -> *mut c_void {
        let mut original = ptr::null_mut::<c_void>();
        let header = user.as_ptr().wrapping_sub(HEADER_BYTES);
        // SAFETY: `user` came from `aligned_user_ptr`, which initialized this
        // header, and `original` has space for exactly `HEADER_BYTES`.
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

/// A fallibly allocated single value with normal shared, mutable, and drop semantics.
#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct FallibleBox<T> {
    pointer: NonNull<T>,
}

// SAFETY: this is the same exclusive ownership contract as `Box<T>`.
#[cfg(feature = "alloc")]
unsafe impl<T: Send> Send for FallibleBox<T> {}

// SAFETY: shared references may cross threads exactly when `T` permits it.
#[cfg(feature = "alloc")]
unsafe impl<T: Sync> Sync for FallibleBox<T> {}

#[cfg(feature = "alloc")]
impl<T> FallibleBox<T> {
    /// Allocate and initialize one value, returning it on allocation failure.
    ///
    /// # Errors
    ///
    /// Returns the original value when the global allocator cannot reserve its slot.
    pub fn try_new(value: T) -> Result<Self, T> {
        let layout = Layout::new::<T>();
        let pointer: NonNull<T> = if layout.size() == 0 {
            NonNull::dangling()
        } else {
            // SAFETY: `layout` is non-zero and valid for one `T`; null is
            // converted to allocation failure before the value is moved.
            let Some(pointer) = NonNull::new(unsafe { alloc(layout) }) else {
                return Err(value);
            };
            pointer.cast()
        };
        // SAFETY: `pointer` names the live, aligned slot exclusively owned by
        // the returned box and is initialized exactly once here.
        unsafe { pointer.as_ptr().write(value) };
        Ok(Self { pointer })
    }
}

#[cfg(feature = "alloc")]
impl<T> Deref for FallibleBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: construction initializes this exclusively owned slot, and it
        // remains allocated until `Drop` after all borrows have ended.
        unsafe { self.pointer.as_ref() }
    }
}

#[cfg(feature = "alloc")]
impl<T> DerefMut for FallibleBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: `&mut self` proves exclusive access to the initialized slot.
        unsafe { self.pointer.as_mut() }
    }
}

#[cfg(feature = "alloc")]
impl<T> Drop for FallibleBox<T> {
    fn drop(&mut self) {
        // SAFETY: this owner initialized the value exactly once and has not
        // moved or dropped it since, so its destructor must run exactly once.
        unsafe { ptr::drop_in_place(self.pointer.as_ptr()) };
        deallocate_value(self.pointer);
    }
}

#[cfg(feature = "alloc")]
fn deallocate_value<T>(pointer: NonNull<T>) {
    let layout = Layout::new::<T>();
    if layout.size() != 0 {
        // SAFETY: `pointer` came from the global allocator with exactly
        // `Layout::new::<T>()` and the owning wrapper calls this only once.
        unsafe { dealloc(pointer.cast().as_ptr(), layout) };
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AllocationSizeOverflow;

#[cfg(feature = "alloc")]
// SAFETY: every returned pointer comes from the VESC allocator, satisfies the
// requested layout, and retains its original firmware pointer for deallocation.
unsafe impl GlobalAlloc for VescAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match AllocationHeader::request_bytes(layout) {
            Ok(request) => {
                let raw = call_vesc_ffi!(vesc_malloc(request)).cast::<u8>();
                let Some(user) = aligned_user_ptr(raw, layout.align()) else {
                    if !raw.is_null() {
                        call_vesc_ffi!(vesc_free(raw.cast()));
                    }
                    return ptr::null_mut();
                };
                user.as_ptr()
            }
            Err(AllocationSizeOverflow) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        // SAFETY: the null case returned above, so `ptr` is non-null.
        let user = unsafe { NonNull::new_unchecked(ptr) };
        // SAFETY: `GlobalAlloc::dealloc` requires `ptr` to have been returned by
        // this allocator, which means `aligned_user_ptr` initialized its header.
        let original = unsafe { AllocationHeader::read_before(user) };
        call_vesc_ffi!(vesc_free(original));
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: this method forwards its `GlobalAlloc` layout contract.
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            // SAFETY: a successful allocation is writable for `layout.size()` bytes.
            unsafe { zero_allocation_bytes(ptr, layout.size()) };
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size == 0 {
            // SAFETY: this method receives the same live allocation and layout
            // required by `GlobalAlloc::dealloc`.
            unsafe { self.dealloc(ptr, layout) };
            return ptr::null_mut();
        }

        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };
        // SAFETY: `new_layout` was validated by `Layout::from_size_align`.
        let new_ptr = unsafe { self.alloc(new_layout) };
        if !new_ptr.is_null() {
            let bytes_to_copy = layout.size().min(new_size);
            // SAFETY: both allocations are live, non-overlapping, and contain
            // at least `bytes_to_copy` initialized/writable bytes respectively.
            unsafe { copy_allocation_bytes(ptr, new_ptr, bytes_to_copy) };
            // SAFETY: copying is complete and `ptr` still satisfies the original
            // allocation's deallocation contract.
            unsafe { self.dealloc(ptr, layout) };
        }
        new_ptr
    }
}

#[cfg(feature = "alloc")]
unsafe fn zero_allocation_bytes(dst: *mut u8, len: usize) {
    // SAFETY: the caller guarantees `dst` is writable for `len` bytes.
    unsafe { ptr::write_bytes(dst, 0, len) };
}

#[cfg(feature = "alloc")]
unsafe fn copy_allocation_bytes(src: *const u8, dst: *mut u8, len: usize) {
    // SAFETY: the caller guarantees both regions cover `len` bytes and do not overlap.
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

    // SAFETY: the request reserved the header plus alignment padding before
    // `user`, and both pointers belong to the same live firmware allocation.
    unsafe { AllocationHeader::write_before(user, raw) };

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
    // SAFETY: tests pass a pointer returned by `aligned_user_ptr`.
    unsafe { AllocationHeader::read_before(user) }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use super::{
        AllocationHeader, FallibleBox, HEADER_ALIGN, HEADER_BYTES, aligned_user_ptr,
        copy_allocation_bytes, stored_original_ptr, zero_allocation_bytes,
    };
    #[cfg(feature = "alloc")]
    use core::alloc::Layout;
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

    #[cfg(feature = "alloc")]
    #[test]
    fn fallible_box_initializes_in_place_and_drops_the_value_once() {
        use std::{cell::Cell, rc::Rc};

        #[derive(Debug)]
        struct DropCounter {
            drops: Rc<Cell<u8>>,
            value: u8,
        }

        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.drops.set(self.drops.get().saturating_add(1));
            }
        }

        let drops = Rc::new(Cell::new(0));
        let mut allocation = FallibleBox::try_new(DropCounter {
            drops: Rc::clone(&drops),
            value: 0,
        })
        .expect("host allocation succeeds");
        assert_eq!(drops.get(), 0);
        allocation.value = 7;
        assert_eq!(allocation.value, 7);
        assert_eq!(drops.get(), 0);
        drop(allocation);
        assert_eq!(drops.get(), 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn fallible_box_owner_preserves_pointer_sized_optional_layout() {
        assert_eq!(
            core::mem::size_of::<Option<super::FallibleBox<u32>>>(),
            core::mem::size_of::<*mut u32>()
        );
    }
}
