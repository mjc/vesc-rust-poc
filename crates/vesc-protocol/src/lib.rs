//! Typed `no_std` protocol values for host tools and device-side VESC packages.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.
//!
//! The normal API uses project-owned Rust types such as [`WireVersion`],
//! [`WireCommand`], and [`Frame`]. Primitive wire bytes are only exposed at
//! explicit encode/decode boundaries through standard conversions such as
//! [`TryFrom`] and [`From`].
//!
//! This crate owns the protocol/wire contract. Reusable physical units belong
//! in the current `vescpkg-rs-units` crate, and VESC-domain semantic values belong in
//! the `vescpkg-rs` package layer.

#![no_std]
#![forbid(unused_extern_crates)]
// Embedded package code has no unwinder or operator console. Reject explicit
// crash paths in production while still allowing ordinary assertions in tests.
#![cfg_attr(
    not(test),
    deny(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unwrap_used
    )
)]

#[cfg(test)]
extern crate std;

/// BLE loopback wire-format helpers and response handling.
pub mod ble_loopback;
/// VESC firmware buffer-compatible primitive encoders.
pub mod buffer;

/// Version tag carried by the shared loopback wire frame.
///
/// Keep this in sync with the device-side and host-side loopback handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireVersion(u8);

impl WireVersion {
    /// Current loopback wire protocol version.
    pub const CURRENT: Self = Self(1);

    /// Create a version tag from its wire value.
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Explicitly extract the primitive wire value.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<WireVersion> for u8 {
    fn from(version: WireVersion) -> Self {
        version.get()
    }
}

/// Error returned when a primitive command code is not recognized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidWireCommand(u8);

impl InvalidWireCommand {
    /// Create an invalid command marker from the rejected wire value.
    #[must_use]
    pub const fn new(code: u8) -> Self {
        Self(code)
    }

    /// Explicitly extract the rejected primitive command code.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<InvalidWireCommand> for u8 {
    fn from(command: InvalidWireCommand) -> Self {
        command.get()
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
    const fn wire_value(self) -> u8 {
        match self {
            Self::Ping => 1,
            Self::Echo => 2,
            Self::Status => 3,
            Self::Teardown => 4,
        }
    }
}

impl TryFrom<u8> for WireCommand {
    type Error = InvalidWireCommand;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(Self::Ping),
            2 => Ok(Self::Echo),
            3 => Ok(Self::Status),
            4 => Ok(Self::Teardown),
            _ => Err(InvalidWireCommand::new(code)),
        }
    }
}

impl From<WireCommand> for u8 {
    fn from(command: WireCommand) -> Self {
        command.wire_value()
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
    #[must_use]
    pub const fn new(version: WireVersion, command: WireCommand, payload: &'a [u8]) -> Self {
        Self {
            version,
            command,
            payload,
        }
    }

    /// Return the frame version.
    #[must_use]
    pub const fn version(&self) -> WireVersion {
        self.version
    }

    /// Return the frame command.
    #[must_use]
    pub const fn command(&self) -> WireCommand {
        self.command
    }

    /// Return the borrowed payload bytes.
    #[must_use]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, WireCommand, WireVersion};

    #[test]
    fn exposes_a_stable_current_version() {
        assert_eq!(WireVersion::CURRENT.get(), 1);
        assert_eq!(u8::from(WireVersion::CURRENT), 1);
    }

    #[test]
    fn maps_command_codes_round_trip() {
        assert_eq!(u8::from(WireCommand::Ping), 1);
        assert_eq!(WireCommand::try_from(4), Ok(WireCommand::Teardown));
        assert_eq!(
            WireCommand::try_from(99).map_err(super::InvalidWireCommand::get),
            Err(99)
        );
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
