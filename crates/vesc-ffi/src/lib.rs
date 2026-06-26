#![cfg_attr(not(test), no_std)]

use core::ffi::{c_char, c_void, CStr};

macro_rules! transparent_value_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        $vis struct $name(pub $inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

macro_rules! transparent_value_type_generic {
    ($(#[$meta:meta])* $vis:vis struct $name:ident<$lt:lifetime>($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        $vis struct $name<$lt>(pub $inner);

        impl<$lt> $name<$lt> {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl<$lt> From<$inner> for $name<$lt> {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

macro_rules! transparent_eq_value_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis struct $name(pub $inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

macro_rules! transparent_eq_value_type_generic {
    ($(#[$meta:meta])* $vis:vis struct $name:ident<$lt:lifetime>($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis struct $name<$lt>(pub $inner);

        impl<$lt> $name<$lt> {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl<$lt> From<$inner> for $name<$lt> {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

macro_rules! transparent_eq_value_type_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident<$ty:ident>($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis struct $name<$ty>(pub $inner);

        impl<$ty> $name<$ty> {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl<$ty> From<$inner> for $name<$ty> {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

transparent_eq_value_type!(pub struct LbmValue(u32););
transparent_eq_value_type!(pub struct LbmCount(u32););
transparent_eq_value_type!(pub struct LbmInt(i32););
transparent_eq_value_type!(pub struct LbmUint(u32););
transparent_eq_value_type!(pub struct LbmType(u32););
transparent_eq_value_type!(pub struct LbmCid(u32););
transparent_value_type!(pub struct LbmFloat(f32););
transparent_eq_value_type!(pub struct LbmSymbol(u32););
transparent_eq_value_type!(pub struct LbmErrorSymbol(u32););
transparent_eq_value_type!(pub struct LbmBoolSymbol(u32););
transparent_eq_value_type!(pub struct LbmNilSymbol(u32););
transparent_eq_value_type!(pub struct ProgramAddress(u32););
transparent_eq_value_type!(pub struct LoaderBaseAddress(u32););
transparent_eq_value_type!(pub struct Milliseconds(u32););
transparent_eq_value_type!(pub struct Microseconds(u32););
transparent_value_type!(pub struct SecondsF32(f32););
transparent_eq_value_type!(pub struct SystemTicks(u32););
transparent_value_type!(pub struct SystemSeconds(f32););
transparent_value_type!(pub struct Degrees(f32););
transparent_value_type!(pub struct Radians(f32););
transparent_value_type!(pub struct Erpm(f32););
transparent_value_type!(pub struct DutyCycle(f32););
transparent_value_type!(pub struct CurrentAmps(f32););
transparent_value_type!(pub struct BrakeCurrentAmps(f32););
transparent_value_type!(pub struct InputCurrentAmps(f32););
transparent_value_type!(pub struct Voltage(f32););

transparent_eq_value_type!(pub struct AppDataLen(u32););
transparent_eq_value_type!(pub struct UartBaudRate(u32););
transparent_eq_value_type!(pub struct UartWriteLen(u32););
transparent_eq_value_type!(pub struct MotorIndex(i32););
transparent_eq_value_type!(pub struct CanControllerId(u8););
transparent_eq_value_type!(pub struct CanFrameLen(u8););

transparent_value_type_generic!(pub struct AppDataPacket<'a>(&'a [u8]););

impl<'a> AppDataPacket<'a> {
    pub const fn as_bytes(self) -> &'a [u8] {
        self.get()
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

transparent_value_type_generic!(pub struct CommandPacket<'a>(&'a [u8]););

impl<'a> CommandPacket<'a> {
    pub const fn as_bytes(self) -> &'a [u8] {
        self.get()
    }
}

transparent_value_type_generic!(pub struct ReplyPacket<'a>(&'a [u8]););

impl<'a> ReplyPacket<'a> {
    pub const fn as_bytes(self) -> &'a [u8] {
        self.get()
    }
}

transparent_value_type!(pub struct HalfDuplex(bool););

impl HalfDuplex {
    pub const fn is_enabled(self) -> bool {
        self.0
    }
}

transparent_value_type!(pub struct TemperatureC(f32););
transparent_value_type!(pub struct DistanceMeters(f32););
transparent_value_type!(pub struct SpeedMetersPerSecond(f32););
transparent_value_type!(pub struct AmpHours(f32););
transparent_value_type!(pub struct WattHours(f32););
transparent_value_type!(pub struct OdometerMeters(u64););
transparent_value_type!(pub struct BatteryLevel(f32););
transparent_value_type!(pub struct FocChannel(i32););
transparent_value_type!(pub struct ToneFrequencyHz(f32););
transparent_value_type!(pub struct ToneVoltage(f32););
transparent_value_type!(pub struct OffDelaySeconds(f32););
transparent_value_type!(pub struct LatitudeDeg(f64););
transparent_value_type!(pub struct LongitudeDeg(f64););
transparent_value_type!(pub struct AltitudeMeters(f32););
transparent_value_type!(pub struct GnssSpeed(f32););
transparent_value_type!(pub struct Hdop(f32););

transparent_value_type!(pub struct Accel3([f32; 3]););
transparent_value_type!(pub struct Gyro3([f32; 3]););
transparent_value_type!(pub struct Mag3([f32; 3]););
transparent_value_type!(pub struct Quaternion([f32; 4]););
transparent_value_type!(pub struct ImuCalibration([f32; 3]););
transparent_value_type!(pub struct ReadCallbackDtSeconds(f32););
transparent_eq_value_type!(pub struct CfgParam(i32););
transparent_value_type!(pub struct CfgFloat(f32););
transparent_eq_value_type!(pub struct CfgInt(i32););
transparent_eq_value_type!(pub struct ConfigSetResult(i32););
transparent_value_type_generic!(pub struct ConfigXmlBytes<'a>(&'a [u8]););
transparent_value_type_generic!(pub struct ConfigPayload<'a>(&'a [u8]););
transparent_value_type_generic!(pub struct ThreadName<'a>(&'a CStr););
transparent_eq_value_type!(pub struct StackSizeBytes(usize););
transparent_eq_value_type!(pub struct ThreadHandle(core::ptr::NonNull<c_void>););
transparent_eq_value_type!(pub struct MutexHandle(core::ptr::NonNull<c_void>););
transparent_eq_value_type!(pub struct SemaphoreHandle(core::ptr::NonNull<c_void>););
transparent_eq_value_type_type!(pub struct FirmwarePtr<T>(core::ptr::NonNull<T>););
transparent_eq_value_type_type!(pub struct FirmwareNonNull<T>(core::ptr::NonNull<T>););
transparent_eq_value_type!(pub struct MallocLen(usize););
transparent_eq_value_type_type!(pub struct OwnedFirmwareAllocation<T>(core::ptr::NonNull<T>););
transparent_eq_value_type_generic!(pub struct CanPayload<'a>(&'a [u8]););
transparent_eq_value_type!(pub struct CanStatusIndex(i32););
transparent_eq_value_type!(pub struct HardwareType(i32););
transparent_value_type!(pub struct RollDeg(f32););
transparent_value_type!(pub struct PitchDeg(f32););
transparent_value_type!(pub struct YawDeg(f32););

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

    pub const fn roll(self) -> RollDeg {
        self.roll
    }

    pub const fn pitch(self) -> PitchDeg {
        self.pitch
    }

    pub const fn yaw(self) -> YawDeg {
        self.yaw
    }
}

transparent_value_type_generic!(pub struct PlotAxisName<'a>(&'a CStr););
transparent_value_type_generic!(pub struct PlotGraphName<'a>(&'a CStr););
transparent_eq_value_type!(pub struct PlotGraphIndex(i32););

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

    pub const fn x(self) -> f32 {
        self.x
    }

    pub const fn y(self) -> f32 {
        self.y
    }
}

transparent_eq_value_type!(pub struct VescPin(i32););
transparent_eq_value_type!(pub struct VescPinMode(i32););
transparent_eq_value_type!(pub struct GpioPortPtr(core::ptr::NonNull<c_void>););
transparent_eq_value_type!(pub struct GpioPin(u32););
transparent_value_type!(pub struct AnalogVoltage(f32););
transparent_value_type!(pub struct AnalogRaw(f32););
transparent_eq_value_type!(pub struct LbmIoSymbol(LbmSymbol););
transparent_eq_value_type!(pub struct NvmAddress(u32););
transparent_eq_value_type!(pub struct NvmLen(u32););
transparent_value_type_generic!(pub struct NvmBytes<'a>(&'a [u8]););

transparent_eq_value_type!(pub struct EepromAddress(i32););
transparent_eq_value_type!(pub struct EepromVar(i32););

pub type Value = LbmValue;
pub type Count = LbmCount;
pub type Int = LbmInt;
pub type Uint = LbmUint;
pub type TypeCode = LbmType;
pub type ChannelId = LbmCid;
pub type Float = LbmFloat;
pub type Symbol = LbmSymbol;
pub type ErrorSymbol = LbmErrorSymbol;
pub type BoolSymbol = LbmBoolSymbol;
pub type NilSymbol = LbmNilSymbol;
pub type Address = ProgramAddress;
pub type BaseAddress = LoaderBaseAddress;
pub type MillisecondsU32 = Milliseconds;
pub type MicrosecondsU32 = Microseconds;
pub type Packet<'a> = AppDataPacket<'a>;
pub type Command<'a> = CommandPacket<'a>;
pub type Reply<'a> = ReplyPacket<'a>;
pub type Half = HalfDuplex;
pub type ThermodynamicTemperature = TemperatureC;
pub type Length = DistanceMeters;
pub type Velocity = SpeedMetersPerSecond;
pub type ElectricCurrent = CurrentAmps;
pub type ElectricPotential = Voltage;
pub type ElectricCharge = AmpHours;
pub type Energy = WattHours;
pub type Odometer = OdometerMeters;
pub type Battery = BatteryLevel;
pub type Channel = FocChannel;
pub type Frequency = ToneFrequencyHz;
pub type Time = OffDelaySeconds;
pub type Tone = ToneVoltage;
pub type Delay = Time;
pub type Latitude = LatitudeDeg;
pub type Longitude = LongitudeDeg;
pub type Altitude = AltitudeMeters;
pub type HdopValue = Hdop;
pub type Vector3 = Accel3;
pub type AngularVelocity = Gyro3;
pub type MagneticFluxDensity = Mag3;
pub type Quaternion4 = Quaternion;
pub type Calibration = ImuCalibration;
pub type SamplePeriod = ReadCallbackDtSeconds;
pub type Parameter = CfgParam;
pub type ConfigFloat = CfgFloat;
pub type ConfigInt = CfgInt;
pub type ConfigResult = ConfigSetResult;
pub type XmlBytes<'a> = ConfigXmlBytes<'a>;
pub type Payload<'a> = ConfigPayload<'a>;
pub type ThreadLabel<'a> = ThreadName<'a>;
pub type StackSize = StackSizeBytes;
pub type ThreadHandleValue = ThreadHandle;
pub type MutexHandleValue = MutexHandle;
pub type SemaphoreHandleValue = SemaphoreHandle;
pub type Thread = ThreadHandleValue;
pub type Mutex = MutexHandleValue;
pub type Semaphore = SemaphoreHandleValue;
pub type FirmwarePtrValue<T> = FirmwarePtr<T>;
pub type FirmwareNonNullValue<T> = FirmwareNonNull<T>;
pub type MallocSize = MallocLen;
pub type OwnedFirmware<T> = OwnedFirmwareAllocation<T>;
pub type AxisLabel<'a> = PlotAxisName<'a>;
pub type GraphLabel<'a> = PlotGraphName<'a>;
pub type GraphIndex = PlotGraphIndex;
pub type Point = PlotPoint;
pub type Bytes<'a> = NvmBytes<'a>;
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
    #[allow(unused_imports)]
    use super::{
        Accel3, AltitudeMeters, AnalogRaw, AnalogVoltage, AmpHours, AppDataLen, AppDataPacket,
        AngularVelocity, BatteryLevel, BrakeCurrentAmps, Bytes, Calibration, CanControllerId,
        CanFrameLen, CanPayload, CanStatusIndex, CfgFloat, CfgInt, CfgParam, CommandPacket,
        ConfigPayload, ConfigResult, ConfigXmlBytes, CurrentAmps, Degrees, DistanceMeters,
        DutyCycle, EepromAddress, EepromVar, Energy, Erpm, EulerAngles, ExtensionHandler,
        ElectricCurrent, ElectricPotential, FocChannel, FirmwareNonNull, FirmwarePtr, GnssSpeed,
        GpioPin, GpioPortPtr, HdopValue,
        HalfDuplex, HardwareType, ImageOffset, InputCurrentAmps, LatitudeDeg, LbmApi,
        LbmBindings, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt,
        LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, Length, LibInfo,
        LibInfoAbi, LoaderBaseAddress, LongitudeDeg, MallocLen, MagneticFluxDensity, MotorIndex,
        MutexHandle, MutablePacket, NativeAddress, NativeImage, NvmAddress, NvmBytes, NvmLen,
        OdometerMeters, OffDelaySeconds, OwnedFirmwareAllocation, PitchDeg, PlotAxisName,
        PlotGraphIndex, PlotGraphName, PlotPoint, ProgramAddress, Quaternion, Radians, ReplyPacket,
        RollDeg, SamplePeriod, SemaphoreHandle, SecondsF32, SpeedMetersPerSecond, StackSizeBytes,
        SystemSeconds, SystemTicks, TemperatureC, ThreadHandle, ThreadName, ToneFrequencyHz,
        ToneVoltage, UartBaudRate, UartWriteLen, Value, VescIfAbi, VescPin, VescPinMode,
        WattHours, YawDeg,
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
        assert_eq!(core::mem::size_of::<HdopValue>(), core::mem::size_of::<f32>());
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
        assert_eq!(
            core::mem::size_of::<ElectricPotential>(),
            core::mem::size_of::<f32>()
        );
        assert_eq!(core::mem::size_of::<CfgParam>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<CfgFloat>(), core::mem::size_of::<f32>());
        assert_eq!(core::mem::size_of::<CfgInt>(), core::mem::size_of::<i32>());
        assert_eq!(core::mem::size_of::<ConfigResult>(), core::mem::size_of::<i32>());
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

    #[test]
    fn wrapper_helpers_round_trip_their_inner_values() {
        let raw = [1_u8, 2, 3];
        let mut mut_raw = [4_u8, 5, 6];
        let name = c"axis";

        assert_eq!(LbmInt::new(-7).get(), -7);
        assert_eq!(LbmFloat::new(3.5).get(), 3.5);
        assert_eq!(HalfDuplex::new(true).is_enabled(), true);
        assert_eq!(Accel3::new([0.1, 0.2, 0.3]).get(), [0.1, 0.2, 0.3]);
        assert_eq!(Quaternion::new([1.0, 0.0, 0.0, 0.0]).get(), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(ConfigXmlBytes::new(&raw).get(), &raw);
        assert_eq!(ConfigPayload::new(&raw).get(), &raw);
        assert_eq!(ThreadName::new(name).get(), name);
        assert_eq!(CanPayload::new(&raw).get(), &raw);
        assert_eq!(PlotAxisName::new(name).get(), name);
        assert_eq!(PlotGraphName::new(name).get(), name);
        assert_eq!(NvmBytes::new(&raw).get(), &raw);
        {
            let mut packet = MutablePacket::new(&mut mut_raw);
            packet.as_mut_bytes()[0] = 9;
        }
        assert_eq!(mut_raw[0], 9);
        assert_eq!(PlotPoint::new(1.5, 2.5).x(), 1.5);
        assert_eq!(PlotPoint::new(1.5, 2.5).y(), 2.5);
        assert_eq!(
            EulerAngles::new(RollDeg::new(1.0), PitchDeg::new(2.0), YawDeg::new(3.0)).yaw(),
            YawDeg::new(3.0)
        );
    }

    #[test]
    fn normalized_wrappers_support_into_and_from_conversions() {
        let int_value: LbmInt = (-7_i32).into();
        let float_value: LbmFloat = 3.5_f32.into();
        let scalar_value: AppDataLen = 42_u32.into();
        let size_value: StackSizeBytes = 128_usize.into();
        let payload = [7_u8, 8, 9];
        let payload_value: AppDataPacket<'_> = (&payload[..]).into();
        let name = c"axis";
        let name_value: ThreadName<'_> = name.into();
        let axis_value: PlotAxisName<'_> = name.into();
        let can_payload: CanPayload<'_> = (&payload[..]).into();
        let config_payload: ConfigPayload<'_> = (&payload[..]).into();
        let xml_bytes: ConfigXmlBytes<'_> = (&payload[..]).into();
        let nvm_bytes: NvmBytes<'_> = (&payload[..]).into();
        let angle = RollDeg::new(90.0);
        let pitch = PitchDeg::new(45.0);
        let yaw = YawDeg::new(15.0);
        let half_duplex: HalfDuplex = true.into();
        let accel: Accel3 = [1.0, 2.0, 3.0].into();
        let quat: Quaternion = [1.0, 0.0, 0.0, 0.0].into();
        let graph_name: PlotGraphName<'_> = name.into();

        assert_eq!(int_value.get(), -7);
        assert_eq!(float_value.get(), 3.5);
        assert_eq!(scalar_value.get(), 42);
        assert_eq!(size_value.get(), 128);
        assert_eq!(payload_value.as_bytes(), &payload);
        assert_eq!(name_value.get(), name);
        assert_eq!(axis_value.get(), name);
        assert_eq!(can_payload.get(), &payload);
        assert_eq!(config_payload.get(), &payload);
        assert_eq!(xml_bytes.get(), &payload);
        assert_eq!(nvm_bytes.get(), &payload);
        assert_eq!(half_duplex.is_enabled(), true);
        assert_eq!(accel.get(), [1.0, 2.0, 3.0]);
        assert_eq!(quat.get(), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            EulerAngles::new(angle, pitch, yaw),
            EulerAngles::new(angle, pitch, yaw)
        );
        assert_eq!(graph_name.get(), name);
    }

    #[test]
    fn rustic_aliases_point_at_the_public_wrapper_surface() {
        let value: crate::Value = 1_u32.into();
        let speed: crate::Velocity = 2.5_f32.into();
        let bytes: crate::Bytes<'_> = (&[1_u8, 2, 3][..]).into();

        assert_eq!(value.get(), 1);
        assert_eq!(speed.get(), 2.5);
        assert_eq!(bytes.get(), &[1, 2, 3]);
    }

    #[test]
    fn unit_style_metric_aliases_cover_the_public_quantity_surface() {
        let temperature: crate::ThermodynamicTemperature = 21.5_f32.into();
        let distance: crate::Length = 12.0_f32.into();
        let speed: crate::Velocity = 3.25_f32.into();
        let latitude: crate::Latitude = 45.0_f64.into();
        let longitude: crate::Longitude = crate::Longitude::new(-110.0_f64);
        let altitude: crate::Altitude = 1800.0_f32.into();
        let voltage: crate::ElectricPotential = 50.5_f32.into();
        let current: crate::ElectricCurrent = 7.25_f32.into();
        let charge: crate::ElectricCharge = 1.5_f32.into();
        let energy: crate::Energy = 10.0_f32.into();
        let omega: crate::AngularVelocity = [0.1, 0.2, 0.3].into();
        let magnetic: crate::MagneticFluxDensity = [4.0, 5.0, 6.0].into();
        let calibration: crate::Calibration = [0.0, 1.0, 2.0].into();
        let gnss_speed: crate::Velocity = 8.5_f32.into();
        let frequency: crate::Frequency = 1.0_f32.into();
        let tone_voltage: crate::Tone = 12.0_f32.into();
        let delay: crate::Delay = 3.0_f32.into();
        let sample_period: crate::SamplePeriod = 0.25_f32.into();

        assert_eq!(temperature.get(), 21.5);
        assert_eq!(distance.get(), 12.0);
        assert_eq!(speed.get(), 3.25);
        assert_eq!(latitude.get(), 45.0);
        assert_eq!(longitude.get(), -110.0);
        assert_eq!(altitude.get(), 1800.0);
        assert_eq!(voltage.get(), 50.5);
        assert_eq!(current.get(), 7.25);
        assert_eq!(charge.get(), 1.5);
        assert_eq!(energy.get(), 10.0);
        assert_eq!(omega.get(), [0.1, 0.2, 0.3]);
        assert_eq!(magnetic.get(), [4.0, 5.0, 6.0]);
        assert_eq!(calibration.get(), [0.0, 1.0, 2.0]);
        assert_eq!(gnss_speed.get(), 8.5);
        assert_eq!(frequency.get(), 1.0);
        assert_eq!(tone_voltage.get(), 12.0);
        assert_eq!(delay.get(), 3.0);
        assert_eq!(sample_period.get(), 0.25);
    }
}
