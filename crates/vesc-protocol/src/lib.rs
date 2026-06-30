//! Shared no_std wire types for host tools and device-side VESC packages.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

/// BLE loopback wire-format helpers and response handling.
pub mod ble_loopback;

/// Version tag carried by the shared loopback wire frame.
///
/// Keep this in sync with the device-side and host-side loopback handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireVersion(u8);

impl WireVersion {
    /// Current loopback wire protocol version.
    pub const CURRENT: Self = Self(1);

    /// Create a version tag from its raw wire value.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// Return the raw version byte as it appears on the wire.
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// Command codes understood by the shared loopback wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireCommand {
    /// Request a no-op round trip.
    Ping,
    /// Echo the payload bytes back to the caller.
    Echo,
    /// Return status data for the current loopback endpoint.
    Status,
    /// Tear down the loopback session.
    Teardown,
}

impl WireCommand {
    /// Map the command to its wire code.
    pub const fn code(self) -> u8 {
        match self {
            Self::Ping => 1,
            Self::Echo => 2,
            Self::Status => 3,
            Self::Teardown => 4,
        }
    }

    /// Decode a wire command code.
    ///
    /// Unknown codes return `None` so callers can reject frames explicitly.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Ping),
            2 => Some(Self::Echo),
            3 => Some(Self::Status),
            4 => Some(Self::Teardown),
            _ => None,
        }
    }
}

/// A borrowed loopback frame with a version, command, and payload slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Wire protocol version for this frame.
    version: WireVersion,
    /// Command carried by this frame.
    command: WireCommand,
    /// Borrowed payload bytes.
    payload: &'a [u8],
}

impl<'a> Frame<'a> {
    /// Construct a frame from its parts.
    pub const fn new(version: WireVersion, command: WireCommand, payload: &'a [u8]) -> Self {
        Self {
            version,
            command,
            payload,
        }
    }

    /// Return the frame version.
    pub const fn version(&self) -> WireVersion {
        self.version
    }

    /// Return the frame command.
    pub const fn command(&self) -> WireCommand {
        self.command
    }

    /// Return the borrowed payload bytes.
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, WireCommand, WireVersion};

    #[test]
    fn exposes_a_stable_current_version() {
        assert_eq!(WireVersion::CURRENT.raw(), 1);
    }

    #[test]
    fn maps_command_codes_round_trip() {
        assert_eq!(WireCommand::Ping.code(), 1);
        assert_eq!(WireCommand::from_code(4), Some(WireCommand::Teardown));
        assert_eq!(WireCommand::from_code(99), None);
    }

    #[test]
    fn carries_payload_by_slice() {
        let payload = [1_u8, 2, 3];
        let frame = Frame::new(WireVersion::CURRENT, WireCommand::Echo, &payload);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Echo);
        assert_eq!(frame.payload(), &payload);
    }
}
