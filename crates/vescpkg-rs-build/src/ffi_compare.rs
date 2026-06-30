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
    let end_line = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("} vesc_c_if") || trimmed.starts_with("} vesc_if")
    });
    let Some(end_line) = end_line else {
        return Vec::new();
    };
    let Some(start_line) = lines[..end_line]
        .iter()
        .rposition(|line| line.trim() == "typedef struct {")
    else {
        return Vec::new();
    };

    let mut fields = Vec::new();
    let mut pending_decl = String::new();
    let mut pending_line = 0;

    for (offset, line) in lines[start_line + 1..end_line].iter().enumerate() {
        let line_index = start_line + 1 + offset;
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with("/*") || line.starts_with('*') {
            continue;
        }
        if pending_decl.is_empty() {
            pending_line = line_index + 1;
        }
        pending_decl.push(' ');
        pending_decl.push_str(line);

        if !line.contains(';') {
            continue;
        }

        if let Some(name) = parse_c_field_name(&pending_decl) {
            fields.push(Field {
                name,
                line: pending_line,
            });
        }
        pending_decl.clear();
    }

    fields
}

fn parse_c_field_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(start) = line.find("(*") {
        let rest = &line[start + 2..];
        let end = rest.find(')')?;
        let name = rest[..end].trim();
        return (!name.is_empty()).then(|| name.to_owned());
    }

    if !line.ends_with(';') {
        return None;
    }

    let without_semicolon = line.trim_end_matches(';').trim();
    let token = without_semicolon.split_whitespace().last()?;
    if token.contains('[') {
        return None;
    }
    let name = token.trim_matches('*').trim();
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
        CompareError, GPIO_USED_SLOTS, LOOPBACK_USED_SLOTS, compare_used_slots_from_paths,
        default_header_path, default_rust_table_path, parse_rust_vesc_if_fields, slots_present,
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
    fn fixture_header_cannot_match_full_rust_order() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let result = compare_used_slots_from_paths(
            &default_header_path(&manifest),
            &default_rust_table_path(&manifest),
            LOOPBACK_USED_SLOTS,
        );
        assert!(matches!(
            result,
            Err(CompareError::SlotOrderMismatch { .. })
        ));
    }
}
