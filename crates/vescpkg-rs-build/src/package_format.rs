use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use flate2::{Compression, write::ZlibEncoder};
use pulldown_cmark::{Event, Options, Parser, html};

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
    /// Optional path used to resolve imports beside the main Lisp file.
    pub lisp_import_path: Option<&'a Path>,
    /// Policy used when resolving Lisp imports from loader source.
    pub lisp_import_policy: LispImportPolicy,
    /// QML source embedded in the package.
    pub qml_file: &'a str,
    /// `pkgdesc.qml` descriptor contents.
    pub pkg_desc_qml: &'a str,
    /// Whether the package's QML app should run fullscreen.
    pub qml_is_fullscreen: bool,
}

/// Path policy for Lisp `(import "...")` payload references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LispImportPolicy {
    /// Preserve the legacy host-path fallback used by built-in example builds.
    HostPaths,
    /// Resolve only staging-relative paths and reject escapes or symlink traversal.
    StagingOnly,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageField<'a> {
    Text { key: &'static str, value: &'a str },
    Bytes { key: &'static str, value: &'a [u8] },
    Bool { key: &'static str, value: bool },
}

/// Encode a decoded package back to `.vescpkg` bytes without repacking Lisp imports.
pub fn encode_vesc_package(wire: &VescPackageWire<'_>) -> io::Result<Vec<u8>> {
    encode_package_fields([
        PackageField::Text {
            key: "name",
            value: wire.name,
        },
        PackageField::Text {
            key: "description",
            value: wire.description,
        },
        PackageField::Text {
            key: "description_md",
            value: wire.description_md,
        },
        PackageField::Bytes {
            key: "lispData",
            value: wire.lisp_data,
        },
        PackageField::Text {
            key: "qmlFile",
            value: wire.qml_file,
        },
        PackageField::Text {
            key: "pkgDescQml",
            value: wire.pkg_desc_qml,
        },
        PackageField::Bool {
            key: "qmlIsFullscreen",
            value: wire.qml_is_fullscreen,
        },
    ])
}

/// Packs Lisp source and its native imports into the package Lisp payload format.
pub fn build_lisp_data(lisp_source: &str, lisp_editor_path: &Path) -> io::Result<Vec<u8>> {
    pack_lisp_imports(
        lisp_source,
        lisp_editor_path,
        None,
        LispImportPolicy::HostPaths,
    )
}

/// Builds compressed VESC package bytes from source package inputs.
///
/// The VESC wire `description` field is rendered HTML derived from
/// `description_md`; the original markdown is also emitted as `description_md`.
pub fn build_vesc_package(input: &VescPackageInput<'_>) -> io::Result<Vec<u8>> {
    let lisp_data = pack_lisp_imports(
        input.lisp_source,
        input.lisp_editor_path,
        input.lisp_import_path,
        input.lisp_import_policy,
    )?;

    let description_html = markdown_description_html(input.description_md);
    encode_package_fields([
        PackageField::Text {
            key: "name",
            value: input.name,
        },
        PackageField::Text {
            key: "description",
            value: &description_html,
        },
        PackageField::Text {
            key: "description_md",
            value: input.description_md,
        },
        PackageField::Bytes {
            key: "lispData",
            value: &lisp_data,
        },
        PackageField::Text {
            key: "qmlFile",
            value: input.qml_file,
        },
        PackageField::Text {
            key: "pkgDescQml",
            value: input.pkg_desc_qml,
        },
        PackageField::Bool {
            key: "qmlIsFullscreen",
            value: input.qml_is_fullscreen,
        },
    ])
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

fn encode_package_fields<'a>(
    fields: impl IntoIterator<Item = PackageField<'a>>,
) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    append_string(&mut data, PACKAGE_MAGIC);
    fields
        .into_iter()
        .try_for_each(|field| field.append_to(&mut data))?;
    q_compress(&data)
}

impl PackageField<'_> {
    fn append_to(self, buf: &mut Vec<u8>) -> io::Result<()> {
        match self {
            Self::Text { key, value } => append_len_prefixed_field(buf, key, value.as_bytes()),
            Self::Bytes { key, value } => append_len_prefixed_field(buf, key, value),
            Self::Bool { key, value } => {
                append_string(buf, key);
                append_i32_be(buf, 1);
                buf.push(u8::from(value));
                Ok(())
            }
        }
    }
}

fn append_len_prefixed_field(buf: &mut Vec<u8>, key: &str, value: &[u8]) -> io::Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    append_string(buf, key);
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

fn append_i16_be(buf: &mut Vec<u8>, value: i16) {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LispImportPayload {
    tag: String,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LispImportTableEntry<'a> {
    tag: &'a str,
    offset: usize,
    size: usize,
}

fn pack_lisp_imports(
    code_str: &str,
    editor_path: &Path,
    import_path: Option<&Path>,
    policy: LispImportPolicy,
) -> io::Result<Vec<u8>> {
    let imports = code_str
        .lines()
        .map(|line| read_lisp_import(line, editor_path, import_path, policy))
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let mut packed = lisp_code_prefix(code_str);

    let file_table_size = imports
        .iter()
        .map(|import| import.tag.len() + 9)
        .sum::<usize>();
    let num_imports = i16::try_from(imports.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "too many Lisp imports for a VESC package",
        )
    })?;
    packed.extend_from_slice(&num_imports.to_be_bytes());

    let payload_start = packed.len() + file_table_size - 2;
    let table_entries = lisp_import_table_entries(&imports, payload_start)?;
    table_entries
        .iter()
        .try_for_each(|entry| append_lisp_import_table_entry(&mut packed, *entry))?;

    imports
        .iter()
        .for_each(|import| append_aligned_lisp_payload(&mut packed, &import.data));

    Ok(packed)
}

fn lisp_code_prefix(code_str: &str) -> Vec<u8> {
    let mut packed = Vec::with_capacity(2 + code_str.len() + 1);
    append_i16_be(&mut packed, 0);
    append_null_terminated_bytes(&mut packed, code_str.as_bytes());
    packed
}

fn append_null_terminated_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(bytes);
    if !bytes.ends_with(&[0]) {
        buf.push(0);
    }
}

fn read_lisp_import(
    line: &str,
    editor_path: &Path,
    import_path: Option<&Path>,
    policy: LispImportPolicy,
) -> io::Result<Option<LispImportPayload>> {
    parse_import_line(line)
        .map(|(path, tag)| {
            fs::read(resolve_import_path(
                editor_path,
                import_path,
                &path,
                policy,
            )?)
            .map(|mut data| {
                data.push(0);
                LispImportPayload { tag, data }
            })
        })
        .transpose()
}

fn lisp_import_table_entries<'a>(
    imports: &'a [LispImportPayload],
    payload_start: usize,
) -> io::Result<Vec<LispImportTableEntry<'a>>> {
    imports
        .iter()
        .try_fold(
            (Vec::with_capacity(imports.len()), payload_start),
            |(mut entries, offset), import| {
                let aligned_offset = align_lisp_payload_offset(offset)?;
                entries.push(LispImportTableEntry {
                    tag: &import.tag,
                    offset: aligned_offset,
                    size: import.data.len(),
                });
                Ok((
                    entries,
                    aligned_offset
                        .checked_add(import.data.len())
                        .ok_or_else(lisp_import_offset_overflow)?,
                ))
            },
        )
        .map(|(entries, _)| entries)
}

fn append_lisp_import_table_entry(
    packed: &mut Vec<u8>,
    entry: LispImportTableEntry<'_>,
) -> io::Result<()> {
    append_string(packed, entry.tag);
    append_i32_be(
        packed,
        i32::try_from(entry.offset).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Lisp import offset exceeds the VESC package limit",
            )
        })?,
    );
    append_i32_be(
        packed,
        i32::try_from(entry.size).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Lisp import payload exceeds the VESC package limit",
            )
        })?,
    );
    Ok(())
}

fn append_aligned_lisp_payload(packed: &mut Vec<u8>, data: &[u8]) {
    let padding = (4 - ((packed.len() - 2) % 4)) % 4;
    packed.extend(std::iter::repeat_n(0, padding));
    packed.extend_from_slice(data);
}

fn align_lisp_payload_offset(offset: usize) -> io::Result<usize> {
    offset
        .checked_add(3)
        .map(|value| value & !3)
        .ok_or_else(lisp_import_offset_overflow)
}

fn lisp_import_offset_overflow() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Lisp import offset exceeds the VESC package limit",
    )
}

fn markdown_description_html(markdown: &str) -> String {
    if markdown.is_empty() {
        return String::new();
    }

    let mut rendered = String::from(
        "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.0//EN\" \"http://www.w3.org/TR/REC-html40/strict.dtd\">\n",
    );
    let events = Parser::new_ext(markdown, Options::empty()).map(|event| match event {
        Event::Html(html) => Event::Text(html),
        Event::InlineHtml(html) => Event::Text(html),
        event => event,
    });
    html::push_html(&mut rendered, events);
    rendered
}

fn resolve_import_path(
    editor_path: &Path,
    lisp_import_path: Option<&Path>,
    import_path: &str,
    policy: LispImportPolicy,
) -> io::Result<PathBuf> {
    match policy {
        LispImportPolicy::HostPaths => Ok([Some(editor_path), lisp_import_path]
            .into_iter()
            .flatten()
            .map(|base_path| base_path.join(import_path))
            .find(|candidate| candidate.exists())
            .unwrap_or_else(|| PathBuf::from(import_path))),
        LispImportPolicy::StagingOnly => {
            resolve_staged_import_path(editor_path, lisp_import_path, import_path)
        }
    }
}

fn resolve_staged_import_path(
    staging_dir: &Path,
    lisp_import_path: Option<&Path>,
    import_path: &str,
) -> io::Result<PathBuf> {
    let relative = staging_relative_import_path(import_path)?;
    let mut missing_candidate = None;
    for base_path in [Some(staging_dir), lisp_import_path].into_iter().flatten() {
        let candidate = base_path.join(&relative);
        reject_symlink_path(base_path, &candidate)?;
        if candidate.exists() {
            let canonical_base = base_path.canonicalize()?;
            let canonical_candidate = candidate.canonicalize()?;
            if canonical_candidate.starts_with(canonical_base) {
                return Ok(candidate);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Lisp import must stay inside the staging directory: {}",
                    candidate.display()
                ),
            ));
        }
        missing_candidate.get_or_insert(candidate);
    }
    Ok(missing_candidate.unwrap_or_else(|| staging_dir.join(relative)))
}

fn staging_relative_import_path(import_path: &str) -> io::Result<PathBuf> {
    let path = Path::new(import_path);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Lisp import must be relative to the staging directory: {import_path}"),
        ));
    }
    Ok(path.to_path_buf())
}

fn reject_symlink_path(base_path: &Path, path: &Path) -> io::Result<()> {
    let relative = path.strip_prefix(base_path).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Lisp import must stay inside the staging directory: {}",
                path.display()
            ),
        )
    })?;
    let mut current = base_path.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Lisp imports must not traverse symlinks: {}",
                        current.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    Ok(())
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
    use std::path::Path;

    use super::{LispImportPolicy, VescPackageInput, build_vesc_package};
    use super::{lisp_code_prefix, parse_import_line, q_compress, resolve_import_path};
    use crate::package_wire::{LispImport, field_bytes, parse_lisp_imports, parse_vescpkg};
    use crate::test_support::PackageTestHarness;

    fn extract_field(package: &[u8], key: &str) -> Vec<u8> {
        field_bytes(&parse_vescpkg(package).expect("vescpkg-rs"), key)
            .expect("missing field")
            .to_vec()
    }

    #[test]
    fn q_compress_matches_qt_zlib_level_9() {
        let raw = [
            b"VeSC Packet\0name\0".as_slice(),
            &(0u8..64).cycle().take(512).collect::<Vec<_>>(),
            b"qmlFile\0".as_slice(),
            &[b'A'; 200],
        ]
        .concat();

        assert_eq!(
            q_compress(&raw).expect("compressed"),
            [
                0, 0, 2, 225, 120, 218, 11, 75, 13, 118, 86, 8, 72, 76, 206, 78, 45, 97, 200, 75,
                204, 77, 101, 96, 96, 100, 98, 102, 97, 101, 99, 231, 224, 228, 226, 230, 225, 229,
                227, 23, 16, 20, 18, 22, 17, 21, 19, 151, 144, 148, 146, 150, 145, 149, 147, 87,
                80, 84, 82, 86, 81, 85, 83, 215, 208, 212, 210, 214, 209, 213, 211, 55, 48, 52, 50,
                54, 49, 53, 51, 183, 176, 180, 178, 182, 177, 181, 179, 31, 213, 63, 180, 245, 23,
                230, 230, 184, 101, 230, 164, 50, 56, 14, 19, 0, 0, 71, 243, 121, 253,
            ]
        );
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
    fn lisp_code_prefix_stores_null_terminated_source_after_reserved_count() {
        assert_eq!(
            lisp_code_prefix("(code)").as_slice(),
            [0, 0, b'(', b'c', b'o', b'd', b'e', b')', 0]
        );
        assert_eq!(
            lisp_code_prefix("(code)\0").as_slice(),
            [0, 0, b'(', b'c', b'o', b'd', b'e', b')', 0]
        );
    }

    #[test]
    fn resolve_import_path_tries_editor_path_then_lisp_import_path_then_raw_path() {
        let harness = PackageTestHarness::new()
            .write_bytes("editor.bin", [1])
            .write_bytes("imports/native.bin", [2]);
        let import_root = harness.root().join("imports");

        assert_eq!(
            resolve_import_path(
                harness.root(),
                Some(&import_root),
                "editor.bin",
                LispImportPolicy::HostPaths
            )
            .expect("editor import path"),
            harness.root().join("editor.bin")
        );
        assert_eq!(
            resolve_import_path(
                harness.root(),
                Some(&import_root),
                "native.bin",
                LispImportPolicy::HostPaths
            )
            .expect("native import path"),
            import_root.join("native.bin")
        );
        assert_eq!(
            resolve_import_path(
                harness.root(),
                Some(&import_root),
                "missing.bin",
                LispImportPolicy::HostPaths
            )
            .expect("missing import path"),
            std::path::PathBuf::from("missing.bin")
        );
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
            lisp_import_path: None,
            lisp_import_policy: LispImportPolicy::HostPaths,
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
    fn lisp_imports_append_terminators_and_align_payloads() {
        let harness = PackageTestHarness::new()
            .write_bytes("first.bin", [1, 2, 0])
            .write_bytes("second.bin", [3]);
        let loader = "(import \"first.bin\" 'first)\n(import \"second.bin\" 'second)\n";

        let package = build_vesc_package(&VescPackageInput {
            name: "test",
            description_md: "",
            lisp_source: loader,
            lisp_editor_path: harness.root(),
            lisp_import_path: None,
            lisp_import_policy: LispImportPolicy::HostPaths,
            qml_file: "",
            pkg_desc_qml: "",
            qml_is_fullscreen: false,
        })
        .expect("package");
        let lisp_data = extract_field(&package, "lispData");
        let (_, imports) = parse_lisp_imports(&lisp_data).expect("lisp imports");

        assert_eq!(imports[0].payload, [1, 2, 0, 0]);
        assert_eq!(imports[1].payload, [3, 0]);
        assert_eq!(imports[0].offset % 4, 0);
        assert_eq!(imports[1].offset % 4, 0);
        assert_eq!(imports[1].offset, imports[0].offset + imports[0].size);
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
            lisp_import_path: None,
            lisp_import_policy: LispImportPolicy::HostPaths,
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
                "description",
                "description_md",
                "lispData",
                "qmlFile",
                "pkgDescQml",
                "qmlIsFullscreen",
            ]
        );
        assert_eq!(fields[0].value, b"test");
        assert_eq!(
            fields[1].value,
            br#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.0//EN" "http://www.w3.org/TR/REC-html40/strict.dtd">
<p>markdown</p>
"#
        );
        assert_eq!(fields[2].value, b"markdown");
        assert_eq!(fields[4].value, b"qml");
        assert_eq!(fields[5].value, b"descriptor");
        assert_eq!(fields[6].value, [0]);
    }

    #[test]
    fn markdown_description_escapes_raw_html() {
        let package = build_vesc_package(&VescPackageInput {
            name: "test",
            description_md: "**safe** <script>alert(1)</script>\n\n<div>raw</div>",
            lisp_source: "",
            lisp_editor_path: Path::new("."),
            lisp_import_path: None,
            lisp_import_policy: LispImportPolicy::HostPaths,
            qml_file: "",
            pkg_desc_qml: "",
            qml_is_fullscreen: false,
        })
        .expect("package");
        let description = extract_field(&package, "description");
        let html = String::from_utf8(description).expect("html");

        assert!(html.contains("<strong>safe</strong>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("&lt;div&gt;raw&lt;/div&gt;"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<div>raw</div>"));
    }
}
