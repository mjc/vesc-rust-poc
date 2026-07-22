//! Native image rebasing helpers for position-independent package payloads.

use crate::loader::LibInfo;

/// Byte offset into a native image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageOffset(usize);

impl ImageOffset {
    /// Create a new byte offset.
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }
}

/// Absolute address in a rebased native image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(pub(crate) usize);

impl NativeAddress {
    /// Return the absolute address as a machine-sized integer.
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Base address used to rebase image-relative pointers and offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeImage {
    base_addr: NativeAddress,
}

impl NativeImage {
    /// Construct a native image from its base address.
    pub const fn new(base_addr: u32) -> Self {
        Self {
            base_addr: NativeAddress(base_addr as usize),
        }
    }

    /// Construct a native image from loader metadata.
    pub fn from_info(info: &LibInfo) -> Self {
        Self::new(info.base_addr)
    }

    /// Return the stored base address.
    pub const fn base_addr(self) -> NativeAddress {
        self.base_addr
    }

    /// Rebase a byte offset into the native image.
    pub fn rebase_offset(self, offset: ImageOffset) -> NativeAddress {
        NativeAddress(self.base_addr.0 + offset.0)
    }

    /// Rebase a raw image-relative address.
    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.rebase_offset(ImageOffset::new(image_addr)).0
    }

    /// Resolve a package-local address materialized by position-independent code.
    ///
    /// Rust may load the same function as either an image-relative GOT value or
    /// an already relocated PC-relative address. VESC loads package images above
    /// their link-time offset range, so only values below the image base need
    /// rebasing.
    pub fn resolve_addr(self, address: usize) -> usize {
        if address < self.base_addr.0 {
            self.rebase_addr(address)
        } else {
            address
        }
    }

    /// Rebase a raw pointer into the native image address space.
    pub fn rebase_ptr<T>(self, ptr: *const T) -> *const T {
        self.rebase_addr(ptr as usize) as *const T
    }
}
