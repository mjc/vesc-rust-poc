use std::fmt;
use std::io::Read;

use flate2::read::ZlibDecoder;
use sha2::{Digest, Sha256};

pub const VESC_PACKET_HEADER: &str = "VESC Packet";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageField {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispImport {
    pub tag: String,
    pub offset: usize,
    pub size: usize,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    TooShort,
    DecompressionFailed(String),
    LengthMismatch { expected: usize, actual: usize },
    InvalidHeader,
    InvalidUtf8,
    UnexpectedEof,
    NegativeImportCount,
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

pub fn parse_vescpkg(data: &[u8]) -> Result<Vec<PackageField>, WireError> {
    parse_decompressed_vescpkg(&decompress_vescpkg(data)?)
}

pub fn field_bytes<'a>(fields: &'a [PackageField], key: &str) -> Option<&'a [u8]> {
    fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.as_slice())
}

pub fn parse_lisp_imports(lisp_data: &[u8]) -> Result<(String, Vec<LispImport>), WireError> {
    let mut cursor = lisp_data;
    if read_i16_be(&mut cursor)? != 0 {
        return Err(WireError::UnexpectedEof);
    }

    let code = read_string(&mut cursor)?;
    let import_count = read_i16_be(&mut cursor)?;
    if import_count < 0 {
        return Err(WireError::NegativeImportCount);
    }

    let mut imports = Vec::with_capacity(import_count as usize);
    for _ in 0..import_count {
        let tag = read_string(&mut cursor)?;
        let offset = read_i32_be(&mut cursor)? as usize;
        let size = read_i32_be(&mut cursor)? as usize;
        let start = 2usize.saturating_add(offset);
        let end = start
            .checked_add(size)
            .ok_or(WireError::ImportOutOfBounds)?;
        if end > lisp_data.len() {
            return Err(WireError::ImportOutOfBounds);
        }

        imports.push(LispImport {
            tag,
            offset,
            size,
            payload: lisp_data[start..end].to_vec(),
        });
    }

    Ok((code, imports))
}

pub fn wire_snapshot_report(data: &[u8]) -> Result<String, WireError> {
    let decompressed = decompress_vescpkg(data)?;
    let fields = parse_decompressed_vescpkg(&decompressed)?;
    let mut lines = vec![
        format!("compressed_len: {}", data.len()),
        format!("decompressed_len: {}", decompressed.len()),
        "fields:".to_owned(),
    ];

    for field in &fields {
        lines.push(format!(
            "  {}: len={} sha256={}",
            field.key,
            field.value.len(),
            sha256_hex(&field.value)
        ));
        if field.key == "lispData" {
            let (_, imports) = parse_lisp_imports(&field.value)?;
            lines.push("    imports:".to_owned());
            for import in imports {
                lines.push(format!(
                    "      {}: offset={} size={} payload_sha256={}",
                    import.tag,
                    import.offset,
                    import.size,
                    sha256_hex(&import.payload)
                ));
            }
        }
    }

    Ok(lines.join("\n"))
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn read_i16_be(cursor: &mut &[u8]) -> Result<i16, WireError> {
    let bytes = take(cursor, 2)?;
    Ok(i16::from_be_bytes(bytes.try_into().expect("slice length")))
}

fn take(cursor: &mut &[u8], len: usize) -> Result<Vec<u8>, WireError> {
    if cursor.len() < len {
        return Err(WireError::UnexpectedEof);
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{
        PackageField, VESC_PACKET_HEADER, parse_decompressed_vescpkg, parse_lisp_imports,
        wire_snapshot_report,
    };

    fn field_spine() -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(format!("{VESC_PACKET_HEADER}\0").as_bytes());
        raw.extend_from_slice(b"name\0");
        raw.extend_from_slice(&4i32.to_be_bytes());
        raw.extend_from_slice(b"demo");
        raw
    }

    #[test]
    fn parse_decompressed_vescpkg_reads_field_spine() {
        let fields = parse_decompressed_vescpkg(&field_spine()).expect("field spine");
        assert_eq!(
            fields,
            vec![PackageField {
                key: "name".to_owned(),
                value: b"demo".to_vec(),
            }]
        );
    }

    #[test]
    fn parse_lisp_imports_reads_native_import_table() {
        let payload = [0xAA, 0xBB, 0xCC, 0xDD];
        let code = b"(import \"src/package_lib.bin\" 'package-lib)\0";

        let mut lisp = Vec::new();
        lisp.extend_from_slice(&0i16.to_be_bytes());
        lisp.extend_from_slice(code);
        lisp.extend_from_slice(&1i16.to_be_bytes());
        lisp.extend_from_slice(b"package-lib\0");
        let offset = i32::try_from(lisp.len() + 8 - 2).expect("offset fits in i32");
        lisp.extend_from_slice(&offset.to_be_bytes());
        lisp.extend_from_slice(&(payload.len() as i32).to_be_bytes());
        lisp.extend_from_slice(&payload);

        let (parsed_code, imports) = parse_lisp_imports(&lisp).expect("lisp imports");
        assert!(parsed_code.contains("package-lib"));
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].tag, "package-lib");
        assert_eq!(imports[0].payload, payload.to_vec());
    }

    #[test]
    fn wire_snapshot_report_redacts_payloads_to_lengths_and_hashes() {
        let payload = [0xAA, 0xBB, 0xCC, 0xDD];
        let code = b"(import \"src/package_lib.bin\" 'package-lib)\0";

        let mut lisp = Vec::new();
        lisp.extend_from_slice(&0i16.to_be_bytes());
        lisp.extend_from_slice(code);
        lisp.extend_from_slice(&1i16.to_be_bytes());
        lisp.extend_from_slice(b"package-lib\0");
        let offset = i32::try_from(lisp.len() + 8 - 2).expect("offset fits in i32");
        lisp.extend_from_slice(&offset.to_be_bytes());
        lisp.extend_from_slice(&(payload.len() as i32).to_be_bytes());
        lisp.extend_from_slice(&payload);

        let mut raw = field_spine();
        raw.extend_from_slice(b"lispData\0");
        raw.extend_from_slice(&(lisp.len() as i32).to_be_bytes());
        raw.extend_from_slice(&lisp);

        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &raw).expect("zlib payload");
        let compressed_body = encoder.finish().expect("zlib finish");
        let mut package = (raw.len() as u32).to_be_bytes().to_vec();
        package.extend_from_slice(&compressed_body);

        let report = wire_snapshot_report(&package).expect("wire snapshot");
        assert!(report.contains("fields:"));
        assert!(report.contains("name: len=4 sha256="));
        assert!(report.contains("lispData:"));
        assert!(report.contains("imports:"));
        assert!(report.contains("package-lib: offset="));
        assert!(!report.contains("0xaa"));
    }
}
