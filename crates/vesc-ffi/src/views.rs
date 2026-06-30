use core::ffi::CStr;

/// Borrowed application data bytes handed to or from the firmware.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppDataPacket<'a>(pub &'a [u8]);

/// Mutable application data buffer provided by firmware callbacks.
#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct MutablePacket<'a>(pub &'a mut [u8]);

/// Borrowed command payload bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandPacket<'a>(pub &'a [u8]);

/// Borrowed reply payload bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplyPacket<'a>(pub &'a [u8]);

/// Borrowed XML configuration bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigXmlBytes<'a>(pub &'a [u8]);

/// Borrowed configuration payload bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfigPayload<'a>(pub &'a [u8]);

/// NUL-terminated firmware thread name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreadName<'a>(pub &'a CStr);

/// NUL-terminated plot axis name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotAxisName<'a>(pub &'a CStr);

/// NUL-terminated plot graph name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotGraphName<'a>(pub &'a CStr);

/// Borrowed CAN payload bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanPayload<'a>(pub &'a [u8]);

/// Borrowed nonvolatile-memory bytes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NvmBytes<'a>(pub &'a [u8]);
