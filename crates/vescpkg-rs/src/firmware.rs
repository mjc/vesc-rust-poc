//! Safe adapters for firmware callback pointers.

use core::ffi::{c_int, c_void};
use core::ptr::{self, NonNull};

use vescpkg_rs_sys::{
    AppDataPacket, ConfigPayload, ConfigXmlBytes, LbmValue, LibInfo, MutablePacket, StopHandler,
};

/// Borrow nullable loader metadata from a firmware entrypoint.
pub fn loader_info_mut(info: *mut LibInfo) -> Option<&'static mut LibInfo> {
    NonNull::new(info).map(|mut info| unsafe { info.as_mut() })
}

/// Clear package state and stop metadata when startup fails.
pub fn clear_loader_info(info: *mut LibInfo) {
    if let Some(info) = loader_info_mut(info) {
        info.arg = ptr::null_mut();
        info.stop_fun = None;
    }
}

/// Store package state and a stop hook in loader metadata.
pub fn install_loader_state<T>(
    info: *mut LibInfo,
    stop_handler: StopHandler,
    state: &mut T,
) -> bool {
    if let Some(info) = loader_info_mut(info) {
        info.arg = ptr::from_mut(state).cast();
        info.stop_fun = Some(stop_handler);
    }
    true
}

/// Recover typed package state from loader metadata.
pub fn loader_state_mut<T: 'static>(info: &mut LibInfo) -> Option<&mut T> {
    arg_mut(info.arg)
}

/// Recover typed package state from a firmware ARG pointer.
pub fn arg_mut<T: 'static>(arg: *mut c_void) -> Option<&'static mut T> {
    NonNull::new(arg.cast::<T>()).map(|mut arg| unsafe { arg.as_mut() })
}

/// Borrow typed package state from a firmware ARG pointer.
pub fn arg_ref<T: 'static>(arg: *mut c_void) -> Option<&'static T> {
    NonNull::new(arg.cast::<T>()).map(|arg| unsafe { arg.as_ref() })
}

/// Convert firmware app-data callback arguments into a packet view.
pub fn app_data_packet(data: *mut u8, len: u32) -> Option<AppDataPacket<'static>> {
    let len = usize::try_from(len).ok()?;
    borrowed_bytes(data.cast_const(), len).map(AppDataPacket)
}

/// Convert LispBM extension callback arguments into typed values.
pub fn lbm_args(args: *mut u32, argn: u32) -> Option<&'static [LbmValue]> {
    let len = usize::try_from(argn).ok()?;
    let args = NonNull::new(args.cast::<LbmValue>())?;
    Some(unsafe { core::slice::from_raw_parts(args.as_ptr().cast_const(), len) })
}

/// Mutable custom-config output buffer for `get_cfg` callbacks.
pub struct CustomConfigGetBuffer(MutablePacket<'static>);

impl CustomConfigGetBuffer {
    /// Borrow the firmware-provided output buffer.
    pub fn new(buffer: *mut u8, len: usize) -> Option<Self> {
        mutable_bytes(buffer, len).map(Self)
    }

    /// Write serialized config bytes into the firmware-provided output buffer.
    pub fn write(&mut self, payload: ConfigPayload<'_>) -> c_int {
        if payload.0.len() > self.0.0.len() {
            return 0;
        }
        self.0.0[..payload.0.len()].copy_from_slice(payload.0);
        payload.0.len() as c_int
    }
}

/// Borrowed custom-config input for `set_cfg` callbacks.
pub fn custom_config_payload(buffer: *mut u8, len: usize) -> Option<ConfigPayload<'static>> {
    borrowed_bytes(buffer.cast_const(), len).map(ConfigPayload)
}

/// Firmware XML pointer output for `get_cfg_xml` callbacks.
pub struct CustomConfigXmlOut(NonNull<*mut u8>);

impl CustomConfigXmlOut {
    /// Borrow the firmware-provided XML output pointer slot.
    pub fn new(buffer: *mut *mut u8) -> Option<Self> {
        NonNull::new(buffer).map(Self)
    }

    /// Return XML bytes to firmware by pointer and byte count.
    pub fn return_xml(self, xml: ConfigXmlBytes<'static>) -> c_int {
        unsafe { *self.0.as_ptr() = xml.0.as_ptr().cast_mut() };
        xml.0.len() as c_int
    }
}

/// Borrow XML bytes from a rebased firmware image address.
pub fn config_xml_bytes(data: *const u8, len: usize) -> Option<ConfigXmlBytes<'static>> {
    borrowed_bytes(data, len).map(ConfigXmlBytes)
}

/// Copy a fixed-size firmware array into Rust-owned storage.
pub fn firmware_array<T: Copy, const N: usize>(values: *const T) -> Option<[T; N]> {
    let values = NonNull::new(values.cast_mut())?;
    let values = unsafe { core::slice::from_raw_parts(values.as_ptr().cast_const(), N) };
    values.try_into().ok()
}

fn borrowed_bytes(data: *const u8, len: usize) -> Option<&'static [u8]> {
    let data = NonNull::new(data.cast_mut())?;
    Some(unsafe { core::slice::from_raw_parts(data.as_ptr().cast_const(), len) })
}

fn mutable_bytes(data: *mut u8, len: usize) -> Option<MutablePacket<'static>> {
    let data = NonNull::new(data)?;
    Some(MutablePacket(unsafe {
        core::slice::from_raw_parts_mut(data.as_ptr(), len)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn stop(_arg: *mut c_void) {}

    #[test]
    fn loader_state_round_trips_through_loader_info() {
        let mut state = 42_u32;
        let mut info = LibInfo {
            stop_fun: None,
            arg: ptr::null_mut(),
            base_addr: 0,
        };

        assert!(install_loader_state(&mut info, stop, &mut state));
        *loader_state_mut::<u32>(&mut info).expect("state") = 7;

        assert_eq!(state, 7);
        assert!(info.stop_fun.is_some());

        clear_loader_info(&mut info);
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn custom_config_get_buffer_writes_payload_bytes() {
        let mut buffer = [0_u8; 3];
        let mut output =
            CustomConfigGetBuffer::new(buffer.as_mut_ptr(), buffer.len()).expect("config output");

        assert_eq!(output.write(ConfigPayload(b"abc")), 3);
        assert_eq!(&buffer, b"abc");
        assert!(CustomConfigGetBuffer::new(ptr::null_mut(), 3).is_none());
    }

    #[test]
    fn firmware_packet_and_array_helpers_reject_null() {
        assert!(app_data_packet(ptr::null_mut(), 3).is_none());
        assert!(firmware_array::<f32, 3>(ptr::null()).is_none());

        let values = [1.0_f32, 2.0, 3.0];
        assert_eq!(firmware_array::<f32, 3>(values.as_ptr()), Some(values));
    }

    #[test]
    fn custom_config_xml_out_writes_pointer_and_len() {
        let xml = b"<xml/>";
        let mut out = ptr::null_mut();
        let output = CustomConfigXmlOut::new(&mut out).expect("xml output");

        assert_eq!(output.return_xml(ConfigXmlBytes(xml)), 6);
        assert_eq!(out, xml.as_ptr().cast_mut());
        assert!(CustomConfigXmlOut::new(ptr::null_mut()).is_none());
    }
}
