use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use flate2::{Compression, write::ZlibEncoder};

const PACKAGE_MAGIC: &str = "VESC Packet";

/// Source inputs used to build a VESC package archive.
#[derive(Debug, Clone)]
pub struct VescPackageInput<'a> {
    /// Package name embedded in the archive.
    pub name: &'a str,
    /// Markdown description used for the generated README.
    pub description_md: &'a str,
    /// Lisp loader source before native-payload packing.
    pub lisp_source: &'a str,
    /// Workspace path used to resolve Lisp imports.
    pub lisp_editor_path: &'a Path,
    /// QML source embedded in the package.
    pub qml_file: &'a str,
    /// `pkgdesc.qml` descriptor contents.
    pub pkg_desc_qml: &'a str,
    /// Whether the package's QML app should run fullscreen.
    pub qml_is_fullscreen: bool,
}

/// Fully materialized package fields written to the VESC package wire format.
#[derive(Debug, Clone)]
pub struct VescPackageWire<'a> {
    /// Package name field.
    pub name: &'a str,
    /// Plain-text description derived from the markdown source.
    pub description: &'a str,
    /// Markdown description field.
    pub description_md: &'a str,
    /// Packed Lisp payload bytes.
    pub lisp_data: &'a [u8],
    /// QML source field.
    pub qml_file: &'a str,
    /// `pkgdesc.qml` descriptor field.
    pub pkg_desc_qml: &'a str,
    /// Whether the package's QML app should run fullscreen.
    pub qml_is_fullscreen: bool,
}

/// Encode a decoded package back to `.vescpkg` bytes without repacking Lisp imports.
pub fn encode_vesc_package(wire: &VescPackageWire<'_>) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    append_string(&mut data, PACKAGE_MAGIC);

    append_text_field(&mut data, "name", wire.name)?;
    if !wire.description_md.is_empty() {
        append_text_field(&mut data, "description_md", wire.description_md)?;
    } else {
        append_text_field(&mut data, "description", wire.description)?;
    }
    append_bytes_field(&mut data, "lispData", wire.lisp_data)?;
    append_text_field(&mut data, "qmlFile", wire.qml_file)?;
    append_text_field(&mut data, "pkgDescQml", wire.pkg_desc_qml)?;

    append_string(&mut data, "qmlIsFullscreen");
    append_i32_be(&mut data, 1);
    data.push(u8::from(wire.qml_is_fullscreen));

    q_compress(&data)
}

/// Packs Lisp source and its native imports into the package Lisp payload format.
pub fn build_lisp_data(lisp_source: &str, lisp_editor_path: &Path) -> io::Result<Vec<u8>> {
    pack_lisp_imports(lisp_source, lisp_editor_path)
}

/// Builds compressed VESC package bytes from source package inputs.
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

/// Builds a VESC package and writes the resulting bytes to `output_path`.
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
    use super::parse_import_line;
    use super::{VescPackageInput, build_vesc_package};
    use crate::package_wire::{LispImport, field_bytes, parse_lisp_imports, parse_vescpkg};
    use crate::test_support::PackageTestHarness;

    fn extract_field(package: &[u8], key: &str) -> Vec<u8> {
        field_bytes(&parse_vescpkg(package).expect("vescpkg-rs"), key)
            .expect("missing field")
            .to_vec()
    }

    #[test]
    fn parse_import_line_reads_path_and_tag_from_lisp_import_forms() {
        assert_eq!(
            parse_import_line("(import \"src/package_lib.bin\" 'package-lib)"),
            Some(("src/package_lib.bin".to_owned(), "package-lib".to_owned()))
        );
        assert_eq!(
            parse_import_line("  (import \"relative/path.bin\" 'my-tag) ; comment"),
            Some(("relative/path.bin".to_owned(), "my-tag".to_owned()))
        );
        assert_eq!(parse_import_line("(load-native-lib package-lib)"), None);
        assert_eq!(parse_import_line("(import \"\" 'package-lib)"), None);
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
}
