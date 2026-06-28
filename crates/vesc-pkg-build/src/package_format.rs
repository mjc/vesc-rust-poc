use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use flate2::{write::ZlibEncoder, Compression};

const PACKAGE_MAGIC: &str = "VESC Packet";

#[derive(Debug, Clone)]
pub struct VescPackageInput<'a> {
    pub name: &'a str,
    pub description_md: &'a str,
    pub lisp_source: &'a str,
    pub lisp_editor_path: &'a Path,
    pub qml_file: &'a str,
    pub pkg_desc_qml: &'a str,
    pub qml_is_fullscreen: bool,
}

pub fn build_vesc_package(input: &VescPackageInput<'_>) -> io::Result<Vec<u8>> {
    let lisp_data = pack_lisp_imports(input.lisp_source, input.lisp_editor_path)?;

    let mut data = Vec::new();
    append_string(&mut data, PACKAGE_MAGIC);

    append_text_field(&mut data, "name", input.name)?;
    append_text_field(&mut data, "description_md", input.description_md)?;
    append_bytes_field(&mut data, "lispData", &lisp_data)?;
    append_text_field(&mut data, "qmlFile", input.qml_file)?;
    append_text_field(&mut data, "pkgDescQml", input.pkg_desc_qml)?;

    append_string(&mut data, "qmlIsFullscreen");
    append_i32_be(&mut data, 1);
    data.push(u8::from(input.qml_is_fullscreen));

    q_compress(&data)
}

pub fn write_vesc_package(
    output_path: impl AsRef<Path>,
    input: &VescPackageInput<'_>,
) -> io::Result<Vec<u8>> {
    let bytes = build_vesc_package(input)?;

    if let Some(parent) = output_path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path.as_ref(), &bytes)?;
    Ok(bytes)
}

fn append_text_field(buf: &mut Vec<u8>, key: &str, value: &str) -> io::Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    append_string(buf, key);
    append_bytes(buf, value.as_bytes())?;
    Ok(())
}

fn append_bytes_field(buf: &mut Vec<u8>, key: &str, value: &[u8]) -> io::Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    append_string(buf, key);
    append_bytes(buf, value)?;
    Ok(())
}

fn append_bytes(buf: &mut Vec<u8>, value: &[u8]) -> io::Result<()> {
    let len = i32::try_from(value.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "package field exceeds the VESC packet length limit",
        )
    })?;
    append_i32_be(buf, len);
    buf.extend_from_slice(value);
    Ok(())
}

fn append_string(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(value.as_bytes());
    buf.push(0);
}

fn append_i32_be(buf: &mut Vec<u8>, value: i32) {
    buf.extend_from_slice(&value.to_be_bytes());
}

fn q_compress(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;

    let uncompressed_len = u32::try_from(data.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "package payload exceeds the Qt qCompress length limit",
        )
    })?;

    let mut output = Vec::with_capacity(4 + compressed.len());
    output.extend_from_slice(&uncompressed_len.to_be_bytes());
    output.extend_from_slice(&compressed);
    Ok(output)
}

fn pack_lisp_imports(code_str: &str, editor_path: &Path) -> io::Result<Vec<u8>> {
    let mut packed = Vec::new();
    packed.extend_from_slice(&0u16.to_be_bytes());
    packed.extend_from_slice(code_str.as_bytes());
    if packed.last().copied() != Some(0) {
        packed.push(0);
    }

    let mut imports = Vec::new();
    for line in code_str.lines() {
        let Some((path, tag)) = parse_import_line(line) else {
            continue;
        };

        let source_path = resolve_import_path(editor_path, &path);
        let mut file_data = fs::read(&source_path)?;
        if file_data.last().copied() != Some(0) {
            file_data.push(0);
        }
        imports.push((tag, file_data));
    }

    let file_table_size = imports
        .iter()
        .fold(0usize, |acc, (tag, _)| acc + tag.len() + 9);
    let num_imports = i16::try_from(imports.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "too many Lisp imports for a VESC package",
        )
    })?;
    packed.extend_from_slice(&num_imports.to_be_bytes());

    let mut file_offset = packed.len() + file_table_size - 2;
    for (tag, data) in &imports {
        while file_offset % 4 != 0 {
            file_offset += 1;
        }

        append_string(&mut packed, tag);
        append_i32_be(
            &mut packed,
            i32::try_from(file_offset).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Lisp import offset exceeds the VESC package limit",
                )
            })?,
        );
        append_i32_be(
            &mut packed,
            i32::try_from(data.len()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Lisp import payload exceeds the VESC package limit",
                )
            })?,
        );
        file_offset += data.len();
    }

    for (_, data) in &imports {
        while (packed.len() - 2) % 4 != 0 {
            packed.push(0);
        }
        packed.extend_from_slice(data);
    }

    Ok(packed)
}

fn resolve_import_path(editor_path: &Path, import_path: &str) -> std::path::PathBuf {
    let relative_candidate = editor_path.join(import_path);
    if relative_candidate.exists() {
        return relative_candidate;
    }

    std::path::PathBuf::from(import_path)
}

fn parse_import_line(line: &str) -> Option<(String, String)> {
    let mut trimmed = line.trim_start();
    while trimmed.starts_with("( ") {
        trimmed = &trimmed[1..];
    }

    if !trimmed.starts_with("(import ") {
        return None;
    }

    let start = trimmed.find('"')?;
    let end = trimmed.rfind('"')?;
    if end <= start {
        return None;
    }

    let path = trimmed[start + 1..end].to_owned();
    let mut tag = trimmed[end + 1..].replace(['\r', ' ', ')', '\''], "");
    if let Some(index) = tag.find(';') {
        tag.truncate(index);
    }

    if path.is_empty() || tag.is_empty() {
        return None;
    }

    Some((path, tag))
}

#[cfg(test)]
mod tests {
    use super::{build_vesc_package, VescPackageInput};
    use crate::package_wire::{field_bytes, parse_lisp_imports, parse_vescpkg, LispImport};
    use crate::test_support::PackageTestHarness;
    use crate::{PackageAssets, PackageLayout, PackageProvenance, BLE_LOOPBACK_PACKAGE_NAME};

    fn extract_field(package: &[u8], key: &str) -> Vec<u8> {
        field_bytes(&parse_vescpkg(package).expect("vescpkg"), key)
            .expect("missing field")
            .to_vec()
    }

    fn payload_matches_native_with_only_nul_tail(payload: &[u8], native: &[u8]) -> bool {
        payload.starts_with(native) && payload[native.len()..].iter().all(|byte| *byte == 0)
    }

    #[test]
    fn lisp_imports_embed_native_payload_bytes() {
        let harness = PackageTestHarness::new().write_native_payload([0, 1, 2, 3, 0xff]);
        let loader = harness.loopback_loader_lisp();

        let package = build_vesc_package(&VescPackageInput {
            name: "test",
            description_md: "",
            lisp_source: &loader,
            lisp_editor_path: harness.root(),
            qml_file: "",
            pkg_desc_qml: "",
            qml_is_fullscreen: false,
        })
        .expect("package");
        let lisp_data = extract_field(&package, "lispData");
        let (code, imports) = parse_lisp_imports(&lisp_data).expect("lisp imports");

        assert_eq!(code, loader);
        assert_eq!(
            imports,
            vec![LispImport {
                tag: "package-lib".to_owned(),
                offset: 100,
                size: 6,
                payload: vec![0, 1, 2, 3, 0xff, 0],
            }]
        );
        assert_eq!(imports[0].offset % 4, 0);
    }

    #[test]
    fn lisp_import_payload_preserves_native_bytes_with_only_nul_padding() {
        let native_payload = [0, 1, 2, 3, 0];
        let harness = PackageTestHarness::new().write_native_payload(native_payload);
        let loader = harness.loopback_loader_lisp();

        let package = build_vesc_package(&VescPackageInput {
            name: "test",
            description_md: "",
            lisp_source: &loader,
            lisp_editor_path: harness.root(),
            qml_file: "",
            pkg_desc_qml: "",
            qml_is_fullscreen: false,
        })
        .expect("package");
        let lisp_data = extract_field(&package, "lispData");
        let (_, imports) = parse_lisp_imports(&lisp_data).expect("lisp imports");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].tag, "package-lib");
        assert!(payload_matches_native_with_only_nul_tail(
            &imports[0].payload,
            &native_payload
        ));
    }

    #[test]
    fn package_uses_the_vesc_tool_field_spine() {
        let harness = PackageTestHarness::new().write_native_payload([0xaa]);
        let loader = harness.loopback_loader_lisp_import_only();

        let package = build_vesc_package(&VescPackageInput {
            name: "test",
            description_md: "markdown",
            lisp_source: &loader,
            lisp_editor_path: harness.root(),
            qml_file: "qml",
            pkg_desc_qml: "descriptor",
            qml_is_fullscreen: false,
        })
        .expect("package");
        let fields = parse_vescpkg(&package).expect("vescpkg fields");

        assert_eq!(
            fields
                .iter()
                .map(|field| field.key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "name",
                "description_md",
                "lispData",
                "qmlFile",
                "pkgDescQml",
                "qmlIsFullscreen",
            ]
        );
        assert_eq!(fields[0].value, b"test");
        assert_eq!(fields[1].value, b"markdown");
        assert_eq!(fields[3].value, b"qml");
        assert_eq!(fields[4].value, b"descriptor");
        assert_eq!(fields[5].value, [0]);
    }

    #[test]
    fn generated_ble_package_pins_the_expected_field_sizes_and_native_import_layout() {
        let mut native_payload = (0..205).map(|byte| byte as u8).collect::<Vec<_>>();
        native_payload[204] = 0;
        let harness = PackageTestHarness::new().write_native_payload(&native_payload);

        let assets = PackageAssets::new(
            PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, "0.1.0"),
            PackageProvenance::empty(),
        );
        let package = build_vesc_package(&VescPackageInput {
            name: assets.package_name(),
            description_md: &assets.render_readme(),
            lisp_source: &assets.render_loader(),
            lisp_editor_path: harness.root(),
            qml_file: "",
            pkg_desc_qml: &assets.render_descriptor(),
            qml_is_fullscreen: false,
        })
        .expect("package");
        let fields = parse_vescpkg(&package).expect("vescpkg fields");

        assert_eq!(
            fields
                .iter()
                .map(|field| (field.key.as_str(), field.value.len()))
                .collect::<Vec<_>>(),
            vec![
                ("name", 30),
                ("description_md", 37),
                ("lispData", 371),
                ("pkgDescQml", 336),
                ("qmlIsFullscreen", 1),
            ]
        );

        let lisp_data = extract_field(&package, "lispData");
        let (code, imports) = parse_lisp_imports(&lisp_data).expect("lisp imports");
        assert_eq!(
            code,
            "; Auto-generated loader for the Rust BLE loopback test package.\n(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n"
        );
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].tag, "package-lib");
        assert_eq!(imports[0].offset, 164);
        assert_eq!(imports[0].size, 205);
        assert!(payload_matches_native_with_only_nul_tail(
            &imports[0].payload,
            &native_payload
        ));
    }

    #[test]
    fn native_import_payload_preserves_the_loader_header_prefix() {
        let native_payload = [
            0x00, 0x00, 0x00, 0x00, // .program_ptr placeholder
            0x08, 0xb5, 0x09, 0x4b, // current Thumb init prologue prefix
            0x09, 0x4a, 0x7b, 0x44,
        ];
        let harness = PackageTestHarness::new().write_native_payload(native_payload);

        let assets = PackageAssets::new(
            PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, "0.1.0"),
            PackageProvenance::empty(),
        );
        let package = build_vesc_package(&VescPackageInput {
            name: assets.package_name(),
            description_md: &assets.render_readme(),
            lisp_source: &assets.render_loader(),
            lisp_editor_path: harness.root(),
            qml_file: "",
            pkg_desc_qml: &assets.render_descriptor(),
            qml_is_fullscreen: false,
        })
        .expect("package");

        let lisp_data = extract_field(&package, "lispData");
        let (_, imports) = parse_lisp_imports(&lisp_data).expect("lisp imports");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].tag, "package-lib");
        assert_eq!(&imports[0].payload[..native_payload.len()], &native_payload);
        assert!(payload_matches_native_with_only_nul_tail(
            &imports[0].payload,
            &native_payload
        ));
    }
}
