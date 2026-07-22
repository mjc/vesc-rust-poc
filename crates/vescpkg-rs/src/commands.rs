//! Owned command-packet reply callbacks.

use core::marker::PhantomData;

const MAX_COMMAND_PACKET: usize = 512;

/// Failure returned by command reply processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandError {
    /// The command processor slot is not available.
    Unavailable,
    /// The command packet exceeds the firmware payload limit.
    PacketTooLong,
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
        let registered = unsafe {
            crate::ffi::commands_process_packet(
                packet.as_mut_ptr(),
                packet.len() as u32,
                reply::<H>,
            )
        };
        registered
            .then_some(CommandReplyLease {
                _handler: PhantomData,
            })
            .ok_or(CommandError::Unavailable)
    }
}

impl<H: CommandReplyHandler> Drop for CommandReplyLease<H> {
    fn drop(&mut self) {
        let _ = unsafe { crate::ffi::commands_unregister_reply_func(reply::<H>) };
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
