//! Native image rebasing helpers for position-independent package payloads.

use crate::loader::LibInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageOffset(usize);

impl ImageOffset {
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeImage {
    base_addr: NativeAddress,
}

impl NativeImage {
    pub const fn new(base_addr: u32) -> Self {
        Self {
            base_addr: NativeAddress(base_addr as usize),
        }
    }

    pub fn from_info(info: &LibInfo) -> Self {
        Self::new(info.base_addr)
    }

    pub const fn base_addr(self) -> NativeAddress {
        self.base_addr
    }

    pub fn rebase_offset(self, offset: ImageOffset) -> NativeAddress {
        NativeAddress(self.base_addr.0 + offset.0)
    }

    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.rebase_offset(ImageOffset::new(image_addr)).0
    }

    pub fn rebase_ptr<T>(self, ptr: *const T) -> *const T {
        self.rebase_addr(ptr as usize) as *const T
    }
}
