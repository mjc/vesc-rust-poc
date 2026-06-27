use flate2::read::ZlibDecoder;
use std::io::Read;

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

pub fn read_string(cursor: &mut &[u8]) -> String {
    let end = cursor
        .iter()
        .position(|byte| *byte == 0)
        .expect("nul-terminated string");
    let value = std::str::from_utf8(&cursor[..end])
        .expect("utf-8 string")
        .to_owned();
    *cursor = &cursor[end + 1..];
    value
}

pub fn read_i32_be(cursor: &mut &[u8]) -> i32 {
    let (bytes, rest) = cursor.split_at(4);
    *cursor = rest;
    i32::from_be_bytes(bytes.try_into().expect("i32 bytes"))
}

pub fn read_i16_be(cursor: &mut &[u8]) -> i16 {
    let (bytes, rest) = cursor.split_at(2);
    *cursor = rest;
    i16::from_be_bytes(bytes.try_into().expect("i16 bytes"))
}

pub fn decompress_vescpkg(package: &[u8]) -> Vec<u8> {
    let declared_len =
        u32::from_be_bytes(package[..4].try_into().expect("qCompress length")) as usize;
    let mut decoder = ZlibDecoder::new(&package[4..]);
    let mut raw = Vec::new();
    decoder
        .read_to_end(&mut raw)
        .expect("decompress package payload");
    assert_eq!(raw.len(), declared_len);
    raw
}

pub fn package_fields(package: &[u8]) -> Vec<PackageField> {
    let raw = decompress_vescpkg(package);
    let mut cursor = raw.as_slice();
    assert_eq!(read_string(&mut cursor), "VESC Packet");

    let mut fields = Vec::new();
    while !cursor.is_empty() {
        let key = read_string(&mut cursor);
        let len = read_i32_be(&mut cursor) as usize;
        let (value, rest) = cursor.split_at(len);
        cursor = rest;
        fields.push(PackageField {
            key,
            value: value.to_vec(),
        });
    }
    fields
}

pub fn extract_field(package: &[u8], key: &str) -> Vec<u8> {
    package_fields(package)
        .into_iter()
        .find(|field| field.key == key)
        .unwrap_or_else(|| panic!("missing field {key}"))
        .value
}

pub fn parse_lisp_imports(lisp_data: &[u8]) -> (String, Vec<LispImport>) {
    let mut cursor = lisp_data;
    assert_eq!(read_i16_be(&mut cursor), 0);
    let code = read_string(&mut cursor);
    let import_count = read_i16_be(&mut cursor);
    assert!(import_count >= 0, "negative Lisp import count");

    let imports = (0..import_count)
        .map(|_| {
            let tag = read_string(&mut cursor);
            let offset = read_i32_be(&mut cursor) as usize;
            let size = read_i32_be(&mut cursor) as usize;
            let start = 2 + offset;
            let end = start + size;
            LispImport {
                tag,
                offset,
                size,
                payload: lisp_data[start..end].to_vec(),
            }
        })
        .collect();

    (code, imports)
}

pub fn payload_matches_native_with_only_nul_tail(payload: &[u8], native: &[u8]) -> bool {
    payload.starts_with(native) && payload[native.len()..].iter().all(|byte| *byte == 0)
}

#[cfg(test)]
pub fn assert_bytes_eq(actual: &[u8], expected: &[u8], label: &str) {
    if actual == expected {
        return;
    }

    let mismatch = actual
        .iter()
        .zip(expected.iter())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| actual.len().min(expected.len()));

    let actual_snippet = hex_snippet(actual, mismatch);
    let expected_snippet = hex_snippet(expected, mismatch);

    panic!(
        "{label}: byte mismatch at offset {mismatch} (actual len {}, expected len {})\n  actual:   {actual_snippet}\n  expected: {expected_snippet}",
        actual.len(),
        expected.len()
    );
}

#[cfg(test)]
fn hex_snippet(bytes: &[u8], start: usize) -> String {
    let end = (start + 16).min(bytes.len());
    bytes[start..end]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
