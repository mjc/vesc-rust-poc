use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::native_audit::defined_symbols;
use crate::native_inspect::{SectionLayout, all_section_layouts, nm_output};

const LOAD_SECTIONS: [&str; 5] = [".program_ptr", ".init_fun", ".data", ".got", ".text"];
const CONTRACT_SYMBOLS: [&str; 11] = [
    "init",
    "prog_ptr",
    "package_lib_init",
    "refloat_app_data_callback",
    "refloat_get_cfg",
    "refloat_set_cfg",
    "refloat_get_cfg_xml",
    "loopback_handle_app_data",
    "ext_rust_probe_diag_v4",
    "vesc_register_loopback_app_data_handler",
    "vesc_clear_loopback_app_data_handler",
];

/// Compare two native ELF binaries by review-stable loader sections and symbols.
pub fn native_binary_comparison_report(
    left_label: &str,
    left_elf: &Path,
    right_label: &str,
    right_elf: &Path,
) -> String {
    let left_sections = all_section_layouts(left_elf);
    let right_sections = all_section_layouts(right_elf);
    let left_symbols = defined_symbols(&nm_output(left_elf));
    let right_symbols = defined_symbols(&nm_output(right_elf));

    let mut lines = vec![format!(
        "native_binary: left={left_label} right={right_label}"
    )];

    lines.push("load_sections:".to_owned());
    lines.extend(
        LOAD_SECTIONS
            .iter()
            .map(|name| section_comparison_line(name, &left_sections, &right_sections)),
    );

    lines.push("debug_sections:".to_owned());
    lines.extend(
        debug_section_names(&left_sections, &right_sections)
            .iter()
            .map(|name| section_comparison_line(name, &left_sections, &right_sections)),
    );

    lines.push("contract_symbols:".to_owned());
    lines.extend(
        contract_symbol_names(&left_symbols, &right_symbols)
            .iter()
            .map(|name| symbol_comparison_line(name, &left_symbols, &right_symbols)),
    );
    lines.push(format!(
        "defined_symbol_count: left={} right={}",
        left_symbols.len(),
        right_symbols.len()
    ));

    lines.join("\n")
}

fn debug_section_names(
    left: &BTreeMap<String, SectionLayout>,
    right: &BTreeMap<String, SectionLayout>,
) -> BTreeSet<String> {
    left.keys()
        .chain(right.keys())
        .filter(|name| name.starts_with(".debug") || matches!(name.as_str(), ".symtab" | ".strtab"))
        .cloned()
        .collect()
}

fn contract_symbol_names(left: &BTreeSet<String>, right: &BTreeSet<String>) -> BTreeSet<String> {
    left.iter()
        .chain(right.iter())
        .filter_map(|name| stable_contract_symbol_name(name))
        .collect()
}

fn stable_contract_symbol_name(name: &str) -> Option<String> {
    if CONTRACT_SYMBOLS.contains(&name) {
        Some(name.to_owned())
    } else if name.contains("stop_callback") {
        Some("stop_callback".to_owned())
    } else if name.contains("stop_package") {
        Some("stop_package".to_owned())
    } else {
        None
    }
}

fn section_comparison_line(
    name: &str,
    left: &BTreeMap<String, SectionLayout>,
    right: &BTreeMap<String, SectionLayout>,
) -> String {
    match (left.get(name), right.get(name)) {
        (Some(left), Some(right)) if left.size == right.size && left.vma == right.vma => {
            format!("  {name}: match size={} vma={:#x}", left.size, left.vma)
        }
        (Some(left), Some(right)) => format!(
            "  {name}: left size={} vma={:#x} right size={} vma={:#x}",
            left.size, left.vma, right.size, right.vma
        ),
        (Some(left), None) => format!("  {name}: left_only size={} vma={:#x}", left.size, left.vma),
        (None, Some(right)) => {
            format!(
                "  {name}: right_only size={} vma={:#x}",
                right.size, right.vma
            )
        }
        (None, None) => format!("  {name}: missing"),
    }
}

fn symbol_comparison_line(name: &str, left: &BTreeSet<String>, right: &BTreeSet<String>) -> String {
    match (
        contains_contract_symbol(left, name),
        contains_contract_symbol(right, name),
    ) {
        (true, true) => format!("  {name}: both"),
        (true, false) => format!("  {name}: left_only"),
        (false, true) => format!("  {name}: right_only"),
        (false, false) => format!("  {name}: missing"),
    }
}

fn contains_contract_symbol(symbols: &BTreeSet<String>, expected: &str) -> bool {
    symbols
        .iter()
        .filter_map(|name| stable_contract_symbol_name(name))
        .any(|name| name == expected)
}
