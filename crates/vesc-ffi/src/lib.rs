#![cfg_attr(not(test), no_std)]

use core::ffi::{c_char, c_void, CStr};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmValue(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmCount(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmInt(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmUint(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmType(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmCid(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LbmFloat(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmSymbol(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmErrorSymbol(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmBoolSymbol(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmNilSymbol(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramAddress(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoaderBaseAddress(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Milliseconds(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Microseconds(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecondsF32(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTicks(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemSeconds(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Degrees(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radians(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Erpm(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyCycle(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurrentAmps(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrakeCurrentAmps(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputCurrentAmps(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voltage(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppDataLen(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartBaudRate(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartWriteLen(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotorIndex(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanControllerId(pub u8);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanFrameLen(pub u8);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppDataPacket<'a>(pub &'a [u8]);

impl<'a> AppDataPacket<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct MutablePacket<'a>(pub &'a mut [u8]);

impl<'a> MutablePacket<'a> {
    pub const fn new(bytes: &'a mut [u8]) -> Self {
        Self(bytes)
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandPacket<'a>(pub &'a [u8]);

impl<'a> CommandPacket<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplyPacket<'a>(pub &'a [u8]);

impl<'a> ReplyPacket<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfDuplex(pub bool);

impl HalfDuplex {
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub const fn is_enabled(self) -> bool {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemperatureC(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceMeters(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedMetersPerSecond(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmpHours(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WattHours(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OdometerMeters(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryLevel(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocChannel(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneFrequencyHz(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneVoltage(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffDelaySeconds(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatitudeDeg(pub f64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LongitudeDeg(pub f64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AltitudeMeters(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GnssSpeed(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hdop(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accel3(pub [f32; 3]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gyro3(pub [f32; 3]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mag3(pub [f32; 3]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion(pub [f32; 4]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuCalibration(pub [f32; 3]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadCallbackDtSeconds(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CfgParam(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CfgFloat(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CfgInt(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigSetResult(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigXmlBytes<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigPayload<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreadName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSizeBytes(pub usize);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadHandle(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutexHandle(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemaphoreHandle(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwarePtr<T>(pub core::ptr::NonNull<T>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareNonNull<T>(pub core::ptr::NonNull<T>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MallocLen(pub usize);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedFirmwareAllocation<T>(pub core::ptr::NonNull<T>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanPayload<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStatusIndex(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareType(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RollDeg(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchDeg(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YawDeg(pub f32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EulerAngles {
    pub roll: RollDeg,
    pub pitch: PitchDeg,
    pub yaw: YawDeg,
}

impl EulerAngles {
    pub const fn new(roll: RollDeg, pitch: PitchDeg, yaw: YawDeg) -> Self {
        Self { roll, pitch, yaw }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotAxisName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotGraphName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlotGraphIndex(pub i32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub x: f32,
    pub y: f32,
}

impl PlotPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescPin(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VescPinMode(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioPortPtr(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioPin(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalogVoltage(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalogRaw(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbmIoSymbol(pub LbmSymbol);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvmAddress(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvmLen(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NvmBytes<'a>(pub &'a [u8]);

impl<'a> NvmBytes<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EepromAddress(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EepromVar(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct FaultCode(pub i32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct CustomConfigCallback(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct TerminalCommandName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct TerminalHelp<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct TerminalArgNames<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct TerminalCallback(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ThreadEntry(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct GnssData(pub core::ptr::NonNull<c_void>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct RemoteButton(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct RemoteAgeSeconds(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct RemoteState(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct JoystickAxis(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct PpmValue(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct PpmAgeSeconds(pub f32);

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

    pub const fn offset(self) -> usize {
        self.offset
    }

    pub const fn host_offset(self, pointer_size: usize) -> usize {
        self.offset * (pointer_size / 4)
    }
}

pub struct VescIfAbi;

impl VescIfAbi {
    pub const BASE_ADDR: NativeAddress = NativeAddress(0x1000_f800);
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

    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeAddress(usize);

impl NativeAddress {
    pub const fn get(self) -> usize {
        self.0
    }
}

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
        NativeAddress(self.base_addr.get() + offset.get())
    }

    pub fn rebase_addr(self, image_addr: usize) -> usize {
        self.rebase_offset(ImageOffset::new(image_addr)).get()
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

pub struct PackageLifecycle<B = RealBindings> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn register_extension(&self, descriptor: ExtensionDescriptor) -> i32 {
        let descriptor = match descriptor.validate() {
            Ok(descriptor) => descriptor,
            Err(_) => return -1,
        };

        self.api
            .register_extension(descriptor.name(), descriptor.handler())
    }

    pub fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> i32 {
        let descriptor = match descriptor.validate() {
            Ok(descriptor) => descriptor,
            Err(_) => return -1,
        };

        self.api
            .register_extension_from_image(image, descriptor.name(), descriptor.handler())
    }

    pub fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: &[ExtensionDescriptor],
    ) -> i32 {
        descriptors.iter().fold(0, |_, descriptor| {
            self.register_extension_from_image(image, *descriptor)
        })
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

    pub fn install(
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

    const VESC_IF: *const VescIf = VescIfAbi::BASE_ADDR.get() as *const VescIf;

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
    use super::{
        AltitudeMeters, AnalogRaw, AnalogVoltage, AmpHours, AppDataLen, AppDataPacket,
        BatteryLevel, BrakeCurrentAmps, CanControllerId, CanFrameLen, CanPayload, CanStatusIndex,
        CfgFloat, CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult, ConfigXmlBytes,
        CurrentAmps, Degrees, DistanceMeters, DutyCycle, EepromAddress, EepromVar, Erpm,
        EulerAngles, ExtensionHandler, FocChannel, FirmwareNonNull, FirmwarePtr, GnssSpeed,
        GpioPin, GpioPortPtr, Hdop, HardwareType, HalfDuplex, ImageOffset, InputCurrentAmps,
        LatitudeDeg,
        LbmApi, LbmBindings, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt,
        LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo, LibInfoAbi,
        LoaderBaseAddress, LongitudeDeg, MallocLen, MotorIndex, MutexHandle, MutablePacket,
        NativeAddress, NativeImage, NvmAddress, NvmBytes, NvmLen, OdometerMeters, OffDelaySeconds,
        OwnedFirmwareAllocation, PitchDeg, PlotAxisName, PlotGraphIndex, PlotGraphName, PlotPoint,
        ProgramAddress, Radians, ReplyPacket, RollDeg, SemaphoreHandle, SecondsF32,
        SpeedMetersPerSecond, StackSizeBytes, SystemSeconds, SystemTicks, TemperatureC,
        ThreadHandle, ThreadName, ToneFrequencyHz, ToneVoltage, UartBaudRate, UartWriteLen,
        VescIfAbi, VescPin, VescPinMode, Voltage, WattHours, YawDeg,
    };
    use core::cell::Cell;
    use core::ffi::{c_char, c_void, CStr};

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
        unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> i32 {
            self.add_calls.set(self.add_calls.get() + 1);
            self.last_name.set(name as usize);
            self.last_handler.set(handler as usize);
            17
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
    fn raw_vesc_if_offsets_match_the_32_bit_package_header() {
        let expected =
            VescIfAbi::USED_SLOTS.map(|slot| slot.host_offset(core::mem::size_of::<usize>()));

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
            slots.map(|slot| slot.offset()),
            [0, 64, 100, 124, 148, 592, 596, 952]
        );
    }

    #[test]
    fn newtypes_wrap_the_expected_scalar_shapes() {
        assert_eq!(core::mem::size_of::<LbmInt>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<LbmUint>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmType>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmCid>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmFloat>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<LbmSymbol>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmErrorSymbol>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmBoolSymbol>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<LbmNilSymbol>(), core::mem::size_of::<u32>());
        assert_eq!(
            core::mem::size_of::<ProgramAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<LoaderBaseAddress>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(core::mem::size_of::<SecondsF32>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<AppDataLen>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<UartBaudRate>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<UartWriteLen>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<MotorIndex>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<CanControllerId>(), core::mem::size_of::<u8>());
        assert_eq!(core::mem::size_of::<CanFrameLen>(), core::mem::size_of::<u8>());
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
        assert_eq!(core::mem::size_of::<HalfDuplex>(), core::mem::size_of::<bool>());
        assert_eq!(core::mem::size_of::<TemperatureC>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<DistanceMeters>(), core::mem::size_of::<f32>());
        assert_eq!(
            core::mem::size_of::<SpeedMetersPerSecond>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<AmpHours>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<WattHours>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<OdometerMeters>(), core::mem::size_of::<u64>());
        assert_eq!(core::mem::size_of::<BatteryLevel>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<FocChannel>(), core::mem::size_of::<i32>());
        assert_eq!(
            core::mem::size_of::<ToneFrequencyHz>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<ToneVoltage>(), core::mem::size_of::<f32>());
        assert_eq!(
            core::mem::size_of::<OffDelaySeconds>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<LatitudeDeg>(), core::mem::size_of::<f64>());
        assert_eq!(core::mem::size_of::<LongitudeDeg>(), core::mem::size_of::<f64>());
        assert_eq!(core::mem::size_of::<AltitudeMeters>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<GnssSpeed>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<Hdop>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<SystemTicks>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<SystemSeconds>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<Degrees>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<Radians>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<Erpm>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<DutyCycle>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<CurrentAmps>(), core::mem::size_of::<f32>());
        assert_eq!(
            core::mem::size_of::<BrakeCurrentAmps>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(
            core::mem::size_of::<InputCurrentAmps>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<Voltage>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<CfgParam>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<CfgFloat>(), core::mem::size_of::<f32>());
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
        assert_eq!(core::mem::size_of::<ThreadName<'_>>(), core::mem::size_of::<&CStr>());
        assert_eq!(core::mem::size_of::<StackSizeBytes>(), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of::<ThreadHandle>(), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of::<MutexHandle>(), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of::<SemaphoreHandle>(), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of::<FirmwarePtr::<u8>>(), core::mem::size_of::<usize>());
        assert_eq!(
            core::mem::size_of::<FirmwareNonNull::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(core::mem::size_of::<MallocLen>(), core::mem::size_of::<usize>());
        assert_eq!(
            core::mem::size_of::<OwnedFirmwareAllocation::<u8>>(),
            core::mem::size_of::<usize>()
        );
        assert_eq!(core::mem::size_of::<CanPayload<'_>>(), core::mem::size_of::<&[u8]>());
        assert_eq!(core::mem::size_of::<CanStatusIndex>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<HardwareType>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<RollDeg>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<PitchDeg>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<YawDeg>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<EulerAngles>(), core::mem::size_of::<f32>() * 3);
        assert_eq!(
            core::mem::size_of::<PlotAxisName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(
            core::mem::size_of::<PlotGraphName<'_>>(),
            core::mem::size_of::<&CStr>()
        );
        assert_eq!(core::mem::size_of::<PlotGraphIndex>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<PlotPoint>(), core::mem::size_of::<f32>() * 2);
        assert_eq!(core::mem::size_of::<VescPin>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<VescPinMode>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<GpioPortPtr>(), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of::<GpioPin>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<AnalogVoltage>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<AnalogRaw>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<LbmIoSymbol>(), core::mem::size_of::<LbmSymbol>());
        assert_eq!(core::mem::size_of::<NvmAddress>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<NvmLen>(), core::mem::size_of::<u32>());
        assert_eq!(core::mem::size_of::<NvmBytes<'_>>(), core::mem::size_of::<&[u8]>());
        assert_eq!(core::mem::size_of::<EepromAddress>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<EepromVar>(), core::mem::size_of::<i32>());
    }
}
