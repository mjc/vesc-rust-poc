use super::{Frame, WireCommand, WireVersion};
use core::fmt;

pub const BLE_LOOPBACK_PROTOCOL_VERSION: WireVersion = WireVersion::CURRENT;
pub const MAX_LOOPBACK_PAYLOAD_BYTES: usize = 16;
pub const MIN_WIRE_FRAME_BYTES: usize = 3;
pub const MAX_LOOPBACK_FRAME_BYTES: usize = MIN_WIRE_FRAME_BYTES + MAX_LOOPBACK_PAYLOAD_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackError {
    FrameTooShort,
    InvalidVersion {
        expected: WireVersion,
        actual: WireVersion,
    },
    InvalidCommand {
        code: u8,
    },
    PayloadTooLong {
        len: usize,
        max: usize,
    },
}

impl fmt::Display for LoopbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooShort => f.write_str("frame too short"),
            Self::InvalidVersion { expected, actual } => {
                write!(
                    f,
                    "invalid protocol version: expected {}, got {}",
                    expected.raw(),
                    actual.raw()
                )
            }
            Self::InvalidCommand { code } => write!(f, "invalid command code: {code}"),
            Self::PayloadTooLong { len, max } => {
                write!(f, "payload too long: {len} bytes (max {max})")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackPacket<'a> {
    frame: Frame<'a>,
}

impl<'a> LoopbackPacket<'a> {
    pub const fn new(command: WireCommand, payload: &'a [u8]) -> Result<Self, LoopbackError> {
        if payload.len() > MAX_LOOPBACK_PAYLOAD_BYTES {
            return Err(LoopbackError::PayloadTooLong {
                len: payload.len(),
                max: MAX_LOOPBACK_PAYLOAD_BYTES,
            });
        }

        Ok(Self {
            frame: Frame::new(BLE_LOOPBACK_PROTOCOL_VERSION, command, payload),
        })
    }

    pub fn frame(&self) -> Frame<'a> {
        self.frame.clone()
    }

    pub fn encode(
        &self,
    ) -> (
        [u8; MIN_WIRE_FRAME_BYTES + MAX_LOOPBACK_PAYLOAD_BYTES],
        usize,
    ) {
        let mut bytes = [0_u8; MIN_WIRE_FRAME_BYTES + MAX_LOOPBACK_PAYLOAD_BYTES];
        bytes[0] = self.frame.version().raw();
        bytes[1] = self.frame.command().code();
        bytes[2] = self.frame.payload().len() as u8;
        bytes[MIN_WIRE_FRAME_BYTES..MIN_WIRE_FRAME_BYTES + self.frame.payload().len()]
            .copy_from_slice(self.frame.payload());

        (bytes, MIN_WIRE_FRAME_BYTES + self.frame.payload().len())
    }

    pub fn decode(bytes: &'a [u8]) -> Result<Self, LoopbackError> {
        if bytes.len() < MIN_WIRE_FRAME_BYTES {
            return Err(LoopbackError::FrameTooShort);
        }

        let actual = WireVersion::new(bytes[0]);
        if actual != BLE_LOOPBACK_PROTOCOL_VERSION {
            return Err(LoopbackError::InvalidVersion {
                expected: BLE_LOOPBACK_PROTOCOL_VERSION,
                actual,
            });
        }

        let command = WireCommand::from_code(bytes[1])
            .ok_or(LoopbackError::InvalidCommand { code: bytes[1] })?;
        let payload_len = bytes[2] as usize;
        let required = MIN_WIRE_FRAME_BYTES + payload_len;

        if payload_len > MAX_LOOPBACK_PAYLOAD_BYTES {
            return Err(LoopbackError::PayloadTooLong {
                len: payload_len,
                max: MAX_LOOPBACK_PAYLOAD_BYTES,
            });
        }

        if bytes.len() < required {
            return Err(LoopbackError::FrameTooShort);
        }

        Ok(Self {
            frame: Frame::new(actual, command, &bytes[MIN_WIRE_FRAME_BYTES..required]),
        })
    }
}

/// Build the wire response for an incoming loopback frame.
pub fn handle_loopback_frame(
    bytes: &[u8],
    now_ms: u64,
) -> Result<([u8; MAX_LOOPBACK_FRAME_BYTES], usize), LoopbackError> {
    if bytes.len() < MIN_WIRE_FRAME_BYTES {
        return Err(LoopbackError::FrameTooShort);
    }

    let actual_version = WireVersion::new(bytes[0]);
    if actual_version != BLE_LOOPBACK_PROTOCOL_VERSION {
        return Err(LoopbackError::InvalidVersion {
            expected: BLE_LOOPBACK_PROTOCOL_VERSION,
            actual: actual_version,
        });
    }

    let command =
        WireCommand::from_code(bytes[1]).ok_or(LoopbackError::InvalidCommand { code: bytes[1] })?;
    let payload_len = bytes[2] as usize;
    if payload_len > MAX_LOOPBACK_PAYLOAD_BYTES {
        return Err(LoopbackError::PayloadTooLong {
            len: payload_len,
            max: MAX_LOOPBACK_PAYLOAD_BYTES,
        });
    }

    let required = MIN_WIRE_FRAME_BYTES + payload_len;
    if bytes.len() < required {
        return Err(LoopbackError::FrameTooShort);
    }

    let status_bytes = now_ms.to_le_bytes();
    let payload = match command {
        WireCommand::Ping | WireCommand::Teardown => &[][..],
        WireCommand::Echo => &bytes[MIN_WIRE_FRAME_BYTES..required],
        WireCommand::Status => &status_bytes,
    };

    let mut response = [0_u8; MAX_LOOPBACK_FRAME_BYTES];
    response[0] = BLE_LOOPBACK_PROTOCOL_VERSION.raw();
    response[1] = command.code();
    response[2] = payload.len() as u8;
    response[MIN_WIRE_FRAME_BYTES..MIN_WIRE_FRAME_BYTES + payload.len()].copy_from_slice(payload);

    Ok((response, MIN_WIRE_FRAME_BYTES + payload.len()))
}

#[cfg(test)]
mod tests {
    use super::{
        LoopbackError, LoopbackPacket, BLE_LOOPBACK_PROTOCOL_VERSION, MAX_LOOPBACK_PAYLOAD_BYTES,
        MIN_WIRE_FRAME_BYTES,
    };
    use crate::{WireCommand, WireVersion};
    use alloc::string::ToString;

    #[test]
    fn round_trips_ping_frames() {
        let packet = LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping packet");
        let (bytes, len) = packet.encode();

        assert_eq!(len, MIN_WIRE_FRAME_BYTES);
        assert_eq!(bytes[..len], [1, WireCommand::Ping.code(), 0]);

        let decoded = LoopbackPacket::decode(&bytes[..len]).expect("decoded packet");

        assert_eq!(decoded.frame().version(), BLE_LOOPBACK_PROTOCOL_VERSION);
        assert_eq!(decoded.frame().command(), WireCommand::Ping);
        assert_eq!(decoded.frame().payload(), &[]);
    }

    #[test]
    fn round_trips_echo_payloads() {
        let payload = [4_u8, 5, 6, 7];
        let packet = LoopbackPacket::new(WireCommand::Echo, &payload).expect("echo packet");
        let (bytes, len) = packet.encode();

        assert_eq!(len, MIN_WIRE_FRAME_BYTES + payload.len());
        let decoded = LoopbackPacket::decode(&bytes[..len]).expect("decoded packet");

        assert_eq!(decoded.frame().version(), BLE_LOOPBACK_PROTOCOL_VERSION);
        assert_eq!(decoded.frame().command(), WireCommand::Echo);
        assert_eq!(decoded.frame().payload(), &payload);
    }

    #[test]
    fn round_trips_every_loopback_command() {
        let payload = [0xaa, 0x55];

        [
            (WireCommand::Ping, &[][..]),
            (WireCommand::Echo, &payload[..]),
            (WireCommand::Status, &payload[..]),
            (WireCommand::Teardown, &[][..]),
        ]
        .into_iter()
        .for_each(|(command, payload)| {
            let packet = LoopbackPacket::new(command, payload).expect("loopback packet");
            let (bytes, len) = packet.encode();
            let decoded = LoopbackPacket::decode(&bytes[..len]).expect("decoded packet");

            assert_eq!(decoded.frame().version(), BLE_LOOPBACK_PROTOCOL_VERSION);
            assert_eq!(decoded.frame().command(), command);
            assert_eq!(decoded.frame().payload(), payload);
        });
    }

    #[test]
    fn rejects_unknown_versions_and_commands() {
        assert_eq!(
            LoopbackPacket::decode(&[2, WireCommand::Ping.code(), 0]),
            Err(LoopbackError::InvalidVersion {
                expected: BLE_LOOPBACK_PROTOCOL_VERSION,
                actual: WireVersion::new(2),
            })
        );
        assert_eq!(
            LoopbackPacket::decode(&[1, 99, 0]),
            Err(LoopbackError::InvalidCommand { code: 99 })
        );
    }

    #[test]
    fn rejects_payloads_that_exceed_the_ble_budget() {
        let payload = [0_u8; MAX_LOOPBACK_PAYLOAD_BYTES + 1];

        assert_eq!(
            LoopbackPacket::new(WireCommand::Status, &payload),
            Err(LoopbackError::PayloadTooLong {
                len: MAX_LOOPBACK_PAYLOAD_BYTES + 1,
                max: MAX_LOOPBACK_PAYLOAD_BYTES,
            })
        );
    }

    #[test]
    fn formats_loopback_errors_for_humans() {
        assert_eq!(LoopbackError::FrameTooShort.to_string(), "frame too short");
        assert_eq!(
            LoopbackError::InvalidVersion {
                expected: BLE_LOOPBACK_PROTOCOL_VERSION,
                actual: WireVersion::new(2),
            }
            .to_string(),
            "invalid protocol version: expected 1, got 2"
        );
        assert_eq!(
            LoopbackError::InvalidCommand { code: 99 }.to_string(),
            "invalid command code: 99"
        );
        assert_eq!(
            LoopbackError::PayloadTooLong { len: 17, max: 16 }.to_string(),
            "payload too long: 17 bytes (max 16)"
        );
    }

    #[test]
    fn handle_loopback_frame_echoes_ping_status_and_rejects_invalid_frames() {
        use super::handle_loopback_frame;

        let ping = LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping");
        let (bytes, len) = ping.encode();
        let (response, response_len) =
            handle_loopback_frame(&bytes[..len], 1234).expect("ping response");
        assert_eq!(&response[..response_len], &bytes[..len]);

        let status = LoopbackPacket::new(WireCommand::Status, &[]).expect("status");
        let (bytes, len) = status.encode();
        let (response, response_len) =
            handle_loopback_frame(&bytes[..len], 0x0102_0304_0506_0708).expect("status response");
        assert_eq!(
            response[..response_len],
            [1, WireCommand::Status.code(), 8, 8, 7, 6, 5, 4, 3, 2, 1]
        );

        assert!(handle_loopback_frame(&[9, 1, 0], 0).is_err());
    }
}
