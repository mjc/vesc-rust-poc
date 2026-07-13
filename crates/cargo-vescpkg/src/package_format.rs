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
    /// QML source embedded in the package.
    pub qml_file: &'a str,
    /// `pkgdesc.qml` descriptor contents.
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

/// Builds compressed VESC package bytes from source package inputs.
///
/// The VESC wire `description` field is rendered HTML derived from
/// `description_md`; the original markdown is also emitted as `description_md`.
pub fn build_vesc_package(input: &VescPackageInput<'_>) -> io::Result<Vec<u8>> {
    let lisp_data = pack_lisp_imports(input.lisp_source, input.lisp_editor_path)?;

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

fn pack_lisp_imports(code_str: &str, editor_path: &Path) -> io::Result<Vec<u8>> {
    let imports = code_str
        .lines()
        .map(|line| read_lisp_import(line, editor_path))
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

fn read_lisp_import(line: &str, editor_path: &Path) -> io::Result<Option<LispImportPayload>> {
    parse_import_line(line)
        .map(|(path, tag)| {
            fs::read(resolve_staged_import_path(editor_path, &path)?).map(|mut data| {
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

fn resolve_staged_import_path(staging_dir: &Path, import_path: &str) -> io::Result<PathBuf> {
    let relative = staging_relative_import_path(import_path)?;
    let candidate = staging_dir.join(&relative);
    reject_symlink_path(staging_dir, &candidate)?;
    if candidate.exists() {
        let canonical_base = staging_dir.canonicalize()?;
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
    Ok(candidate)
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
