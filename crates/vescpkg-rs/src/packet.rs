//! Owned packet framing around the concrete VESC `PACKET_STATE_t` layout.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use vescpkg_rs_sys::raw::{PACKET_BUFFER_LEN, PACKET_MAX_PL_LEN, PacketState};

const MAX_PACKET_BYTES: usize = PACKET_MAX_PL_LEN;

static PACKET_CODEC_REGISTERED: AtomicBool = AtomicBool::new(false);
static PACKET_CODEC_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn disable_callback_dispatch() {
    PACKET_CODEC_ACTIVE.store(false, Ordering::Release);
}

/// Failure returned by packet framing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PacketError {
    /// The loaded firmware does not expose the packet slot.
    Unavailable,
    /// A packet exceeds the pinned firmware framing buffer.
    PacketTooLong,
    /// Another packet codec already owns the global firmware callback.
    Busy,
}

/// Safe callback behavior for a packet codec.
pub trait PacketHandler {
    /// Receive one complete packet payload copied into a scoped slice.
    fn send(data: &[u8]);
    /// Handle one received packet payload copied into a scoped slice.
    fn process(data: &[u8]);
}

/// Firmware packet state owned by the package while registered.
pub struct PacketCodec<H: PacketHandler> {
    state: PacketState,
    _handler: PhantomData<H>,
}

/// Active packet registration borrowing its codec and firmware-owned state.
pub struct PacketRegistration<'a, H: PacketHandler> {
    codec: &'a mut PacketCodec<H>,
}

impl<H: PacketHandler> PacketCodec<H> {
    /// Construct an unregistered packet codec with zeroed framing state.
    pub const fn new() -> Self {
        Self {
            state: PacketState {
                send_func: None,
                process_func: None,
                rx_read_ptr: 0,
                rx_write_ptr: 0,
                bytes_left: 0,
                rx_buffer: [0; PACKET_BUFFER_LEN],
                tx_buffer: [0; PACKET_BUFFER_LEN],
            },
            _handler: PhantomData,
        }
    }

    /// Register the package-owned state and typed callback trampolines.
    pub fn register(&mut self) -> Result<PacketRegistration<'_, H>, PacketError> {
        if PACKET_CODEC_REGISTERED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(PacketError::Busy);
        }
        let registered = unsafe {
            crate::ffi::packet_init(packet_send::<H>, packet_process::<H>, &mut self.state)
        };
        if registered {
            PACKET_CODEC_ACTIVE.store(true, Ordering::Release);
            Ok(PacketRegistration { codec: self })
        } else {
            PACKET_CODEC_REGISTERED.store(false, Ordering::Release);
            Err(PacketError::Unavailable)
        }
    }
}

impl<H: PacketHandler> Default for PacketCodec<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: PacketHandler> PacketRegistration<'_, H> {
    /// Feed one byte into the registered framing state.
    pub fn process_byte(&mut self, byte: u8) -> Result<(), PacketError> {
        unsafe { crate::ffi::packet_process_byte(byte, &mut self.codec.state) }
            .then_some(())
            .ok_or(PacketError::Unavailable)
    }

    /// Send one bounded packet payload through the registered framing state.
    pub fn send_packet(&mut self, data: &mut [u8]) -> Result<(), PacketError> {
        if data.len() > MAX_PACKET_BYTES {
            return Err(PacketError::PacketTooLong);
        }
        unsafe {
            crate::ffi::packet_send_packet(
                data.as_mut_ptr(),
                data.len() as u32,
                &mut self.codec.state,
            )
        }
        .then_some(())
        .ok_or(PacketError::Unavailable)
    }
}

impl<H: PacketHandler> Drop for PacketRegistration<'_, H> {
    fn drop(&mut self) {
        PACKET_CODEC_ACTIVE.store(false, Ordering::Release);
        let _ = unsafe { crate::ffi::packet_reset(&mut self.codec.state) };
        PACKET_CODEC_REGISTERED.store(false, Ordering::Release);
    }
}

unsafe extern "C" fn packet_send<H: PacketHandler>(data: *mut u8, len: u32) {
    if !PACKET_CODEC_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let len = len as usize;
    if data.is_null() || len > MAX_PACKET_BYTES {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    H::send(data);
}

unsafe extern "C" fn packet_process<H: PacketHandler>(data: *mut u8, len: u32) {
    if !PACKET_CODEC_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let len = len as usize;
    if data.is_null() || len > MAX_PACKET_BYTES {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    H::process(data);
}

#[cfg(test)]
mod tests {
    use super::{PACKET_CODEC_ACTIVE, PacketHandler, packet_process, packet_send};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static SEND_CALLS: AtomicUsize = AtomicUsize::new(0);
    static PROCESS_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct Handler;

    impl PacketHandler for Handler {
        fn send(_data: &[u8]) {
            SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        }

        fn process(_data: &[u8]) {
            PROCESS_CALLS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn late_packet_callbacks_after_drop_fail_closed() {
        SEND_CALLS.store(0, Ordering::Relaxed);
        PROCESS_CALLS.store(0, Ordering::Relaxed);
        PACKET_CODEC_ACTIVE.store(true, Ordering::Release);
        let mut data = [1, 2];
        unsafe {
            packet_send::<Handler>(data.as_mut_ptr(), data.len() as u32);
            packet_process::<Handler>(data.as_mut_ptr(), data.len() as u32);
        }
        PACKET_CODEC_ACTIVE.store(false, Ordering::Release);
        unsafe {
            packet_send::<Handler>(data.as_mut_ptr(), data.len() as u32);
            packet_process::<Handler>(data.as_mut_ptr(), data.len() as u32);
        }
        assert_eq!(SEND_CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(PROCESS_CALLS.load(Ordering::Relaxed), 1);
    }
}
