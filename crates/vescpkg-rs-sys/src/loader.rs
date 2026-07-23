//! Native loader metadata and firmware callback type aliases.

use core::ffi::c_void;

/// Handler used to register a `LispBM` extension from native code.
pub type ExtensionHandler = unsafe extern "C" fn(*mut u32, u32) -> u32;
/// Handler used when firmware streams application data into Rust code.
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
/// Function invoked when a native package is asked to stop.
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ThreadEntry(pub core::ptr::NonNull<c_void>);

/// Loader metadata passed from firmware to a native package.
#[repr(C)]
pub struct LibInfo {
    /// Optional stop callback supplied by the firmware.
    pub stop_fun: Option<StopHandler>,
    /// Opaque package argument pointer.
    pub arg: *mut c_void,
    /// Base address of the loaded native image.
    pub base_addr: u32,
}

/// ABI layout metadata for [`LibInfo`].
pub struct LibInfoAbi;

impl LibInfoAbi {
    /// Byte offset of `stop_fun` in the 32-bit firmware layout.
    pub const STOP_FUN_OFFSET: usize = 0;
    /// Byte offset of `arg` in the 32-bit firmware layout.
    pub const ARG_OFFSET: usize = 4;
    /// Byte offset of `base_addr` in the 32-bit firmware layout.
    pub const BASE_ADDR_OFFSET: usize = 8;
    /// Size of the 32-bit firmware layout in bytes.
    pub const SIZE: usize = 12;
    /// Alignment of the 32-bit firmware layout in bytes.
    pub const ALIGN: usize = 4;

    /// Return whether [`LibInfo`] matches the 32-bit firmware layout.
    #[must_use]
    pub const fn has_vesc32_layout() -> bool {
        core::mem::size_of::<LibInfo>() == Self::SIZE
            && core::mem::align_of::<LibInfo>() == Self::ALIGN
            && core::mem::offset_of!(LibInfo, stop_fun) == Self::STOP_FUN_OFFSET
            && core::mem::offset_of!(LibInfo, arg) == Self::ARG_OFFSET
            && core::mem::offset_of!(LibInfo, base_addr) == Self::BASE_ADDR_OFFSET
    }
}

#[cfg(target_pointer_width = "32")]
// A mismatched array length fails compilation if Rust and the C firmware ever
// disagree about this struct. Unlike a runtime assertion, this cannot become a
// crash path in an installed package.
const _: [(); 1] = [(); LibInfoAbi::has_vesc32_layout() as usize];
