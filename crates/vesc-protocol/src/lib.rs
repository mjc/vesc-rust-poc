//! Shared wire types for device-side VESC packages.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

pub mod ble_loopback;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireVersion(u8);

impl WireVersion {
    pub const CURRENT: Self = Self(1);

    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireCommand {
    Ping,
    Echo,
    Status,
    Teardown,
}

impl WireCommand {
    pub const fn code(self) -> u8 {
        match self {
            Self::Ping => 1,
            Self::Echo => 2,
            Self::Status => 3,
            Self::Teardown => 4,
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    version: WireVersion,
    command: WireCommand,
    payload: &'a [u8],
}

impl<'a> Frame<'a> {
    pub const fn new(version: WireVersion, command: WireCommand, payload: &'a [u8]) -> Self {
        Self {
            version,
            command,
            payload,
        }
    }

    pub const fn version(&self) -> WireVersion {
        self.version
    }

    pub const fn command(&self) -> WireCommand {
        self.command
    }

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
