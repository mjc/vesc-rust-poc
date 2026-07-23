//! Native image rebasing helpers for position-independent package payloads.

use crate::loader::LibInfo;

/// Byte offset into a native image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageOffset(usize);

impl ImageOffset {
    /// Create a new byte offset.
    #[must_use]
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }
}

/// Absolute address in a rebased native image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(pub(crate) usize);

/// Base address used to rebase image-relative pointers and offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeImage {
    base_addr: NativeAddress,
}

impl NativeImage {
    /// Construct a native image from its base address.
    #[must_use]
    pub const fn new(base_addr: u32) -> Self {
        Self {
            base_addr: NativeAddress(base_addr as usize),
        }
    }

    /// Construct a native image from loader metadata.
    #[must_use]
    pub fn from_info(info: &LibInfo) -> Self {
        Self::new(info.base_addr)
    }

    /// Return the stored base address.
    #[must_use]
    pub const fn base_addr(self) -> NativeAddress {
        self.base_addr
    }

    /// Rebase a byte offset into the native image.
    #[must_use]
    pub fn rebase_offset(self, offset: ImageOffset) -> NativeAddress {
        // Loader-provided offsets are bounded by the package image. Saturation
        // also keeps a malformed value from triggering integer-overflow panic
        // before the higher-level loader validation can reject it.
        NativeAddress(self.base_addr.0.saturating_add(offset.0))
    }

    /// Rebase a raw image-relative address.
    #[must_use]
    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.rebase_offset(ImageOffset::new(image_addr)).0
    }

    /// Resolve a package-local address materialized by position-independent code.
    ///
    /// Rust may load the same function as either an image-relative GOT value or
    /// an already relocated PC-relative address. VESC loads package images above
    /// their link-time offset range, so only values below the image base need
    /// rebasing.
    #[must_use]
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
