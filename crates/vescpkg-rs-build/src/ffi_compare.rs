use std::path::{Path, PathBuf};

/// One field parsed from the C or Rust VESC interface table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// 1-based source line where the field was found.
    pub line: usize,
}

/// Errors returned while comparing the C and Rust VESC interface tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// Reading one of the source files failed.
    ReadFailed {
        /// Path that failed to read.
        path: PathBuf,
        /// Read failure reason.
        reason: String,
    },
    /// One side was missing a required slot.
    SlotMissing {
        /// Missing slot name.
        slot: String,
        /// Side that was missing the slot.
        side: &'static str,
    },
    /// A shared slot appeared at a different index on the two sides.
    SlotOrderMismatch {
        /// Mismatched slot name.
        slot: String,
        /// Slot index in the C table.
        c_index: usize,
        /// Slot index in the Rust table.
        rust_index: usize,
    },
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed { path, reason } => {
                write!(f, "failed to read {}: {reason}", path.display())
            }
            Self::SlotMissing { slot, side } => write!(f, "{side} table missing slot {slot}"),
            Self::SlotOrderMismatch {
                slot,
                c_index,
                rust_index,
            } => write!(
                f,
                "slot {slot} order mismatch: C index {c_index}, Rust index {rust_index}"
            ),
        }
    }
}

/// Slots used by the loopback package path.
pub const LOOPBACK_USED_SLOTS: &[&str] = &[
    "lbm_add_extension",
    "lbm_enc_i",
    "lbm_dec_as_i32",
    "lbm_is_number",
    "lbm_enc_sym_eerror",
    "send_app_data",
    "set_app_data_handler",
    "system_time_ticks",
];

/// Slots used by the GPIO path.
pub const GPIO_USED_SLOTS: &[&str] = &["io_set_mode", "io_write", "io_read"];

/// All pinned VESC interface slots currently depended on by this workspace.
pub const ALL_PINNED_USED_SLOTS: &[&str] = &[
    "lbm_add_extension",
    "lbm_enc_i",
    "lbm_dec_as_i32",
    "lbm_is_number",
    "lbm_enc_sym_eerror",
    "send_app_data",
    "set_app_data_handler",
    "system_time_ticks",
    "io_set_mode",
    "io_write",
    "io_read",
];

/// Return the workspace root from a crate manifest directory.
pub fn workspace_root_from_manifest(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("../..")
}

/// Return the default C header path, honoring `VESC_C_IF_HEADER` when set.
pub fn default_header_path(manifest_dir: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("VESC_C_IF_HEADER") {
        return PathBuf::from(path);
    }
    workspace_root_from_manifest(manifest_dir).join("fixtures/native-lib-baseline/src/vesc_c_if.h")
}

/// Return the default Rust VESC interface table path.
pub fn default_rust_table_path(manifest_dir: &Path) -> PathBuf {
    workspace_root_from_manifest(manifest_dir).join("crates/vescpkg-rs-sys/src/raw.rs")
}

/// Compare the used slots from the C header and Rust source files.
pub fn compare_used_slots_from_paths(
    c_header: &Path,
    rust_source: &Path,
    slots: &[&str],
) -> Result<(), CompareError> {
    let c_source = std::fs::read_to_string(c_header).map_err(|error| CompareError::ReadFailed {
        path: c_header.to_path_buf(),
        reason: error.to_string(),
    })?;
    let rust_source =
        std::fs::read_to_string(rust_source).map_err(|error| CompareError::ReadFailed {
            path: rust_source.to_path_buf(),
            reason: error.to_string(),
        })?;
    compare_used_slots(&c_source, &rust_source, slots)
}

/// Compare the used slot ordering between the C and Rust tables.
pub fn compare_used_slots(
    c_source: &str,
    rust_source: &str,
    slots: &[&str],
) -> Result<(), CompareError> {
    let c_fields = parse_c_vesc_if_fields(c_source);
    let rust_fields = parse_rust_vesc_if_fields(rust_source);

    for slot in slots {
        let c_index = c_fields
            .iter()
            .position(|field| field.name == *slot)
            .ok_or(CompareError::SlotMissing {
                slot: (*slot).to_owned(),
                side: "C",
            })?;
        let rust_index = rust_fields
            .iter()
            .position(|field| field.name == *slot)
            .ok_or(CompareError::SlotMissing {
                slot: (*slot).to_owned(),
                side: "Rust",
            })?;
        if c_index != rust_index {
            return Err(CompareError::SlotOrderMismatch {
                slot: (*slot).to_owned(),
                c_index,
                rust_index,
            });
        }
    }

    Ok(())
}

/// Assert that the given slot names are present in the C table source.
pub fn slots_present(c_source: &str, slots: &[&str]) -> Result<(), CompareError> {
    let c_fields = parse_c_vesc_if_fields(c_source);
    for slot in slots {
        if !c_fields.iter().any(|field| field.name == *slot) {
            return Err(CompareError::SlotMissing {
                slot: (*slot).to_owned(),
                side: "C",
            });
        }
    }
    Ok(())
}

/// Return full-table mismatches between the C and Rust interface tables.
pub fn compare_full_table(c_source: &str, rust_source: &str) -> Vec<(usize, Field, Option<Field>)> {
    let c_fields = parse_c_vesc_if_fields(c_source);
    let rust_fields = parse_rust_vesc_if_fields(rust_source);
    let max_len = c_fields.len().max(rust_fields.len());
    (0..max_len)
        .filter_map(|index| {
            let c = c_fields.get(index)?;
            let rust = rust_fields.get(index);
            if rust.is_some_and(|rust| rust.name == c.name) {
                return None;
            }
            Some((index, c.clone(), rust.cloned()))
        })
        .collect()
}

/// Parse field names from the C `vesc_if` struct source.
pub fn parse_c_vesc_if_fields(source: &str) -> Vec<Field> {
    let lines: Vec<&str> = source.lines().collect();
    c_vesc_if_body_lines(&lines)
        .into_iter()
        .filter_map(|(line, source)| {
            c_declaration_fragment(source).map(|fragment| (line, fragment))
        })
        .scan(
            PendingCDeclaration::default(),
            |pending, (line, fragment)| Some(pending.push(line, fragment)),
        )
        .flatten()
        .flat_map(|declaration| {
            parse_c_field(&declaration.source)
                .into_iter()
                .flat_map(|field| field.names())
                .map(move |name| Field {
                    name,
                    line: declaration.line,
                })
        })
        .collect()
}

#[derive(Default)]
struct PendingCDeclaration {
    line: Option<usize>,
    source: String,
}

impl PendingCDeclaration {
    fn push(&mut self, line: usize, fragment: &str) -> Option<CDeclaration> {
        self.line.get_or_insert(line);
        self.source.push(' ');
        self.source.push_str(fragment);

        fragment.contains(';').then(|| {
            let declaration = CDeclaration {
                line: self.line.take().expect("pending declaration line"),
                source: std::mem::take(&mut self.source),
            };
            self.source.clear();
            declaration
        })
    }
}

struct CDeclaration {
    line: usize,
    source: String,
}

struct ParsedCField {
    name: String,
    array_len: Option<usize>,
}

impl ParsedCField {
    fn names(self) -> impl Iterator<Item = String> {
        let name = self.name;
        let array_len = self.array_len;
        (0..array_len.unwrap_or(1)).map(move |index| match array_len {
            Some(_) => format!("{name}[{index}]"),
            None => name.clone(),
        })
    }
}

fn c_vesc_if_body_lines<'a>(lines: &'a [&'a str]) -> Vec<(usize, &'a str)> {
    c_vesc_if_bounds(lines)
        .map(|(start, end)| {
            lines[start + 1..end]
                .iter()
                .enumerate()
                .map(move |(offset, line)| (start + offset + 2, *line))
                .collect()
        })
        .unwrap_or_default()
}

fn c_vesc_if_bounds(lines: &[&str]) -> Option<(usize, usize)> {
    let end = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("} vesc_c_if") || trimmed.starts_with("} vesc_if")
    })?;
    let start = lines[..end]
        .iter()
        .rposition(|line| line.trim() == "typedef struct {")?;
    Some((start, end))
}

fn c_declaration_fragment(line: &str) -> Option<&str> {
    line.split("//")
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("/*"))
        .filter(|line| !line.starts_with('*'))
}

fn parse_c_field(declaration: &str) -> Option<ParsedCField> {
    c_function_pointer_field(declaration)
        .or_else(|| c_array_field(declaration))
        .or_else(|| c_scalar_field(declaration))
}

fn c_function_pointer_field(declaration: &str) -> Option<ParsedCField> {
    declaration.find("(*").and_then(|start| {
        let rest = &declaration[start + 2..];
        rest.find(')').and_then(|end| {
            non_empty_name(rest[..end].trim()).map(|name| ParsedCField {
                name,
                array_len: None,
            })
        })
    })
}

fn c_array_field(declaration: &str) -> Option<ParsedCField> {
    c_decl_token(declaration).and_then(|token| {
        let (name, len) = token.trim_matches('*').split_once('[')?;
        let len = len.strip_suffix(']')?.parse().ok()?;
        non_empty_name(name).map(|name| ParsedCField {
            name,
            array_len: Some(len),
        })
    })
}

fn c_scalar_field(declaration: &str) -> Option<ParsedCField> {
    c_decl_token(declaration).and_then(|token| {
        let name = token.trim_matches('*').trim();
        (!name.contains('[')).then_some(name).and_then(|name| {
            non_empty_name(name).map(|name| ParsedCField {
                name,
                array_len: None,
            })
        })
    })
}

fn c_decl_token(declaration: &str) -> Option<&str> {
    declaration
        .trim()
        .strip_suffix(';')
        .map(str::trim)
        .and_then(|declaration| declaration.split_whitespace().last())
}

fn non_empty_name(name: &str) -> Option<String> {
    (!name.is_empty()).then(|| name.to_owned())
}

/// Parse field names from the Rust `VescIf` struct source.
pub fn parse_rust_vesc_if_fields(source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut in_table = false;

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "pub struct VescIf {" {
            in_table = true;
            continue;
        }
        if in_table && trimmed == "}" {
            break;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            fields.push(Field {
                name: name.to_owned(),
                line: line_index + 1,
            });
        }
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::{
        GPIO_USED_SLOTS, LOOPBACK_USED_SLOTS, compare_used_slots_from_paths, default_header_path,
        default_rust_table_path, parse_rust_vesc_if_fields, slots_present,
    };
    use std::path::PathBuf;

    #[test]
    fn parses_rust_vesc_if_field_names_from_raw_rs() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let raw = std::fs::read_to_string(default_rust_table_path(&manifest)).expect("raw.rs");
        let fields = parse_rust_vesc_if_fields(&raw);
        assert!(
            fields
                .first()
                .is_some_and(|field| field.name == "lbm_add_extension")
        );
        assert!(fields.iter().any(|field| field.name == "shutdown_disable"));
    }

    #[test]
    fn fixture_header_contains_loopback_used_slots() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let header = std::fs::read_to_string(default_header_path(&manifest)).expect("header");
        slots_present(&header, LOOPBACK_USED_SLOTS).expect("loopback slots present");
    }

    #[test]
    fn gpio_used_slots_exist_in_rust_table() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let raw = std::fs::read_to_string(default_rust_table_path(&manifest)).expect("raw.rs");
        for slot in GPIO_USED_SLOTS {
            assert!(
                parse_rust_vesc_if_fields(&raw)
                    .iter()
                    .any(|field| field.name == *slot),
                "missing rust slot {slot}"
            );
        }
    }

    #[test]
    fn refloat_header_matches_pinned_used_slot_order_when_available() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let refloat = PathBuf::from("/Users/mjc/projects/refloat/vesc_pkg_lib/vesc_c_if.h");
        if !refloat.is_file() {
            return;
        }
        compare_used_slots_from_paths(
            &refloat,
            &default_rust_table_path(&manifest),
            super::ALL_PINNED_USED_SLOTS,
        )
        .expect("refloat used-slot order");
    }

    #[test]
    fn refloat_header_full_table_matches_when_available() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let refloat = PathBuf::from("/Users/mjc/projects/refloat/vesc_pkg_lib/vesc_c_if.h");
        if !refloat.is_file() {
            return;
        }
        let c_source = std::fs::read_to_string(&refloat).expect("refloat header");
        let rust_source =
            std::fs::read_to_string(default_rust_table_path(&manifest)).expect("raw.rs");
        let mismatches = super::compare_full_table(&c_source, &rust_source);
        assert!(
            mismatches.is_empty(),
            "full-table mismatches: {mismatches:?}"
        );
    }

    #[test]
    fn fixture_header_matches_loopback_used_slot_order() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        compare_used_slots_from_paths(
            &default_header_path(&manifest),
            &default_rust_table_path(&manifest),
            LOOPBACK_USED_SLOTS,
        )
        .expect("fixture used-slot order");
    }
}
