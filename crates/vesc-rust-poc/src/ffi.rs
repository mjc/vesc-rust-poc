use core::ffi::{c_char, c_void, CStr};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmValue(pub usize);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmCount(pub usize);

pub type ExtensionHandler = unsafe extern "C" fn(*mut LbmValue, LbmCount) -> LbmValue;
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct LibInfo {
    pub stop_fun: Option<StopHandler>,
    pub arg: *mut c_void,
    pub base_addr: u32,
}

pub trait LbmBindings {
    /// # Safety
    /// `name` must be a valid NUL-terminated string for the duration of the call,
    /// and `handler` must obey the firmware's extension callback ABI.
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32;
    /// # Safety
    /// `value` must be a valid firmware-provided LispBM value.
    unsafe fn decode_i32(&self, value: LbmValue) -> i32;
    /// # Safety
    /// The returned value is owned by the caller as an opaque LispBM value.
    unsafe fn encode_i32(&self, value: i32) -> LbmValue;
}

pub struct RealBindings;

impl LbmBindings for RealBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32 {
        raw::lbm_add_extension(name, handler)
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        raw::lbm_dec_as_i32(value)
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        raw::lbm_enc_i(value)
    }
}

pub struct LbmApi<B = RealBindings> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn register_extension(&self, name: &CStr, handler: ExtensionHandler) -> i32 {
        unsafe { self.bindings.add_extension(name.as_ptr(), handler) }
    }

    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    pub fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { self.bindings.encode_i32(value) }
    }
}

pub(crate) mod raw {
    use super::{AppDataHandler, ExtensionHandler, LbmValue};
    use core::ffi::c_char;

    #[repr(C)]
    pub(crate) struct VescIf {
        _pad0: [u8; 592],
        send_app_data: unsafe extern "C" fn(*mut u8, u32),
        set_app_data_handler: unsafe extern "C" fn(Option<AppDataHandler>) -> bool,
        _pad1: [u8; 352],
        system_time_ticks: unsafe extern "C" fn() -> u32,
    }

    const VESC_IF: *const VescIf = 0x1000_f800 as *const VescIf;

    extern "C" {
        #[link_name = "lbm_add_extension"]
        fn raw_lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> i32;
        #[link_name = "lbm_dec_as_i32"]
        fn raw_lbm_dec_as_i32(value: LbmValue) -> i32;
        #[link_name = "lbm_enc_i"]
        fn raw_lbm_enc_i(value: i32) -> LbmValue;
    }

    pub(crate) unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> i32 {
        // Safety: this simply forwards the raw pointer and callback to the firmware ABI.
        raw_lbm_add_extension(name, handler)
    }

    pub(crate) unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
        // Safety: the value is forwarded by value to the firmware decoder.
        raw_lbm_dec_as_i32(value)
    }

    pub(crate) unsafe fn lbm_enc_i(value: i32) -> LbmValue {
        // Safety: the firmware encodes the integer into an opaque LispBM value.
        raw_lbm_enc_i(value)
    }

    pub(crate) unsafe fn vesc_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
        ((*VESC_IF).set_app_data_handler)(handler)
    }

    pub(crate) unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
        ((*VESC_IF).send_app_data)(data as *mut u8, len)
    }

    pub(crate) unsafe fn vesc_system_time_ticks() -> u32 {
        ((*VESC_IF).system_time_ticks)()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtensionHandler, LbmApi, LbmBindings, LbmCount, LbmValue};
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        encode_calls: Cell<usize>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                encode_calls: Cell::new(0),
            }
        }
    }

    impl LbmBindings for FakeBindings {
        /// # Safety
        /// The fake test binding ignores the pointer and callback, so it cannot violate
        /// the firmware ABI invariants.
        unsafe fn add_extension(&self, _name: *const c_char, _handler: ExtensionHandler) -> i32 {
            self.add_calls.set(self.add_calls.get() + 1);
            17
        }

        /// # Safety
        /// The fake test binding only decodes the raw integer wrapper by value.
        unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
            self.decode_calls.set(self.decode_calls.get() + 1);
            value.0 as i32
        }

        /// # Safety
        /// The fake test binding only rewraps the integer into the opaque type.
        unsafe fn encode_i32(&self, value: i32) -> LbmValue {
            self.encode_calls.set(self.encode_calls.get() + 1);
            LbmValue(value as usize)
        }
    }

    unsafe extern "C" fn stub_handler(_args: *mut LbmValue, _count: LbmCount) -> LbmValue {
        LbmValue(0)
    }

    #[test]
    fn wrapper_delegates_through_the_binding_trait() {
        let bindings = FakeBindings::new();
        let api = LbmApi::new(bindings);
        let name = c"ext-rust-add";

        assert_eq!(api.register_extension(name, stub_handler), 17);
        assert_eq!(api.decode_i32(LbmValue(3)), 3);
        assert_eq!(api.encode_i32(9), LbmValue(9));
    }
}
