//! Owned command-packet reply callbacks.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

const MAX_COMMAND_PACKET: usize = 512;
static COMMAND_REPLY_OWNED: AtomicBool = AtomicBool::new(false);

/// Failure returned by command reply processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandError {
    /// The command processor slot is not available.
    Unavailable,
    /// The command packet exceeds the firmware payload limit.
    PacketTooLong,
    /// Another command reply callback already owns the firmware slot.
    Busy,
}

/// Safe callback behavior for one command reply.
pub trait CommandReplyHandler {
    /// Handle a firmware-owned reply through a scoped slice.
    fn reply(data: &[u8]);
}

/// Optional command-processing capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Commands;

/// RAII lease for a registered command reply callback.
pub struct CommandReplyLease<H: CommandReplyHandler> {
    _handler: PhantomData<H>,
}

impl Commands {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Process a command packet and retain its reply callback until drop.
    pub fn process<H: CommandReplyHandler>(
        &self,
        packet: &mut [u8],
    ) -> Result<CommandReplyLease<H>, CommandError> {
        if packet.len() > MAX_COMMAND_PACKET {
            return Err(CommandError::PacketTooLong);
        }
        if COMMAND_REPLY_OWNED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(CommandError::Busy);
        }
        let registered = unsafe {
            crate::ffi::commands_process_packet(
                packet.as_mut_ptr(),
                packet.len() as u32,
                reply::<H>,
            )
        };
        if !registered {
            COMMAND_REPLY_OWNED.store(false, Ordering::Release);
            return Err(CommandError::Unavailable);
        }
        Ok(CommandReplyLease {
            _handler: PhantomData,
        })
    }
}

impl<H: CommandReplyHandler> Drop for CommandReplyLease<H> {
    fn drop(&mut self) {
        // Keep the ownership bit set if the optional cleanup slot is absent;
        // replacing a callback we could not unregister would be unsafe.
        if unsafe { crate::ffi::commands_unregister_reply_func(reply::<H>) } {
            COMMAND_REPLY_OWNED.store(false, Ordering::Release);
        }
    }
}

unsafe extern "C" fn reply<H: CommandReplyHandler>(data: *mut u8, len: u32) {
    let len = len as usize;
    if data.is_null() || len > MAX_COMMAND_PACKET {
        return;
    }
    H::reply(unsafe { core::slice::from_raw_parts(data, len) });
}

impl crate::Firmware {
    /// Return the optional command-processing capability handle.
    pub fn commands(&self) -> Commands {
        Commands::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional command-processing capability handle.
    pub fn commands(&self) -> Commands {
        Commands::new()
    }
}
