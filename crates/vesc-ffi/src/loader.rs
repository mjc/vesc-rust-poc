//! Native loader metadata and firmware callback type aliases.

use core::ffi::c_void;

pub type ExtensionHandler = unsafe extern "C" fn(*mut u32, u32) -> u32;
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ThreadEntry(pub core::ptr::NonNull<c_void>);

#[repr(C)]
pub struct LibInfo {
    pub stop_fun: Option<StopHandler>,
    pub arg: *mut c_void,
    pub base_addr: u32,
}

pub struct LibInfoAbi;

impl LibInfoAbi {
    pub const STOP_FUN_OFFSET: usize = 0;
    pub const ARG_OFFSET: usize = 4;
    pub const BASE_ADDR_OFFSET: usize = 8;
    pub const SIZE: usize = 12;
    pub const ALIGN: usize = 4;
}
