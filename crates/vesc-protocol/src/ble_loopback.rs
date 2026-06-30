use super::{Frame, InvalidWireCommand, WireCommand, WireVersion};
use core::fmt;

/// Wire protocol version used by loopback frames.
pub const BLE_LOOPBACK_PROTOCOL_VERSION: WireVersion = WireVersion::CURRENT;
/// Maximum payload size accepted by the loopback frame encoder.
pub const MAX_LOOPBACK_PAYLOAD_BYTES: usize = 16;
/// Size of the fixed frame header in bytes.
pub const MIN_WIRE_FRAME_BYTES: usize = 3;
/// Maximum encoded loopback frame size in bytes.
pub const MAX_LOOPBACK_FRAME_BYTES: usize = MIN_WIRE_FRAME_BYTES + MAX_LOOPBACK_PAYLOAD_BYTES;

/// Errors returned when loopback frame decoding or handling fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackError {
    /// The frame was shorter than the fixed header.
    FrameTooShort,
    /// The encoded version byte did not match the loopback protocol version.
    InvalidVersion {
        /// Expected wire version.
        expected: WireVersion,
        /// Actual wire version found in the frame.
        actual: WireVersion,
    },
    /// The encoded command byte was not recognized.
    InvalidCommand {
        /// Unknown command code.
        code: InvalidWireCommand,
    },
    /// The output buffer cannot hold the encoded frame.
    BufferTooShort {
        /// Provided output buffer length.
        len: usize,
        /// Required encoded frame length.
        required: usize,
    },
    /// The payload length exceeded the maximum supported frame payload.
    PayloadTooLong {
        /// Payload length from the wire frame.
        len: usize,
        /// Maximum supported payload length.
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
                    expected.get(),
                    actual.get()
                )
            }
            Self::InvalidCommand { code } => {
                write!(f, "invalid command code: {}", code.get())
            }
            Self::BufferTooShort { len, required } => {
                write!(f, "buffer too short: {len} bytes (need {required})")
            }
            Self::PayloadTooLong { len, max } => {
                write!(f, "payload too long: {len} bytes (max {max})")
            }
        }
    }
}

/// Borrowed loopback packet wrapper used by the shared wire protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackPacket<'a> {
    frame: Frame<'a>,
}

impl<'a> LoopbackPacket<'a> {
    /// Construct a validated loopback packet.
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

    /// Return the underlying typed frame by value.
    pub fn frame(&self) -> Frame<'a> {
        self.frame.clone()
    }

    /// Encode the packet into the provided output buffer.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, LoopbackError> {
        let required = MIN_WIRE_FRAME_BYTES + self.frame.payload().len();
        if out.len() < required {
            return Err(LoopbackError::BufferTooShort {
                len: out.len(),
                required,
            });
        }

        out[0] = self.frame.version().get();
        out[1] = u8::from(self.frame.command());
        out[2] = self.frame.payload().len() as u8;

        let mut index = 0;
        while index < self.frame.payload().len() {
            out[MIN_WIRE_FRAME_BYTES + index] = self.frame.payload()[index];
            index += 1;
        }

        Ok(required)
    }

    /// Encode the packet into a fixed-size byte buffer and its used length.
    pub fn encode(&self) -> ([u8; MAX_LOOPBACK_FRAME_BYTES], usize) {
        let mut bytes = [0_u8; MAX_LOOPBACK_FRAME_BYTES];
        let payload = self.frame.payload();
        bytes[0] = self.frame.version().get();
        bytes[1] = u8::from(self.frame.command());
        bytes[2] = payload.len() as u8;
        copy_payload_no_runtime(&mut bytes, payload);

        (bytes, MIN_WIRE_FRAME_BYTES + payload.len())
    }

    /// Decode a packet from raw bytes.
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

        let command = WireCommand::try_from(bytes[1])
            .map_err(|code| LoopbackError::InvalidCommand { code })?;
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

#[cfg_attr(all(not(test), target_arch = "arm"), no_panic::no_panic)]
fn copy_payload_no_runtime(response: &mut [u8; MAX_LOOPBACK_FRAME_BYTES], payload: &[u8]) {
    let mut index = 0;
    while index < payload.len() {
        // SAFETY: callers only pass payload slices that are already capped to
        // MAX_LOOPBACK_PAYLOAD_BYTES, and response has exactly enough trailing
        // capacity after the fixed header. Pointer copy avoids target codegen
        // pulling in memcpy/compiler-builtins in the final native package.
        unsafe {
            *response.as_mut_ptr().add(MIN_WIRE_FRAME_BYTES + index) = *payload.as_ptr().add(index);
        }
        index += 1;
    }
}

/// Build the wire response for an incoming loopback frame.
#[cfg_attr(all(not(test), target_arch = "arm"), no_panic::no_panic)]
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
        WireCommand::try_from(bytes[1]).map_err(|code| LoopbackError::InvalidCommand { code })?;
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
    response[0] = BLE_LOOPBACK_PROTOCOL_VERSION.get();
    response[1] = u8::from(command);
    response[2] = payload.len() as u8;
    copy_payload_no_runtime(&mut response, payload);

    Ok((response, MIN_WIRE_FRAME_BYTES + payload.len()))
}

#[cfg(test)]
mod tests {
    use super::{
        BLE_LOOPBACK_PROTOCOL_VERSION, LoopbackError, LoopbackPacket, MAX_LOOPBACK_FRAME_BYTES,
        MAX_LOOPBACK_PAYLOAD_BYTES, MIN_WIRE_FRAME_BYTES,
    };
    use crate::{WireCommand, WireVersion};
    use std::format;

    #[test]
    fn round_trips_ping_frames() {
        let packet = LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping packet");
        let (bytes, len) = packet.encode();

        assert_eq!(len, MIN_WIRE_FRAME_BYTES);
        assert_eq!(bytes[..len], [1, u8::from(WireCommand::Ping), 0]);

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
    fn encode_into_reports_short_output_buffers() {
        let payload = [4_u8, 5, 6, 7];
        let packet = LoopbackPacket::new(WireCommand::Echo, &payload).expect("echo packet");
        let mut bytes = [0_u8; MIN_WIRE_FRAME_BYTES + 1];

        assert_eq!(
            packet.encode_into(&mut bytes),
            Err(LoopbackError::BufferTooShort {
                len: MIN_WIRE_FRAME_BYTES + 1,
                required: MIN_WIRE_FRAME_BYTES + payload.len(),
            })
        );
    }

    #[test]
    fn encode_into_writes_typed_protocol_frame() {
        let payload = [9_u8, 8];
        let packet = LoopbackPacket::new(WireCommand::Echo, &payload).expect("echo packet");
        let mut bytes = [0_u8; MAX_LOOPBACK_FRAME_BYTES];

        let len = packet.encode_into(&mut bytes).expect("encoded frame");

        assert_eq!(len, MIN_WIRE_FRAME_BYTES + payload.len());
        assert_eq!(&bytes[..len], &[1, u8::from(WireCommand::Echo), 2, 9, 8]);
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
            LoopbackPacket::decode(&[2, u8::from(WireCommand::Ping), 0]),
            Err(LoopbackError::InvalidVersion {
                expected: BLE_LOOPBACK_PROTOCOL_VERSION,
                actual: WireVersion::new(2),
            })
        );
        assert_eq!(
            LoopbackPacket::decode(&[1, 99, 0]),
            Err(LoopbackError::InvalidCommand {
                code: crate::InvalidWireCommand::new(99),
            })
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
        assert_eq!(
            format!("{}", LoopbackError::FrameTooShort),
            "frame too short"
        );
        assert_eq!(
            format!(
                "{}",
                LoopbackError::InvalidVersion {
                    expected: BLE_LOOPBACK_PROTOCOL_VERSION,
                    actual: WireVersion::new(2),
                }
            ),
            "invalid protocol version: expected 1, got 2"
        );
        assert_eq!(
            format!(
                "{}",
                LoopbackError::InvalidCommand {
                    code: crate::InvalidWireCommand::new(99),
                }
            ),
            "invalid command code: 99"
        );
        assert_eq!(
            format!("{}", LoopbackError::PayloadTooLong { len: 17, max: 16 }),
            "payload too long: 17 bytes (max 16)"
        );
        assert_eq!(
            format!(
                "{}",
                LoopbackError::BufferTooShort {
                    len: 4,
                    required: 7,
                }
            ),
            "buffer too short: 4 bytes (need 7)"
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
            [1, u8::from(WireCommand::Status), 8, 8, 7, 6, 5, 4, 3, 2, 1]
        );

        assert!(handle_loopback_frame(&[9, 1, 0], 0).is_err());
    }
}
