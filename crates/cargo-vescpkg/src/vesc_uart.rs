use std::vec::Vec;

const START_8B: u8 = 2;
const START_16B: u8 = 3;
const START_24B: u8 = 4;
const STOP_BYTE: u8 = 3;

const CRC16_TABLE: [u16; 256] = crc16_table();

#[allow(clippy::cast_possible_truncation)]
const fn crc16_table() -> [u16; 256] {
    let mut table = [0; 256];
    let mut index = 0;
    while index < table.len() {
        let mut crc = (index as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

/// Incremental decoder for VESC UART packet frames.
#[derive(Debug, Default)]
pub struct PacketDecoder {
    buffer: Vec<u8>,
}

impl PacketDecoder {
    /// Drops any buffered partial bytes.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Pushes bytes into the decoder and returns complete payloads.
    ///
    /// # Errors
    ///
    /// Returns an error when a complete frame is malformed.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, PacketDecodeError> {
        self.buffer.extend_from_slice(bytes);
        let mut packets = Vec::new();

        loop {
            match try_decode_packet(&self.buffer) {
                DecodeOutcome::NeedMore => return Ok(packets),
                DecodeOutcome::Discard(bytes) => {
                    self.buffer.drain(..bytes);
                }
                DecodeOutcome::Packet { packet, consumed } => {
                    packets.push(packet);
                    self.buffer.drain(..consumed);
                }
            }
        }
    }
}

/// Errors returned while decoding VESC UART packet frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketDecodeError {
    /// The buffered packet header, length, CRC, or terminator was malformed.
    Malformed,
}

/// Encodes a payload into a VESC UART packet with length, CRC, and terminator bytes.
///
/// # Panics
///
/// Panics when `payload` exceeds the UART protocol's 24-bit length field.
#[must_use]
pub fn encode_packet(payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(payload.len() + 8);
    let len = payload.len();
    assert!(len <= 0xFF_FFFF, "VESC UART payload exceeds 24-bit length");

    if len <= 255 {
        packet.push(START_8B);
        packet.push(u8::try_from(len & 0xff).expect("short UART length fits u8"));
    } else if len <= 65535 {
        packet.push(START_16B);
        packet.push(u8::try_from((len >> 8) & 0xff).expect("UART length high byte fits u8"));
        packet.push(u8::try_from(len & 0xff).expect("UART length low byte fits u8"));
    } else {
        packet.push(START_24B);
        packet.push(u8::try_from((len >> 16) & 0xff).expect("UART length high byte fits u8"));
        packet.push(u8::try_from((len >> 8) & 0xff).expect("UART length middle byte fits u8"));
        packet.push(u8::try_from(len & 0xff).expect("UART length low byte fits u8"));
    }

    packet.extend_from_slice(payload);
    let crc = crc16(payload);
    packet.push(u8::try_from(crc >> 8).expect("CRC high byte fits u8"));
    packet.push(u8::try_from(crc & 0xff).expect("CRC low byte fits u8"));
    packet.push(STOP_BYTE);
    packet
}

/// Computes the VESC UART CRC16 checksum for `buf`.
#[must_use]
pub fn crc16(buf: &[u8]) -> u16 {
    buf.iter().fold(0u16, |crc, byte| {
        CRC16_TABLE[usize::from((crc >> 8) as u8 ^ *byte)] ^ (crc << 8)
    })
}

enum DecodeOutcome {
    NeedMore,
    Discard(usize),
    Packet { packet: Vec<u8>, consumed: usize },
}

fn try_decode_packet(buffer: &[u8]) -> DecodeOutcome {
    if buffer.is_empty() {
        return DecodeOutcome::NeedMore;
    }

    let start = buffer[0];
    let (len, header_len) = match start {
        START_8B => {
            if buffer.len() < 2 {
                return DecodeOutcome::NeedMore;
            }
            let len = buffer[1] as usize;
            if len == 0 {
                return DecodeOutcome::Discard(1);
            }
            (len, 2)
        }
        START_16B => {
            if buffer.len() < 3 {
                return DecodeOutcome::NeedMore;
            }
            let len = ((buffer[1] as usize) << 8) | buffer[2] as usize;
            if len < 255 {
                return DecodeOutcome::Discard(1);
            }
            (len, 3)
        }
        START_24B => {
            if buffer.len() < 4 {
                return DecodeOutcome::NeedMore;
            }
            let len =
                ((buffer[1] as usize) << 16) | ((buffer[2] as usize) << 8) | buffer[3] as usize;
            if len < 65535 {
                return DecodeOutcome::Discard(1);
            }
            (len, 4)
        }
        _ => return DecodeOutcome::Discard(1),
    };

    let needed = header_len + len + 3;
    if buffer.len() < needed {
        return DecodeOutcome::NeedMore;
    }
    if buffer[header_len + len + 2] != STOP_BYTE {
        return DecodeOutcome::Discard(1);
    }

    let payload = &buffer[header_len..header_len + len];
    let crc_rx = u16::from(buffer[header_len + len]) << 8 | u16::from(buffer[header_len + len + 1]);
    if crc16(payload) != crc_rx {
        return DecodeOutcome::Discard(1);
    }

    DecodeOutcome::Packet {
        packet: payload.to_vec(),
        consumed: needed,
    }
}

#[cfg(test)]
mod tests {
    use super::{PacketDecoder, START_16B, START_24B, crc16, encode_packet};

    #[test]
    fn encodes_and_decodes_short_packets() {
        let payload = [131_u8, 1];
        let packet = encode_packet(&payload);
        assert_eq!(packet[0], 2);
        assert_eq!(
            packet[1],
            u8::try_from(payload.len()).expect("test payload length fits u8")
        );
        assert_eq!(
            crc16(&payload),
            u16::from(packet[packet.len() - 3]) << 8 | u16::from(packet[packet.len() - 2])
        );

        let mut decoder = PacketDecoder::default();
        let packets = decoder.push(&packet).expect("packets");
        assert_eq!(packets, vec![payload.to_vec()]);
    }

    #[test]
    fn encodes_and_decodes_extended_packet_lengths() {
        for (length, header) in [
            (256, vec![START_16B, 1, 0]),
            (65_536, vec![START_24B, 1, 0, 0]),
        ] {
            let payload = vec![0x5a; length];
            let packet = encode_packet(&payload);
            assert_eq!(&packet[..header.len()], &header);

            let mut decoder = PacketDecoder::default();
            assert_eq!(
                decoder.push(&packet).expect("extended packet"),
                vec![payload]
            );
        }
    }

    #[test]
    fn decodes_split_packets() {
        let payload = [120_u8, 0, 0, 0, 8];
        let packet = encode_packet(&payload);
        let mut decoder = PacketDecoder::default();

        assert!(decoder.push(&packet[..3]).expect("part 1").is_empty());
        let packets = decoder.push(&packet[3..]).expect("part 2");
        assert_eq!(packets, vec![payload.to_vec()]);
    }

    #[test]
    fn clear_drops_partial_packets() {
        let payload = [120_u8, 0, 0, 0, 8];
        let packet = encode_packet(&payload);
        let mut decoder = PacketDecoder::default();

        assert!(decoder.push(&packet[..3]).expect("part 1").is_empty());
        decoder.clear();

        assert!(decoder.push(&packet[3..]).expect("old tail").is_empty());
    }
}
