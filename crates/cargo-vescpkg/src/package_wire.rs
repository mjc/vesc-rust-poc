use std::fmt;
use std::io::Read;

use flate2::read::ZlibDecoder;

/// Wire-format header prefix used by VESC package archives.
pub const VESC_PACKET_HEADER: &str = "VESC Packet";

/// One decoded key/value field from a `.vescpkg` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageField {
    /// Field name from the package payload.
    pub key: String,
    /// Raw field bytes from the package payload.
    pub value: Vec<u8>,
}

/// Errors encountered while parsing or validating package wire data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The compressed payload was too short to contain a length prefix.
    TooShort,
    /// Zlib decompression failed.
    DecompressionFailed(String),
    /// The decompressed byte count did not match the encoded length.
    LengthMismatch {
        /// Declared decompressed byte count.
        expected: usize,
        /// Actual decompressed byte count.
        actual: usize,
    },
    /// The decompressed payload did not start with the expected header.
    InvalidHeader,
    /// A decoded string was not valid UTF-8.
    InvalidUtf8,
    /// The input ended before the requested bytes were available.
    UnexpectedEof,
    /// The Lisp import count was negative.
    NegativeImportCount,
    /// An import range extended past the available data.
    ImportOutOfBounds,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => f.write_str("vescpkg payload is too short"),
            Self::DecompressionFailed(reason) => write!(f, "vescpkg decompress failed: {reason}"),
            Self::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "vescpkg decompress length mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidHeader => f.write_str("vescpkg missing VESC Packet header"),
            Self::InvalidUtf8 => f.write_str("vescpkg field is not valid utf-8"),
            Self::UnexpectedEof => f.write_str("unexpected end of vescpkg wire data"),
            Self::NegativeImportCount => f.write_str("negative Lisp import count"),
            Self::ImportOutOfBounds => f.write_str("Lisp import payload out of bounds"),
        }
    }
}

impl std::error::Error for WireError {}

/// Decompress a `.vescpkg` blob into its raw package payload.
pub fn decompress_vescpkg(data: &[u8]) -> Result<Vec<u8>, WireError> {
    if data.len() < 4 {
        return Err(WireError::TooShort);
    }

    let expected_len = u32::from_be_bytes(data[..4].try_into().expect("slice length")) as usize;
    let mut decoder = ZlibDecoder::new(&data[4..]);
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .map_err(|error| WireError::DecompressionFailed(error.to_string()))?;
    if bytes.len() != expected_len {
        return Err(WireError::LengthMismatch {
            expected: expected_len,
            actual: bytes.len(),
        });
    }

    Ok(bytes)
}

/// Parse a decompressed package payload into its key/value fields.
pub fn parse_decompressed_vescpkg(raw: &[u8]) -> Result<Vec<PackageField>, WireError> {
    let mut cursor = raw;
    if read_string(&mut cursor)? != VESC_PACKET_HEADER {
        return Err(WireError::InvalidHeader);
    }

    let mut fields = Vec::new();
    while !cursor.is_empty() {
        let key = read_string(&mut cursor)?;
        let len = read_i32_be(&mut cursor)?;
        let len = usize::try_from(len).map_err(|_| WireError::UnexpectedEof)?;
        let value = take(&mut cursor, len)?;
        fields.push(PackageField { key, value });
    }

    Ok(fields)
}

/// Decompress and parse a `.vescpkg` blob.
pub fn parse_vescpkg(data: &[u8]) -> Result<Vec<PackageField>, WireError> {
    parse_decompressed_vescpkg(&decompress_vescpkg(data)?)
}

fn read_string(cursor: &mut &[u8]) -> Result<String, WireError> {
    let Some(end) = cursor.iter().position(|byte| *byte == 0) else {
        return Err(WireError::UnexpectedEof);
    };
    let bytes = take(cursor, end)?;
    take(cursor, 1)?;
    String::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8)
}

fn read_i32_be(cursor: &mut &[u8]) -> Result<i32, WireError> {
    let bytes = take(cursor, 4)?;
    Ok(i32::from_be_bytes(bytes.try_into().expect("slice length")))
}

fn take(cursor: &mut &[u8], len: usize) -> Result<Vec<u8>, WireError> {
    if cursor.len() < len {
        return Err(WireError::UnexpectedEof);
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head.to_vec())
}
