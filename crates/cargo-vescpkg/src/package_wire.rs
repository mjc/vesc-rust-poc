use std::fmt;
use std::io::Read;

use flate2::read::ZlibDecoder;

/// Wire-format header prefix used by VESC package archives.
const VESC_PACKET_HEADER: &str = "VESC Packet";

/// One decoded key/value field from a `.vescpkg` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageField {
    /// Field name from the package payload.
    pub key: String,
    /// Raw field bytes from the package payload.
    pub value: Vec<u8>,
}

/// Errors encountered while parsing package wire data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError(String);

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for WireError {}

fn decompress(data: &[u8]) -> Result<Vec<u8>, WireError> {
    if data.len() < 4 {
        return Err(WireError("vescpkg payload is too short".to_owned()));
    }

    let expected_len = u32::from_be_bytes(data[..4].try_into().expect("slice length")) as usize;
    let mut decoder = ZlibDecoder::new(&data[4..]).take(expected_len as u64 + 1);
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .map_err(|error| WireError(format!("vescpkg decompress failed: {error}")))?;
    if bytes.len() != expected_len {
        return Err(WireError(format!(
            "vescpkg decompress length mismatch: expected {expected_len}, got {}",
            bytes.len()
        )));
    }

    Ok(bytes)
}

fn parse_raw(raw: &[u8]) -> Result<Vec<PackageField>, WireError> {
    let mut cursor = raw;
    if read_string(&mut cursor)? != VESC_PACKET_HEADER {
        return Err(WireError("vescpkg missing VESC Packet header".to_owned()));
    }

    let mut fields = Vec::new();
    while !cursor.is_empty() {
        let key = read_string(&mut cursor)?;
        let field_len = read_i32_be(&mut cursor)?;
        let len = usize::try_from(field_len).map_err(|_| {
            WireError(format!(
                "negative vescpkg field length {field_len} for {key}"
            ))
        })?;
        let value = take(&mut cursor, len)?;
        fields.push(PackageField { key, value });
    }

    Ok(fields)
}

/// Decompress and parse a `.vescpkg` blob.
pub fn parse_vescpkg(data: &[u8]) -> Result<Vec<PackageField>, WireError> {
    parse_raw(&decompress(data)?)
}

fn read_string(cursor: &mut &[u8]) -> Result<String, WireError> {
    let Some(end) = cursor.iter().position(|byte| *byte == 0) else {
        return Err(WireError("unexpected end of vescpkg wire data".to_owned()));
    };
    let bytes = take(cursor, end)?;
    take(cursor, 1)?;
    String::from_utf8(bytes).map_err(|_| WireError("vescpkg field is not valid utf-8".to_owned()))
}

fn read_i32_be(cursor: &mut &[u8]) -> Result<i32, WireError> {
    let bytes = take(cursor, 4)?;
    Ok(i32::from_be_bytes(bytes.try_into().expect("slice length")))
}

fn take(cursor: &mut &[u8], len: usize) -> Result<Vec<u8>, WireError> {
    if cursor.len() < len {
        return Err(WireError("unexpected end of vescpkg wire data".to_owned()));
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head.to_vec())
}
