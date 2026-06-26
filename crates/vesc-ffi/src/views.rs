use core::ffi::CStr;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppDataPacket<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, PartialEq)]
pub struct MutablePacket<'a>(pub &'a mut [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandPacket<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplyPacket<'a>(pub &'a [u8]);

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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotAxisName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotGraphName<'a>(pub &'a CStr);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanPayload<'a>(pub &'a [u8]);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NvmBytes<'a>(pub &'a [u8]);
