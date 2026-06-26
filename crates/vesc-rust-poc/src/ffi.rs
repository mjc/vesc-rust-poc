use core::ffi::{c_char, c_void, CStr};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmValue(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmCount(pub u32);

pub type ExtensionHandler = unsafe extern "C" fn(*mut LbmValue, LbmCount) -> LbmValue;
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct LibInfo {
    pub stop_fun: Option<StopHandler>,
    pub arg: *mut c_void,
    pub base_addr: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeImage {
    base_addr: u32,
}

impl NativeImage {
    pub const fn new(base_addr: u32) -> Self {
        Self { base_addr }
    }

    pub fn from_info(info: &LibInfo) -> Self {
        Self::new(info.base_addr)
    }

    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.base_addr as usize + image_addr
    }

    pub fn rebase_ptr<T>(self, ptr: *const T) -> *const T {
        self.rebase_addr(ptr as usize) as *const T
    }

    /// # Safety
    ///
    /// `handler` must be a function pointer emitted into the currently loaded native image.
    pub unsafe fn rebase_extension_handler(self, handler: ExtensionHandler) -> ExtensionHandler {
        unsafe { core::mem::transmute(self.rebase_addr(handler as usize)) }
    }

    /// # Safety
    ///
    /// `handler` must be a function pointer emitted into the currently loaded native image.
    pub unsafe fn rebase_app_data_handler(self, handler: AppDataHandler) -> AppDataHandler {
        unsafe { core::mem::transmute(self.rebase_addr(handler as usize)) }
    }

    /// # Safety
    ///
    /// `handler` must be a function pointer emitted into the currently loaded native image.
    pub unsafe fn rebase_stop_handler(self, handler: StopHandler) -> StopHandler {
        unsafe { core::mem::transmute(self.rebase_addr(handler as usize)) }
    }
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
    /// # Safety
    /// `value` must be a valid firmware-provided LispBM value.
    unsafe fn is_number(&self, value: LbmValue) -> bool;
    /// # Safety
    /// The returned value is the firmware's eval-error symbol.
    unsafe fn encode_eval_error(&self) -> LbmValue;
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

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        raw::lbm_is_number(value)
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        raw::lbm_enc_sym_eerror()
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

    pub fn register_extension_from_image(
        &self,
        image: NativeImage,
        name: &CStr,
        handler: ExtensionHandler,
    ) -> i32 {
        let name = image.rebase_ptr(name.as_ptr());
        let handler = unsafe { image.rebase_extension_handler(handler) };
        unsafe { self.bindings.add_extension(name, handler) }
    }

    pub fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { self.bindings.decode_i32(value) }
    }

    pub fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { self.bindings.encode_i32(value) }
    }

    pub fn is_number(&self, value: LbmValue) -> bool {
        unsafe { self.bindings.is_number(value) }
    }

    pub fn encode_eval_error(&self) -> LbmValue {
        unsafe { self.bindings.encode_eval_error() }
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) mod raw {
    use super::{AppDataHandler, ExtensionHandler, LbmValue};
    use core::ffi::{c_char, c_uchar};

    #[repr(C)]
    pub(crate) struct VescIf {
        lbm_add_extension: unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool,
        _reserved_before_lbm_enc_i: [usize; 15],
        lbm_enc_i: unsafe extern "C" fn(i32) -> LbmValue,
        _reserved_before_lbm_dec_as_i32: [usize; 8],
        lbm_dec_as_i32: unsafe extern "C" fn(LbmValue) -> i32,
        _reserved_before_lbm_is_number: [usize; 5],
        lbm_is_number: unsafe extern "C" fn(LbmValue) -> bool,
        _reserved_before_lbm_enc_sym_eerror: [usize; 5],
        lbm_enc_sym_eerror: u32,
        _reserved_after_lbm_enc_sym_eerror: [usize; 110],
        send_app_data: unsafe extern "C" fn(*mut c_uchar, u32),
        set_app_data_handler: unsafe extern "C" fn(Option<AppDataHandler>) -> bool,
        _reserved_after_app_data: [usize; 88],
        system_time_ticks: unsafe extern "C" fn() -> u32,
    }

    const VESC_IF: *const VescIf = 0x1000_f800 as *const VescIf;

    pub(crate) unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> i32 {
        // Safety: this forwards the raw pointer and callback to the firmware ABI.
        ((*VESC_IF).lbm_add_extension)(name as *mut c_char, handler) as i32
    }

    pub(crate) unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
        // Safety: the value is forwarded by value to the firmware decoder.
        ((*VESC_IF).lbm_dec_as_i32)(value)
    }

    pub(crate) unsafe fn lbm_enc_i(value: i32) -> LbmValue {
        // Safety: the firmware encodes the integer into an opaque LispBM value.
        ((*VESC_IF).lbm_enc_i)(value)
    }

    pub(crate) unsafe fn lbm_is_number(value: LbmValue) -> bool {
        // Safety: the value is forwarded by value to the firmware type predicate.
        ((*VESC_IF).lbm_is_number)(value)
    }

    pub(crate) unsafe fn lbm_enc_sym_eerror() -> LbmValue {
        // Safety: the firmware returns its canonical eval-error symbol value.
        LbmValue((*VESC_IF).lbm_enc_sym_eerror)
    }

    pub(crate) unsafe fn vesc_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
        // Safety: the callback has the package app-data ABI and a static lifetime.
        ((*VESC_IF).set_app_data_handler)(handler)
    }

    pub(crate) unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
        // Safety: the firmware reads `len` bytes from the provided packet buffer.
        ((*VESC_IF).send_app_data)(data as *mut c_uchar, len)
    }

    pub(crate) unsafe fn vesc_system_time_ticks() -> u32 {
        // Safety: this forwards to the firmware's monotonic tick source.
        ((*VESC_IF).system_time_ticks)()
    }

    #[cfg(test)]
    pub(crate) fn vesc_if_offsets_for_tests() -> [usize; 8] {
        [
            core::mem::offset_of!(VescIf, lbm_add_extension),
            core::mem::offset_of!(VescIf, lbm_enc_i),
            core::mem::offset_of!(VescIf, lbm_dec_as_i32),
            core::mem::offset_of!(VescIf, lbm_is_number),
            core::mem::offset_of!(VescIf, lbm_enc_sym_eerror),
            core::mem::offset_of!(VescIf, send_app_data),
            core::mem::offset_of!(VescIf, set_app_data_handler),
            core::mem::offset_of!(VescIf, system_time_ticks),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtensionHandler, LbmApi, LbmBindings, LbmCount, LbmValue, NativeImage};
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        encode_calls: Cell<usize>,
        last_name: Cell<usize>,
        last_handler: Cell<usize>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                encode_calls: Cell::new(0),
                last_name: Cell::new(0),
                last_handler: Cell::new(0),
            }
        }
    }

    impl LbmBindings for FakeBindings {
        /// # Safety
        /// The fake test binding ignores the pointer and callback, so it cannot violate
        /// the firmware ABI invariants.
        unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32 {
            self.add_calls.set(self.add_calls.get() + 1);
            self.last_name.set(name as usize);
            self.last_handler.set(handler as usize);
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
            LbmValue(value as u32)
        }

        unsafe fn is_number(&self, _value: LbmValue) -> bool {
            true
        }

        unsafe fn encode_eval_error(&self) -> LbmValue {
            LbmValue(0xffff_ffff)
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
        assert!(api.is_number(LbmValue(9)));
        assert_eq!(api.encode_eval_error(), LbmValue(0xffff_ffff));
    }

    #[test]
    fn native_image_rebases_rust_owned_extension_pointers() {
        let bindings = FakeBindings::new();
        let api = LbmApi::new(bindings);
        let image = NativeImage::new(0x2000);
        let name = c"ext-rust-probe-v5";

        assert_eq!(
            api.register_extension_from_image(image, name, stub_handler),
            17
        );
        assert_eq!(
            api.bindings.last_name.get(),
            name.as_ptr() as usize + 0x2000
        );
        assert_eq!(
            api.bindings.last_handler.get(),
            stub_handler as *const () as usize + 0x2000
        );
        assert_eq!(image.rebase_addr(0x61), 0x2061);
        assert_eq!(image.rebase_ptr(0x1df as *const c_char) as usize, 0x21df);
    }

    #[test]
    fn raw_vesc_if_offsets_match_the_32_bit_package_header() {
        let pointer_scale = core::mem::size_of::<usize>() / 4;
        let expected = [0, 64, 100, 124, 148, 592, 596, 952].map(|offset| offset * pointer_scale);

        assert_eq!(super::raw::vesc_if_offsets_for_tests(), expected);
    }
}
