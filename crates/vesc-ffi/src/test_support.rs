//! Host-side fake firmware bindings for unit tests in dependent crates.

use core::cell::Cell;
use core::ffi::c_char;

use crate::{AppDataBindings, AppDataHandler, ExtensionHandler, LbmBindings, LbmValue};

pub struct FakeBindings {
    pub add_calls: Cell<usize>,
    pub decode_calls: Cell<usize>,
    pub encode_calls: Cell<usize>,
    pub last_name: Cell<usize>,
    pub last_handler: Cell<usize>,
    add_results: Cell<[bool; 2]>,
}

impl Default for FakeBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBindings {
    pub fn new() -> Self {
        Self::with_add_results([true, true])
    }

    pub fn rejecting() -> Self {
        Self::with_add_results([false, false])
    }

    pub fn with_add_results(add_results: [bool; 2]) -> Self {
        Self {
            add_calls: Cell::new(0),
            decode_calls: Cell::new(0),
            encode_calls: Cell::new(0),
            last_name: Cell::new(0),
            last_handler: Cell::new(0),
            add_results: Cell::new(add_results),
        }
    }
}

impl LbmBindings for FakeBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        self.add_calls.set(self.add_calls.get() + 1);
        self.last_name.set(name as usize);
        self.last_handler.set(handler as usize);
        let index = self.add_calls.get().saturating_sub(1).min(1);
        self.add_results.get()[index]
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        self.decode_calls.set(self.decode_calls.get() + 1);
        value.0 as i32
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        self.encode_calls.set(self.encode_calls.get() + 1);
        LbmValue(value as u32)
    }

    unsafe fn is_number(&self, _value: LbmValue) -> bool {
        true
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        LbmValue(0xffff_ffff)
    }
}

pub struct FakeAppDataBindings {
    pub handler_calls: Cell<usize>,
    pub ticks: Cell<u32>,
    pub send_calls: Cell<usize>,
    pub last_handler: Cell<usize>,
    pub last_data: Cell<usize>,
    pub last_len: Cell<u32>,
}

impl Default for FakeAppDataBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeAppDataBindings {
    pub fn new() -> Self {
        Self::with_ticks(0)
    }

    pub fn with_ticks(ticks: u32) -> Self {
        Self {
            handler_calls: Cell::new(0),
            ticks: Cell::new(ticks),
            send_calls: Cell::new(0),
            last_handler: Cell::new(0),
            last_data: Cell::new(0),
            last_len: Cell::new(0),
        }
    }
}

impl AppDataBindings for FakeAppDataBindings {
    unsafe fn set_app_data_handler(&self, handler: Option<AppDataHandler>) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler
            .set(handler.map_or(0, |handler| handler as *const () as usize));
        true
    }

    fn system_time_ticks(&self) -> u32 {
        self.ticks.get()
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        self.send_calls.set(self.send_calls.get() + 1);
        self.last_data.set(data as usize);
        self.last_len.set(len);
    }
}

pub mod stubs {
    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real extension handler ABI.
    pub unsafe extern "C" fn extension_handler(_args: *mut u32, _count: u32) -> u32 {
        0
    }

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real stop handler ABI.
    pub unsafe extern "C" fn stop_handler(_arg: *mut core::ffi::c_void) {}

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real app-data handler ABI.
    pub unsafe extern "C" fn app_data_handler(_data: *mut u8, _len: u32) {}
}
