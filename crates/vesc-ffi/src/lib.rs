#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

use core::ffi::{CStr, c_char, c_void};

mod types;
pub use types::*;

pub mod views;

pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ThreadEntry(pub core::ptr::NonNull<c_void>);

pub type ExtensionHandler = unsafe extern "C" fn(*mut LbmValue, LbmCount) -> LbmValue;
pub type AppDataHandler = unsafe extern "C" fn(*mut u8, u32);
pub type StopHandler = unsafe extern "C" fn(*mut c_void);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescIfSlot {
    name: &'static str,
    offset: usize,
}

impl VescIfSlot {
    pub const fn new(name: &'static str, offset: usize) -> Self {
        Self { name, offset }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn vesc32_byte_offset(self) -> usize {
        self.offset
    }

    pub const fn slot_index(self) -> usize {
        self.offset / 4
    }

    pub const fn host_byte_offset(self, pointer_size: usize) -> usize {
        self.slot_index() * pointer_size
    }
}

pub struct VescIfAbi;

impl VescIfAbi {
    pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
    // These offsets are pinned to the 32-bit VESC native header/table layout.
    pub const LBM_ADD_EXTENSION: VescIfSlot = VescIfSlot::new("lbm_add_extension", 0);
    pub const LBM_ENC_I: VescIfSlot = VescIfSlot::new("lbm_enc_i", 64);
    pub const LBM_DEC_AS_I32: VescIfSlot = VescIfSlot::new("lbm_dec_as_i32", 100);
    pub const LBM_IS_NUMBER: VescIfSlot = VescIfSlot::new("lbm_is_number", 124);
    pub const LBM_ENC_SYM_EERROR: VescIfSlot = VescIfSlot::new("lbm_enc_sym_eerror", 148);
    pub const SEND_APP_DATA: VescIfSlot = VescIfSlot::new("send_app_data", 592);
    pub const SET_APP_DATA_HANDLER: VescIfSlot = VescIfSlot::new("set_app_data_handler", 596);
    pub const SYSTEM_TIME_TICKS: VescIfSlot = VescIfSlot::new("system_time_ticks", 952);

    pub const USED_SLOTS: [VescIfSlot; 8] = [
        Self::LBM_ADD_EXTENSION,
        Self::LBM_ENC_I,
        Self::LBM_DEC_AS_I32,
        Self::LBM_IS_NUMBER,
        Self::LBM_ENC_SYM_EERROR,
        Self::SEND_APP_DATA,
        Self::SET_APP_DATA_HANDLER,
        Self::SYSTEM_TIME_TICKS,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageOffset(usize);

impl ImageOffset {
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(usize);

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

pub trait AppDataBindings {
    /// # Safety
    /// `handler` must be either `None` or a callback with the firmware app-data ABI
    /// that remains valid until it is replaced or cleared.
    unsafe fn set_app_data_handler(&self, handler: Option<AppDataHandler>) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionNameError {
    MissingExtPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
    InvalidExtensionName,
    FirmwareRejected(i32),
}

#[derive(Clone, Copy)]
pub struct ExtensionDescriptor {
    name: &'static CStr,
    handler: ExtensionHandler,
}

impl ExtensionDescriptor {
    pub const fn new(name: &'static CStr, handler: ExtensionHandler) -> Self {
        Self { name, handler }
    }

    pub const fn name(self) -> &'static CStr {
        self.name
    }

    pub const fn handler(self) -> ExtensionHandler {
        self.handler
    }

    pub fn validate(self) -> Result<Self, ExtensionNameError> {
        if self.name.to_bytes().starts_with(b"ext-") {
            Ok(self)
        } else {
            Err(ExtensionNameError::MissingExtPrefix)
        }
    }
}

pub struct RealBindings;

impl LbmBindings for RealBindings {
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32 {
        unsafe { raw::lbm_add_extension(name, handler) }
    }

    unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
        unsafe { raw::lbm_dec_as_i32(value) }
    }

    unsafe fn encode_i32(&self, value: i32) -> LbmValue {
        unsafe { raw::lbm_enc_i(value) }
    }

    unsafe fn is_number(&self, value: LbmValue) -> bool {
        unsafe { raw::lbm_is_number(value) }
    }

    unsafe fn encode_eval_error(&self) -> LbmValue {
        unsafe { raw::lbm_enc_sym_eerror() }
    }
}

impl AppDataBindings for RealBindings {
    unsafe fn set_app_data_handler(&self, handler: Option<AppDataHandler>) -> bool {
        unsafe { raw::vesc_set_app_data_handler(handler) }
    }
}

pub struct LbmApi<B = RealBindings> {
    bindings: B,
}

impl<B: LbmBindings> LbmApi<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    pub fn register_extension(&self, name: &CStr, handler: ExtensionHandler) -> i32 {
        unsafe { self.bindings.add_extension(name.as_ptr(), handler) }
    }

    /// # Safety
    ///
    /// `image` must describe the address space that produced `handler`.
    /// `name` must also point at a string living in that same image if it was
    /// compiled into native code rather than host memory.
    pub unsafe fn register_extension_from_image(
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

pub struct PackageLifecycle<B = RealBindings> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn register_extension(
        &self,
        descriptor: ExtensionDescriptor,
    ) -> Result<i32, RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;

        let result = self
            .api
            .register_extension(descriptor.name(), descriptor.handler());
        if result < 0 {
            Err(RegisterError::FirmwareRejected(result))
        } else {
            Ok(result)
        }
    }

    /// # Safety
    ///
    /// `image` must describe the address space that produced every callback
    /// inside `descriptor`.
    pub unsafe fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<i32, RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| RegisterError::InvalidExtensionName)?;

        let result = unsafe {
            self.api
                .register_extension_from_image(image, descriptor.name(), descriptor.handler())
        };
        if result < 0 {
            Err(RegisterError::FirmwareRejected(result))
        } else {
            Ok(result)
        }
    }

    /// # Safety
    ///
    /// `image` must describe the address space that produced every callback
    /// inside `descriptors`.
    pub unsafe fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: &[ExtensionDescriptor],
    ) -> Result<(), RegisterError> {
        for descriptor in descriptors {
            unsafe {
                self.register_extension_from_image(image, *descriptor)?;
            }
        }
        Ok(())
    }
}

pub struct LoopbackLifecycle<B = RealBindings> {
    bindings: B,
}

impl<B: AppDataBindings> LoopbackLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// # Safety
    ///
    /// `info` must either be null or point to a live `LibInfo` owned by the
    /// native image described by `image`. `stop_handler` and
    /// `app_data_handler` must also originate from that image.
    pub unsafe fn install(
        &self,
        info: *mut LibInfo,
        image: NativeImage,
        stop_handler: StopHandler,
        app_data_handler: AppDataHandler,
    ) -> bool {
        let stop_handler = unsafe { image.rebase_stop_handler(stop_handler) };
        let app_data_handler = unsafe { image.rebase_app_data_handler(app_data_handler) };

        if let Some(info) = unsafe { info.as_mut() } {
            info.stop_fun = Some(stop_handler);
        }

        unsafe { self.bindings.set_app_data_handler(Some(app_data_handler)) }
    }

    pub fn clear_app_data_handler(&self) -> bool {
        unsafe { self.bindings.set_app_data_handler(None) }
    }
}

#[cfg_attr(test, allow(dead_code))]
pub mod raw {
    use super::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi};
    use core::ffi::{c_char, c_uchar};

    #[repr(C)]
    pub struct VescIf {
        // 32-bit package table slot 0.
        lbm_add_extension: unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool,
        _reserved_before_lbm_enc_i: [usize; 15],
        // 32-bit package table slot 16.
        lbm_enc_i: unsafe extern "C" fn(i32) -> LbmValue,
        _reserved_before_lbm_dec_as_i32: [usize; 8],
        // 32-bit package table slot 25.
        lbm_dec_as_i32: unsafe extern "C" fn(LbmValue) -> i32,
        _reserved_before_lbm_is_number: [usize; 5],
        // 32-bit package table slot 31.
        lbm_is_number: unsafe extern "C" fn(LbmValue) -> bool,
        _reserved_before_lbm_enc_sym_eerror: [usize; 5],
        // 32-bit package table slot 37.
        lbm_enc_sym_eerror: u32,
        _reserved_after_lbm_enc_sym_eerror: [usize; 110],
        // 32-bit package table slot 148.
        send_app_data: unsafe extern "C" fn(*mut c_uchar, u32),
        // 32-bit package table slot 149.
        set_app_data_handler: unsafe extern "C" fn(Option<AppDataHandler>) -> bool,
        _reserved_after_app_data: [usize; 88],
        // 32-bit package table slot 238.
        system_time_ticks: unsafe extern "C" fn() -> u32,
    }

    const VESC_IF: *const VescIf = VescIfAbi::BASE_ADDR.0 as *const VescIf;

    pub unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> i32 {
        unsafe { ((*VESC_IF).lbm_add_extension)(name as *mut c_char, handler) as i32 }
    }

    pub unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
        unsafe { ((*VESC_IF).lbm_dec_as_i32)(value) }
    }

    pub unsafe fn lbm_enc_i(value: i32) -> LbmValue {
        unsafe { ((*VESC_IF).lbm_enc_i)(value) }
    }

    pub unsafe fn lbm_is_number(value: LbmValue) -> bool {
        unsafe { ((*VESC_IF).lbm_is_number)(value) }
    }

    pub unsafe fn lbm_enc_sym_eerror() -> LbmValue {
        unsafe { LbmValue((*VESC_IF).lbm_enc_sym_eerror) }
    }

    pub unsafe fn vesc_set_app_data_handler(handler: Option<AppDataHandler>) -> bool {
        unsafe { ((*VESC_IF).set_app_data_handler)(handler) }
    }

    pub unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
        unsafe { ((*VESC_IF).send_app_data)(data as *mut c_uchar, len) }
    }

    pub unsafe fn vesc_system_time_ticks() -> u32 {
        unsafe { ((*VESC_IF).system_time_ticks)() }
    }

    #[cfg(test)]
    pub fn vesc_if_offsets_for_tests() -> [usize; 8] {
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
    #[allow(unused_imports)]
    use super::{
        AppDataLen, AppDataPacket, CanControllerId, CanFrameLen, CanPayload, CanStatusIndex,
        CfgFloat, CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult, ConfigXmlBytes,
        EepromAddress, EepromVar, ExtensionDescriptor, ExtensionHandler, FirmwareNonNull,
        FirmwarePtr, GpioPin, GpioPortPtr, HalfDuplex, HardwareType, ImageOffset, LbmApi,
        LbmBindings, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt,
        LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo, LibInfoAbi,
        LoaderBaseAddress, MallocLen, MotorIndex, MutablePacket, MutexHandle, NativeAddress,
        NativeImage, NvmAddress, NvmBytes, NvmLen, OwnedFirmwareAllocation, PackageLifecycle,
        PlotAxisName, PlotGraphIndex, PlotGraphName, PlotPoint, ProgramAddress, RegisterError,
        ReplyPacket,
        SemaphoreHandle, StackSizeBytes, SystemTicks, ThreadHandle, ThreadName, UartBaudRate,
        UartWriteLen, VescIfAbi, VescPin, VescPinMode,
    };
    use core::cell::Cell;
    use core::ffi::{CStr, c_char, c_void};

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        encode_calls: Cell<usize>,
        last_name: Cell<usize>,
        last_handler: Cell<usize>,
        add_results: Cell<[i32; 2]>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self::with_add_results([17, 17])
        }

        fn with_add_results(add_results: [i32; 2]) -> Self {
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
        unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32 {
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

    unsafe extern "C" fn stub_handler(_args: *mut LbmValue, _count: LbmCount) -> LbmValue {
        LbmValue(0)
    }

    unsafe extern "C" fn stub_app_data_handler(_data: *mut u8, _len: u32) {}

    unsafe extern "C" fn stub_stop_handler(_arg: *mut c_void) {}

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
            unsafe { api.register_extension_from_image(image, name, stub_handler) },
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
        assert_eq!(image.base_addr(), NativeAddress(0x2000));
        assert_eq!(
            image.rebase_offset(ImageOffset::new(0x61)),
            NativeAddress(0x2061)
        );
        assert_eq!(image.rebase_ptr(0x1df as *const c_char) as usize, 0x21df);

        let rebased_app_data =
            unsafe { image.rebase_app_data_handler(stub_app_data_handler) } as *const () as usize;
        assert_eq!(
            rebased_app_data,
            stub_app_data_handler as *const () as usize + 0x2000
        );

        let rebased_stop =
            unsafe { image.rebase_stop_handler(stub_stop_handler) } as *const () as usize;
        assert_eq!(
            rebased_stop,
            stub_stop_handler as *const () as usize + 0x2000
        );
    }

    #[test]
    fn package_registration_reports_name_validation_and_firmware_rejection() {
        let bindings = FakeBindings::with_add_results([-42, 17]);
        let lifecycle = PackageLifecycle::new(bindings);

        let invalid = ExtensionDescriptor::new(c"bad-name", stub_handler);
        assert_eq!(
            lifecycle.register_extension(invalid),
            Err(RegisterError::InvalidExtensionName)
        );

        let rejected = ExtensionDescriptor::new(c"ext-rust-reject", stub_handler);
        assert_eq!(
            lifecycle.register_extension(rejected),
            Err(RegisterError::FirmwareRejected(-42))
        );
    }

    #[test]
    fn package_batch_registration_stops_on_the_first_failure() {
        let bindings = FakeBindings::with_add_results([-42, 17]);
        let lifecycle = PackageLifecycle::new(bindings);

        let first = ExtensionDescriptor::new(c"ext-rust-a", stub_handler);
        let second = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);
        assert_eq!(
            unsafe {
                lifecycle.register_extensions_from_image(
                    NativeImage::new(0x2000),
                    &[first, second],
                )
            },
            Err(RegisterError::FirmwareRejected(-42))
        );
        assert_eq!(lifecycle.api.bindings.add_calls.get(), 1);
    }

    #[test]
    fn lib_info_abi_constants_match_the_vesc_native_loader_layout() {
        assert_eq!(LibInfoAbi::STOP_FUN_OFFSET, 0);
        assert_eq!(LibInfoAbi::ARG_OFFSET, 4);
        assert_eq!(LibInfoAbi::BASE_ADDR_OFFSET, 8);
        assert_eq!(LibInfoAbi::SIZE, 12);
        assert_eq!(LibInfoAbi::ALIGN, 4);
    }

    #[test]
    fn lib_info_repr_c_layout_scales_with_the_compilation_pointer_width() {
        let pointer_size = core::mem::size_of::<usize>();

        assert_eq!(core::mem::size_of::<LibInfo>(), pointer_size * 3);
        assert_eq!(core::mem::align_of::<LibInfo>(), pointer_size);
        assert_eq!(core::mem::offset_of!(LibInfo, stop_fun), 0);
        assert_eq!(core::mem::offset_of!(LibInfo, arg), pointer_size);
        assert_eq!(core::mem::offset_of!(LibInfo, base_addr), pointer_size * 2);
    }

    #[test]
    fn raw_vesc_if_offsets_match_the_documented_32_bit_package_header_slots() {
        let expected =
            VescIfAbi::USED_SLOTS.map(|slot| slot.host_byte_offset(core::mem::size_of::<usize>()));

        assert_eq!(super::raw::vesc_if_offsets_for_tests(), expected);
    }

    #[test]
    fn vesc_if_slot_constants_name_the_package_header_offsets() {
        let slots = VescIfAbi::USED_SLOTS;

        assert_eq!(VescIfAbi::BASE_ADDR, NativeAddress(0x1000_f800));
        assert_eq!(
            slots.map(|slot| slot.name()),
            [
                "lbm_add_extension",
                "lbm_enc_i",
                "lbm_dec_as_i32",
                "lbm_is_number",
                "lbm_enc_sym_eerror",
                "send_app_data",
                "set_app_data_handler",
                "system_time_ticks",
            ]
        );
        assert_eq!(
            slots.map(|slot| slot.vesc32_byte_offset()),
            [0, 64, 100, 124, 148, 592, 596, 952]
        );
        assert_eq!(slots.map(|slot| slot.slot_index()), [0, 16, 25, 31, 37, 148, 149, 238]);
    }

    #[test]
    fn newtypes_wrap_the_expected_scalar_shapes() {
        assert_eq!(core::mem::size_of::<LbmInt>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<LbmUint>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmType>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmCid>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<LbmFloat>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmErrorSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmBoolSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LbmNilSymbol>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<ProgramAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LoaderBaseAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<AppDataLen>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<UartBaudRate>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<UartWriteLen>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<MotorIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<CanControllerId>(),
            core::mem::size_of::<u8>()
        );
        assert_eq!(
            core::mem::size_of::<CanFrameLen>(),
            core::mem::size_of::<u8>()
        );
        assert_eq!(
            core::mem::size_of::<AppDataPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<MutablePacket<'_>>(),
            core::mem::size_of::<&mut [u8]>()
        );
        assert_eq!(
            core::mem::size_of::<CommandPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ReplyPacket<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<HalfDuplex>(),
            core::mem::size_of::<bool>()
        );
        assert_eq!(
            core::mem::size_of::<SystemTicks>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<CfgParam>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<CfgFloat>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<CfgInt>(), core::mem::size_of::<i32>());
        assert_eq!(
            core::mem::size_of::<ConfigSetResult>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<ConfigXmlBytes<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ConfigPayload<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<ThreadName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<StackSizeBytes>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<ThreadHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<MutexHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<SemaphoreHandle>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<FirmwarePtr::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<FirmwareNonNull::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<MallocLen>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<OwnedFirmwareAllocation::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::size_of::<CanPayload<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<CanStatusIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<HardwareType>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<PlotAxisName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<PlotGraphName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<PlotGraphIndex>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<PlotPoint>(),
            core::mem::size_of::<f32>() * 2
        );
        assert_eq!(core::mem::size_of::<VescPin>(), core::mem::size_of::<i32>());
        assert_eq!(
            core::mem::size_of::<VescPinMode>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<GpioPortPtr>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(core::mem::size_of::<GpioPin>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<LbmIoSymbol>(),
            core::mem::size_of::<LbmSymbol>()
        );
        assert_eq!(
            core::mem::size_of::<NvmAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(core::mem::size_of::<NvmLen>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<NvmBytes<'_>>(),
            core::mem::size_of::<&[u8]>()
        );
        assert_eq!(
            core::mem::size_of::<EepromAddress>(),
            core::mem::size_of::<i32>()
        );
        assert_eq!(
            core::mem::size_of::<EepromVar>(),
            core::mem::size_of::<i32>()
        );
    }

    #[test]
    fn transparent_wrappers_expose_raw_tuple_fields() {
        let raw = [1_u8, 2, 3];
        let mut mut_raw = [4_u8, 5, 6];
        let name = c"axis";

        assert_eq!(LbmInt(-7).0, -7);
        assert_eq!(LbmFloat(3.5).0, 3.5);
        assert_eq!(HalfDuplex(true).0, true);
        assert_eq!(ConfigXmlBytes(&raw).0, &raw);
        assert_eq!(ConfigPayload(&raw).0, &raw);
        assert_eq!(ThreadName(name).0, name);
        assert_eq!(CanPayload(&raw).0, &raw);
        assert_eq!(PlotAxisName(name).0, name);
        assert_eq!(PlotGraphName(name).0, name);
        assert_eq!(NvmBytes(&raw).0, &raw);
        {
            let packet = MutablePacket(&mut mut_raw);
            packet.0[0] = 9;
        }
        assert_eq!(mut_raw[0], 9);
        let point = PlotPoint { x: 1.5, y: 2.5 };
        assert_eq!(point.x, 1.5);
        assert_eq!(point.y, 2.5);
    }
}
