use super::{Frame, WireCommand, WireVersion};

pub const BLE_LOOPBACK_PROTOCOL_VERSION: WireVersion = WireVersion::CURRENT;
pub const MAX_LOOPBACK_PAYLOAD_BYTES: usize = 16;
pub const MIN_WIRE_FRAME_BYTES: usize = 3;

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

#[cfg(test)]
mod tests {
    use super::{
        LoopbackError, LoopbackPacket, BLE_LOOPBACK_PROTOCOL_VERSION, MAX_LOOPBACK_PAYLOAD_BYTES,
        MIN_WIRE_FRAME_BYTES,
    };
    use crate::{WireCommand, WireVersion};

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
}
