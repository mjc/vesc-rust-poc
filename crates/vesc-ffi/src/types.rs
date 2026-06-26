use core::ffi::c_void;

macro_rules! transparent_value_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        $vis struct $name(pub $inner);
    };
}

macro_rules! transparent_eq_value_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis struct $name(pub $inner);
    };
}

macro_rules! transparent_eq_value_type_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident<$ty:ident>($inner:ty);) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis struct $name<$ty>(pub $inner);
    };
}

transparent_eq_value_type!(
    pub struct LbmValue(u32);
);
transparent_eq_value_type!(
    pub struct LbmCount(u32);
);
transparent_eq_value_type!(
    pub struct LbmInt(i32);
);
transparent_eq_value_type!(
    pub struct LbmUint(u32);
);
transparent_eq_value_type!(
    pub struct LbmType(u32);
);
transparent_eq_value_type!(
    pub struct LbmCid(u32);
);
transparent_value_type!(
    pub struct LbmFloat(f32);
);
transparent_eq_value_type!(
    pub struct LbmSymbol(u32);
);
transparent_eq_value_type!(
    pub struct LbmErrorSymbol(u32);
);
transparent_eq_value_type!(
    pub struct LbmBoolSymbol(u32);
);
transparent_eq_value_type!(
    pub struct LbmNilSymbol(u32);
);
transparent_eq_value_type!(
    pub struct ProgramAddress(u32);
);
transparent_eq_value_type!(
    pub struct LoaderBaseAddress(u32);
);
transparent_eq_value_type!(
    pub struct SystemTicks(u32);
);

transparent_eq_value_type!(
    pub struct AppDataLen(u32);
);
transparent_eq_value_type!(
    pub struct UartBaudRate(u32);
);
transparent_eq_value_type!(
    pub struct UartWriteLen(u32);
);
transparent_eq_value_type!(
    pub struct MotorIndex(i32);
);
transparent_eq_value_type!(
    pub struct CanControllerId(u8);
);
transparent_eq_value_type!(
    pub struct CanFrameLen(u8);
);

transparent_value_type!(
    pub struct HalfDuplex(bool);
);

transparent_eq_value_type!(
    pub struct CfgParam(i32);
);
transparent_value_type!(
    pub struct CfgFloat(f32);
);
transparent_eq_value_type!(
    pub struct CfgInt(i32);
);
transparent_eq_value_type!(
    pub struct ConfigSetResult(i32);
);
transparent_eq_value_type!(
    pub struct StackSizeBytes(usize);
);
transparent_eq_value_type!(
    pub struct ThreadHandle(core::ptr::NonNull<c_void>);
);
transparent_eq_value_type!(
    pub struct MutexHandle(core::ptr::NonNull<c_void>);
);
transparent_eq_value_type!(
    pub struct SemaphoreHandle(core::ptr::NonNull<c_void>);
);
transparent_eq_value_type_type!(
    pub struct FirmwarePtr<T>(core::ptr::NonNull<T>);
);
transparent_eq_value_type_type!(
    pub struct FirmwareNonNull<T>(core::ptr::NonNull<T>);
);
transparent_eq_value_type!(
    pub struct MallocLen(usize);
);
transparent_eq_value_type_type!(
    pub struct OwnedFirmwareAllocation<T>(core::ptr::NonNull<T>);
);
transparent_eq_value_type!(
    pub struct CanStatusIndex(i32);
);
transparent_eq_value_type!(
    pub struct HardwareType(i32);
);

transparent_eq_value_type!(
    pub struct PlotGraphIndex(i32);
);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub x: f32,
    pub y: f32,
}

transparent_eq_value_type!(
    pub struct VescPin(i32);
);
transparent_eq_value_type!(
    pub struct VescPinMode(i32);
);
transparent_eq_value_type!(
    pub struct GpioPortPtr(core::ptr::NonNull<c_void>);
);
transparent_eq_value_type!(
    pub struct GpioPin(u32);
);
transparent_eq_value_type!(
    pub struct LbmIoSymbol(LbmSymbol);
);
transparent_eq_value_type!(
    pub struct NvmAddress(u32);
);
transparent_eq_value_type!(
    pub struct NvmLen(u32);
);

transparent_eq_value_type!(
    pub struct EepromAddress(i32);
);
transparent_eq_value_type!(
    pub struct EepromVar(i32);
);

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
pub(crate) struct TerminalCommandName<'a>(pub &'a core::ffi::CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct TerminalHelp<'a>(pub &'a core::ffi::CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct TerminalArgNames<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct TerminalCallback(pub core::ptr::NonNull<c_void>);

