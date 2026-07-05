use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use capstone::prelude::*;
use object::read::File as ObjectFile;
use object::{Object, ObjectSection, ObjectSymbol};
use sha2::{Digest, Sha256};

use crate::package_wire::{WireError, field_bytes, parse_lisp_imports, parse_vescpkg};

const VESC_IF_TABLE_BASE: u32 = 0x1000_f800;
const PROBE_LISBM_ENCODED_42: u32 = 680;

/// Semantic view of the linked native library used by audit assertions.
pub struct NativeLibSemantics {
    /// Symbol table keyed by resolved virtual address.
    pub symbols: BTreeMap<u64, String>,
    /// Literal-pool words keyed by virtual address.
    pub literal_pools: BTreeMap<u64, u32>,
    /// Decoded instructions from the loader init routine.
    pub init_insns: Vec<DecodedInsn>,
    /// Decoded instructions from the package init routine.
    pub package_init_insns: Vec<DecodedInsn>,
    /// Decoded instructions from the loader stop routine.
    pub stop_insns: Vec<DecodedInsn>,
    /// Decoded instructions from the probe routine.
    pub probe_insns: Vec<DecodedInsn>,
}

/// Single decoded instruction from a native-lib routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInsn {
    /// Virtual address of the instruction.
    pub address: u64,
    /// Decoded mnemonic.
    pub mnemonic: String,
    /// Operand text rendered from disassembly.
    pub operands: String,
}

/// Decodes the linked native ELF into a semantic report structure.
pub fn analyze_native_lib_elf(elf: &Path) -> NativeLibSemantics {
    let bytes = std::fs::read(elf).unwrap_or_else(|error| panic!("read ELF {elf:?}: {error}"));
    let object =
        ObjectFile::parse(&bytes[..]).unwrap_or_else(|error| panic!("parse ELF {elf:?}: {error}"));

    let mut symbols = BTreeMap::new();
    for symbol in object.symbols() {
        if symbol.is_undefined() {
            continue;
        }
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() || name.starts_with('$') {
            continue;
        }
        symbols
            .entry(symbol_vma(symbol.address()))
            .or_insert_with(|| name.to_owned());
    }

    let cs = Capstone::new()
        .arm()
        .mode(arch::arm::ArchMode::Thumb)
        .build()
        .expect("capstone ARM thumb");

    let mut literal_pools = BTreeMap::new();
    let mut init_insns = Vec::new();
    let mut package_init_insns = Vec::new();
    let mut stop_insns = Vec::new();
    let mut probe_insns = Vec::new();
    let mut text_insns = Vec::new();

    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        if name != ".init_fun" && !name.starts_with(".text") {
            continue;
        }
        let address = section.address();
        let Ok(data) = section.data() else {
            continue;
        };
        if data.is_empty() {
            continue;
        }

        let insns = cs
            .disasm_all(data, address)
            .unwrap_or_else(|error| panic!("disassemble {name} in {elf:?}: {error}"));
        for insn in insns.iter() {
            if let Some(target) = pc_relative_literal_target(insn) {
                literal_pools.insert(target, read_u32_le(&bytes, target, &object));
            }
        }

        let decoded = decode_insns(&insns);
        if name == ".init_fun" {
            init_insns = decoded;
        } else {
            text_insns.extend(decoded);
        }
    }

    text_insns.sort_by_key(|insn| insn.address);

    let probe_start = symbol_address(&symbols, "ext_rust_probe_diag_v4");
    let package_init_start = symbol_address(&symbols, "package_lib_init");
    let stop_start = symbols.iter().find_map(|(addr, name)| {
        (name.contains("stop_package") || name.contains("stop_refloat_app_data")).then_some(*addr)
    });

    if let Some(start) = probe_start {
        let end = package_init_start
            .or(stop_start)
            .or_else(|| next_symbol_address(&symbols, start));
        probe_insns = decode_symbol_insns(&object, &cs, start, end);
    }
    if let Some(start) = stop_start {
        stop_insns = decode_symbol_insns(&object, &cs, start, next_symbol_address(&symbols, start));
    }
    if let Some(start) = package_init_start {
        package_init_insns =
            decode_symbol_insns(&object, &cs, start, next_symbol_address(&symbols, start));
    }

    NativeLibSemantics {
        symbols,
        literal_pools,
        init_insns,
        package_init_insns,
        stop_insns,
        probe_insns,
    }
}

/// Renders a stable human-readable summary of native-lib semantics.
pub fn semantic_report(semantics: &NativeLibSemantics) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "symbols: {}",
        semantics
            .symbols
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!(
        "literal_pools: {}",
        semantics
            .literal_pools
            .iter()
            .map(|(addr, word)| format!("{addr:#x}=0x{word:08x}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!("init: {}", insn_summary(&semantics.init_insns)));
    lines.push(format!(
        "package_init: {}",
        insn_summary(&semantics.package_init_insns)
    ));
    lines.push(format!("probe: {}", insn_summary(&semantics.probe_insns)));
    lines.push(format!("stop: {}", insn_summary(&semantics.stop_insns)));
    lines.join("\n")
}

/// Renders a Refloat-oriented native symbol and firmware-call mapping.
pub fn refloat_mapping_report(elf: &Path) -> String {
    let semantics = analyze_native_lib_elf(elf);
    let mut lines = vec!["refloat_native_mapping:".to_owned()];

    lines.push("entrypoints:".to_owned());
    for (label, needle) in [
        ("package_init", "package_lib_init"),
        ("app_data_handler", "refloat_handle_app_data"),
        ("stop_hook", "stop_refloat_app_data"),
        ("main_thread", "refloat_main_thread"),
        ("aux_thread", "refloat_aux_thread"),
    ] {
        lines.push(mapping_symbol_line(label, needle, &semantics.symbols));
    }

    lines.push("runtime_paths:".to_owned());
    for (label, needle) in [
        ("runtime_refresh", "refresh_runtime_state"),
        ("config_get", "refloat_get_cfg"),
        ("config_set", "refloat_set_cfg"),
        ("config_xml", "refloat_get_cfg_xml"),
        ("state_from_arg", "refloat_state_from_arg"),
    ] {
        lines.push(mapping_symbol_line(label, needle, &semantics.symbols));
    }

    lines.push("firmware_slots:".to_owned());
    for (label, slot, insns) in [
        ("malloc", 184, semantics.package_init_insns.as_slice()),
        (
            "set_app_data_handler",
            596,
            semantics.package_init_insns.as_slice(),
        ),
        (
            "clear_app_data_handler",
            596,
            semantics.stop_insns.as_slice(),
        ),
    ] {
        let status = if package_init_touches_slot(insns, slot) {
            "present"
        } else {
            "missing"
        };
        lines.push(format!("  {label}: {status} slot={slot}"));
    }

    lines.join("\n")
}

/// Renders the upstream C Refloat ELF symbols produced before `package_lib.bin`.
pub fn c_refloat_mapping_report(elf: &Path) -> String {
    let semantics = analyze_native_lib_elf(elf);
    let mut lines = vec!["c_refloat_native_mapping:".to_owned()];

    lines.push("entrypoints:".to_owned());
    for (label, needle, source) in [
        ("package_init", "init", "src/main.c:2415"),
        ("main_thread", "refloat_thd", "src/main.c:767"),
        ("aux_thread", "aux_thd", "src/main.c:1130"),
        ("stop_hook", "stop", "src/main.c:2399"),
    ] {
        lines.push(mapping_symbol_exact_source_line(
            label,
            needle,
            source,
            &semantics.symbols,
        ));
    }

    lines.push("runtime_paths:".to_owned());
    for (label, needle, source) in [
        ("configure", "configure", "src/main.c:185"),
        ("config_get", "get_cfg", "src/main.c:2335"),
        ("config_set", "set_cfg", "src/main.c:2360"),
        ("config_xml", "get_cfg_xml", "src/main.c:2389"),
        ("imu_callback", "imu_ref_callback", "src/main.c:760"),
        ("state_compat", "state_compat", "src/state.c:50"),
    ] {
        lines.push(mapping_symbol_exact_source_line(
            label,
            needle,
            source,
            &semantics.symbols,
        ));
    }

    lines.push("config_payload:".to_owned());
    lines.push(mapping_symbol_source_line(
        "serialized_defaults",
        "data_refloatconfig_",
        "src/conf/confxml.c:5",
        &semantics.symbols,
    ));
    for (label, needle, source) in [
        (
            "defaults",
            "confparser_set_defaults_refloatconfig",
            "src/conf/confparser.c:363",
        ),
        (
            "serialize",
            "confparser_serialize_refloatconfig",
            "src/conf/confparser.c:8",
        ),
        (
            "deserialize",
            "confparser_deserialize_refloatconfig",
            "src/conf/confparser.c:184",
        ),
    ] {
        lines.push(mapping_symbol_exact_source_line(
            label,
            needle,
            source,
            &semantics.symbols,
        ));
    }

    lines.join("\n")
}

/// Renders a side-by-side C baseline to Rust-native Refloat symbol map.
pub fn refloat_c_rust_mapping_report(c_elf: &Path, rust_elf: &Path) -> String {
    let c_semantics = analyze_native_lib_elf(c_elf);
    let rust_semantics = analyze_native_lib_elf(rust_elf);
    let mut lines = vec!["refloat_c_rust_mapping:".to_owned()];

    lines.push("lifecycle:".to_owned());
    for (label, c_symbol, c_source, rust_needle) in [
        (
            "package_init",
            "init",
            "src/main.c:2415",
            "package_lib_init",
        ),
        (
            "main_thread",
            "refloat_thd",
            "src/main.c:767",
            "refloat_main_thread",
        ),
        (
            "aux_thread",
            "aux_thd",
            "src/main.c:1130",
            "refloat_aux_thread",
        ),
        (
            "stop_hook",
            "stop",
            "src/main.c:2399",
            "stop_refloat_app_data",
        ),
    ] {
        lines.push(mapping_pair_line(
            label,
            c_symbol,
            c_source,
            rust_needle,
            &c_semantics.symbols,
            &rust_semantics.symbols,
        ));
    }

    lines.push("app_communication:".to_owned());
    lines.push(mapping_pair_line(
        "app_data_handler",
        "on_command_received",
        "src/main.c:2143",
        "refloat_handle_app_data",
        &c_semantics.symbols,
        &rust_semantics.symbols,
    ));
    lines.push("  app_data_registration: c=[registered source=src/main.c:2457] rust=[slot=596 set during package_lib_init]".to_owned());

    lines.push("runtime:".to_owned());
    for (label, c_symbol, c_source, rust_needle) in [
        (
            "configure",
            "configure",
            "src/main.c:185",
            "refresh_runtime_state",
        ),
        (
            "imu_refresh",
            "imu_ref_callback",
            "src/main.c:760",
            "refresh_runtime_state",
        ),
        (
            "state_recovery",
            "state_compat",
            "src/state.c:50",
            "refloat_state_from_arg",
        ),
    ] {
        lines.push(mapping_pair_line(
            label,
            c_symbol,
            c_source,
            rust_needle,
            &c_semantics.symbols,
            &rust_semantics.symbols,
        ));
    }

    lines.push("config:".to_owned());
    for (label, c_symbol, c_source, rust_needle) in [
        (
            "config_get",
            "get_cfg",
            "src/main.c:2335",
            "refloat_get_cfg",
        ),
        (
            "config_set",
            "set_cfg",
            "src/main.c:2360",
            "refloat_set_cfg",
        ),
        (
            "config_xml",
            "get_cfg_xml",
            "src/main.c:2389",
            "refloat_get_cfg_xml",
        ),
    ] {
        lines.push(mapping_pair_line(
            label,
            c_symbol,
            c_source,
            rust_needle,
            &c_semantics.symbols,
            &rust_semantics.symbols,
        ));
    }
    for (label, c_symbol, c_source, rust_equivalent) in [
        (
            "c_defaults_blob",
            "data_refloatconfig_",
            "src/conf/confxml.c:5",
            "generated config payload source=examples/refloat/src/conf/refloatconfig.dat:1 include=examples/refloat/src/app_data.rs:45",
        ),
        (
            "c_defaults",
            "confparser_set_defaults_refloatconfig",
            "src/conf/confparser.c:363",
            "generated defaults source=examples/refloat/src/conf/default_config.dat:1 include=examples/refloat/src/app_data.rs:60",
        ),
        (
            "c_serialize",
            "confparser_serialize_refloatconfig",
            "src/conf/confparser.c:8",
            "generated defaults copy source=examples/refloat/src/app_data.rs:910",
        ),
        (
            "c_deserialize",
            "confparser_deserialize_refloatconfig",
            "src/conf/confparser.c:184",
            "serialized config store source=examples/refloat/src/app_data.rs:957",
        ),
    ] {
        lines.push(format!(
            "  {label}: c=[{}] rust=[{rust_equivalent}]",
            symbol_status_exact_at(c_symbol, c_source, &c_semantics.symbols)
        ));
    }

    lines.push("helper_classification:".to_owned());
    lines.extend([
        (
            "implemented_rust_peer",
            "on_command_received->refloat_handle_app_data; ext_set_fw_version->ext_set_fw_version; get_cfg/set_cfg/get_cfg_xml callbacks",
        ),
        (
            "folded_into_existing_path",
            "configure/reconfigure/reset_runtime_vars/state_compat fold into refresh_runtime_state, typed ride-state payloads, and runtime state refresh",
        ),
        (
            "intentionally_omitted",
            "none declared permanent in VESCR-214; remaining helpers need porting or explicit omission evidence",
        ),
        (
            "new_follow_up_ticket",
            "VESCR-215 covers config EEPROM/startup helpers, ride/control helpers, app-data command handlers, ext_bms/fatal_error_terminate, and subsystem helpers",
        ),
    ].into_iter().map(|(label, detail)| format!("  {label}: {detail}")));

    lines.push("rust_firmware_slots:".to_owned());
    for (label, slot, insns) in [
        ("malloc", 184, rust_semantics.package_init_insns.as_slice()),
        (
            "set_app_data_handler",
            596,
            rust_semantics.package_init_insns.as_slice(),
        ),
        (
            "clear_app_data_handler",
            596,
            rust_semantics.stop_insns.as_slice(),
        ),
    ] {
        let status = if package_init_touches_slot(insns, slot) {
            "present"
        } else {
            "missing"
        };
        lines.push(format!("  {label}: {status} slot={slot}"));
    }

    lines.join("\n")
}

/// Renders the mapping evidence available from an official captured Refloat package.
pub fn captured_refloat_baseline_mapping_report(package: &[u8]) -> Result<String, WireError> {
    let fields = parse_vescpkg(package)?;
    let lisp_data = field_bytes(&fields, "lispData").ok_or(WireError::UnexpectedEof)?;
    let (_, imports) = parse_lisp_imports(lisp_data)?;
    let Some(package_lib) = imports.iter().find(|import| import.tag == "package-lib") else {
        return Err(WireError::UnexpectedEof);
    };
    let payload = package_lib.payload.as_slice();

    Ok([
        "captured_refloat_baseline_mapping:".to_owned(),
        format!(
            "  package_lib: size={} sha256={}",
            payload.len(),
            sha256_hex(payload)
        ),
        format!("  format: {}", native_payload_format(payload)),
        "  symbols: unavailable captured package stores a flat native payload, not an ELF"
            .to_owned(),
        "  dwarf: unavailable captured package stores a flat native payload, not an ELF".to_owned(),
        "  follow_up: compare against a rebuilt upstream C ELF with debug symbols".to_owned(),
    ]
    .join("\n"))
}

/// Asserts the linked native ELF preserves the expected semantic behavior.
pub fn assert_native_lib_semantics(elf: &Path) {
    let semantics = analyze_native_lib_elf(elf);

    assert!(
        semantics
            .literal_pools
            .values()
            .any(|word| *word == VESC_IF_TABLE_BASE),
        "expected VESC_IF table base literal in native image: {}",
        semantic_report(&semantics)
    );
    assert!(
        !semantics
            .symbols
            .values()
            .any(|name| name == "vesc_send_app_data" || name == "vesc_set_app_data_handler"),
        "expected direct VESC_IF calls without C wrapper stubs: {}",
        semantic_report(&semantics)
    );
    for symbol in ["init", "package_lib_init", "ext_rust_probe_diag_v4"] {
        assert!(
            semantics.symbols.values().any(|name| name == symbol),
            "expected native image to retain `{symbol}`: {}",
            semantic_report(&semantics)
        );
    }

    assert!(
        init_insns_call(
            &semantics.init_insns,
            &semantics.symbols,
            "package_lib_init"
        ),
        "loader init should run Rust package init before registering the probe: {}",
        semantic_report(&semantics)
    );
    assert!(
        init_insns_touch_vesc_if(&semantics.init_insns, &semantics.literal_pools),
        "Rust loader init should register the probe inline through VESC_IF: {}",
        semantic_report(&semantics)
    );
    assert!(
        init_insns_report_success(&semantics.init_insns),
        "loader init should report success after best-effort package setup: {}",
        semantic_report(&semantics)
    );
    assert!(
        !init_insns_have_failure_return(&semantics.init_insns),
        "loader init should not fail load-native-lib when optional setup reports false: {}",
        semantic_report(&semantics)
    );
    assert!(
        !semantics
            .symbols
            .values()
            .any(|name| name == "register_package_extensions_asm"),
        "Rust init should register directly without a registration trampoline: {}",
        semantic_report(&semantics)
    );

    assert!(
        probe_insns_return_encoded_42(&semantics.probe_insns),
        "Rust probe extension should return the LispBM-encoded integer 42 directly: {}",
        semantic_report(&semantics)
    );
    assert!(
        !probe_insns_touch_vesc_if(&semantics.probe_insns, &semantics.literal_pools),
        "Rust probe extension should not reject valid hardware calls through fragile LispBM validation slots: {}",
        semantic_report(&semantics)
    );

    assert_stop_clears_app_data_handler(&semantics, "stop_package");
}

/// Asserts the loader-facing `.init_fun` returns the package init result.
pub fn assert_loader_init_returns_package_result(elf: &Path) {
    let semantics = analyze_native_lib_elf(elf);

    assert!(
        init_insns_transfer_to_symbol(
            &semantics.init_insns,
            &semantics.symbols,
            "package_lib_init"
        ),
        "loader init should transfer to package setup and return its result: {}",
        semantic_report(&semantics)
    );
}

/// Asserts the native package stop hook clears the firmware app-data callback.
pub fn assert_native_stop_clears_app_data_handler(elf: &Path, name: &str) {
    let semantics = analyze_native_lib_elf(elf);
    assert_stop_clears_app_data_handler(&semantics, name);
}

/// Asserts the Refloat native image preserves the upstream registration tail.
pub fn assert_refloat_registration_tail(elf: &Path) {
    let semantics = analyze_native_lib_elf(elf);

    assert!(
        !semantics.package_init_insns.is_empty(),
        "Refloat package init should be decoded before registration-tail assertions: {}",
        semantic_report(&semantics)
    );
    assert!(
        package_init_touches_slot(&semantics.package_init_insns, 596),
        "Refloat package init must call set_app_data_handler; upstream v1.2.1 (0ef6e99d8701) does at src/main.c:2457: {}",
        semantic_report(&semantics)
    );
    assert!(
        package_init_touches_slot(&semantics.package_init_insns, 184),
        "Refloat package init must allocate firmware-owned app-data state; upstream v1.2.1 (0ef6e99d8701) mallocs Data at src/main.c:2419: {}",
        semantic_report(&semantics)
    );
}

fn symbol_vma(address: u64) -> u64 {
    address & !1
}

fn symbol_address(symbols: &BTreeMap<u64, String>, name: &str) -> Option<u64> {
    symbols
        .iter()
        .find_map(|(addr, symbol)| (symbol == name).then_some(*addr))
}

fn mapping_symbol_line(label: &str, needle: &str, symbols: &BTreeMap<u64, String>) -> String {
    match symbols.iter().find(|(_, symbol)| symbol.contains(needle)) {
        Some((addr, symbol)) => format!("  {label}: present addr={addr:#x} symbol={symbol}"),
        None => format!("  {label}: missing needle={needle}"),
    }
}

fn mapping_symbol_source_line(
    label: &str,
    needle: &str,
    source: &str,
    symbols: &BTreeMap<u64, String>,
) -> String {
    format!(
        "  {label}: {}",
        symbol_status_contains_at(needle, source, symbols)
    )
}

fn mapping_symbol_exact_source_line(
    label: &str,
    needle: &str,
    source: &str,
    symbols: &BTreeMap<u64, String>,
) -> String {
    format!(
        "  {label}: {}",
        symbol_status_exact_at(needle, source, symbols)
    )
}

fn mapping_pair_line(
    label: &str,
    c_symbol: &str,
    c_source: &str,
    rust_needle: &str,
    c_symbols: &BTreeMap<u64, String>,
    rust_symbols: &BTreeMap<u64, String>,
) -> String {
    format!(
        "  {label}: c=[{}] rust=[{}]",
        symbol_status_exact_at(c_symbol, c_source, c_symbols),
        symbol_status_contains(rust_needle, rust_symbols)
    )
}

fn symbol_status_contains(needle: &str, symbols: &BTreeMap<u64, String>) -> String {
    match symbols.iter().find(|(_, symbol)| symbol.contains(needle)) {
        Some((addr, symbol)) => format!("present addr={addr:#x} symbol={symbol}"),
        None => format!("missing needle={needle}"),
    }
}

fn symbol_status_exact_at(needle: &str, source: &str, symbols: &BTreeMap<u64, String>) -> String {
    match symbols.iter().find(|(_, symbol)| symbol.as_str() == needle) {
        Some((addr, symbol)) => format!("present addr={addr:#x} symbol={symbol} source={source}"),
        None => format!("missing symbol={needle} source={source}"),
    }
}

fn symbol_status_contains_at(
    needle: &str,
    source: &str,
    symbols: &BTreeMap<u64, String>,
) -> String {
    match symbols.iter().find(|(_, symbol)| symbol.contains(needle)) {
        Some((addr, symbol)) => format!("present addr={addr:#x} symbol={symbol} source={source}"),
        None => format!("missing needle={needle} source={source}"),
    }
}

fn native_payload_format(payload: &[u8]) -> &'static str {
    if payload.starts_with(b"\x7fELF") {
        "elf"
    } else {
        "flat_native_payload"
    }
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn next_symbol_address(symbols: &BTreeMap<u64, String>, start: u64) -> Option<u64> {
    symbols.keys().copied().filter(|addr| *addr > start).min()
}

fn decode_symbol_insns(
    object: &ObjectFile<'_>,
    cs: &Capstone,
    start: u64,
    end: Option<u64>,
) -> Vec<DecodedInsn> {
    for section in object.sections() {
        let section_start = section.address();
        let section_end = section_start + section.size();
        if !(section_start..section_end).contains(&start) {
            continue;
        }

        let Ok(data) = section.data() else {
            return Vec::new();
        };
        let offset = (start - section_start) as usize;
        let end = end.unwrap_or(section_end).min(section_end);
        let end_offset = (end - section_start) as usize;
        if offset >= data.len() || end_offset > data.len() || offset >= end_offset {
            return Vec::new();
        }

        return cs
            .disasm_all(&data[offset..end_offset], start)
            .map(|insns| decode_insns(&insns))
            .unwrap_or_default();
    }

    Vec::new()
}

fn read_u32_le(_bytes: &[u8], address: u64, object: &ObjectFile<'_>) -> u32 {
    for section in object.sections() {
        let start = section.address();
        let end = start + section.size();
        if (start..end).contains(&address) && address + 4 <= end {
            let offset = (address - start) as usize;
            if let Ok(data) = section.data()
                && offset + 4 <= data.len()
            {
                return u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
            }
        }
    }
    0
}

fn pc_relative_literal_target(insn: &capstone::Insn) -> Option<u64> {
    let op = insn.op_str()?;
    if !op.contains("pc") {
        return None;
    }
    let hash = op.find('#')?;
    let suffix = &op[hash + 1..];
    let digits = suffix
        .trim_start_matches("0x")
        .trim_end_matches(']')
        .trim_end_matches(|ch: char| !ch.is_ascii_hexdigit());
    let offset = u64::from_str_radix(digits, 16).ok()?;
    let pc = (insn.address() + 4) & !3;
    Some(pc + offset)
}

fn decode_insns(insns: &capstone::Instructions) -> Vec<DecodedInsn> {
    insns
        .iter()
        .map(|insn| DecodedInsn {
            address: insn.address(),
            mnemonic: insn.mnemonic().unwrap_or("").to_owned(),
            operands: insn.op_str().unwrap_or("").to_owned(),
        })
        .collect()
}

fn insn_summary(insns: &[DecodedInsn]) -> String {
    insns
        .iter()
        .map(|insn| format!("{} {}", insn.mnemonic, insn.operands))
        .collect::<Vec<_>>()
        .join("; ")
}

fn init_insns_call(insns: &[DecodedInsn], symbols: &BTreeMap<u64, String>, target: &str) -> bool {
    insns
        .iter()
        .any(|insn| insn.mnemonic == "bl" && insn_targets_symbol(insn, symbols, target))
}

fn init_insns_transfer_to_symbol(
    insns: &[DecodedInsn],
    symbols: &BTreeMap<u64, String>,
    target: &str,
) -> bool {
    insns.iter().any(|insn| {
        (insn.mnemonic == "bl" || insn.mnemonic.starts_with("b."))
            && insn_targets_symbol(insn, symbols, target)
    })
}

fn insn_targets_symbol(insn: &DecodedInsn, symbols: &BTreeMap<u64, String>, target: &str) -> bool {
    insn.operands.contains(target)
        || symbol_address(symbols, target)
            .is_some_and(|addr| branch_target(&insn.operands) == Some(addr))
}

fn branch_target(operands: &str) -> Option<u64> {
    let target = operands.split(',').next()?.trim().trim_start_matches('#');
    target.strip_prefix("0x").map_or_else(
        || target.parse().ok(),
        |hex| u64::from_str_radix(hex, 16).ok(),
    )
}

fn init_insns_touch_vesc_if(_insns: &[DecodedInsn], literals: &BTreeMap<u64, u32>) -> bool {
    literals.values().any(|word| *word == VESC_IF_TABLE_BASE)
}

fn init_insns_report_success(insns: &[DecodedInsn]) -> bool {
    insns.iter().any(|insn| {
        (insn.mnemonic == "movs" && insn.operands.contains("#1"))
            || (insn.mnemonic == "mov" && insn.operands.contains("#1"))
    })
}

fn init_insns_have_failure_return(insns: &[DecodedInsn]) -> bool {
    insns.iter().any(|insn| {
        (insn.mnemonic == "movs" && insn.operands.contains("r0, #0"))
            || (insn.mnemonic == "mov" && insn.operands.contains("r0, #0"))
    })
}

fn probe_insns_return_encoded_42(insns: &[DecodedInsn]) -> bool {
    insns.iter().any(|insn| {
        insn.operands
            .contains(&format!("#{PROBE_LISBM_ENCODED_42}"))
            || insn.operands.contains("#680")
            || insn.operands.contains("#0x2a8")
            || insn.operands.contains("0x2a8")
    })
}

fn probe_insns_touch_vesc_if(insns: &[DecodedInsn], literals: &BTreeMap<u64, u32>) -> bool {
    let probe_addrs: BTreeSet<_> = insns.iter().map(|insn| insn.address).collect();
    literals
        .iter()
        .any(|(addr, word)| probe_addrs.contains(addr) && *word == VESC_IF_TABLE_BASE)
}

fn package_init_touches_slot(insns: &[DecodedInsn], slot: u32) -> bool {
    let decimal = format!("#{slot}");
    let hex = format!("#0x{slot:x}");
    insns.iter().any(|insn| {
        insn.mnemonic.starts_with("ldr")
            && (insn.operands.contains(&decimal) || insn.operands.contains(&hex))
    })
}

fn stop_insns_clear_app_data_slot(insns: &[DecodedInsn]) -> bool {
    insns.iter().any(|insn| {
        insn.mnemonic.starts_with("ldr")
            && (insn.operands.contains("#596") || insn.operands.contains("#0x254"))
    })
}

fn stop_insns_branch_to_clear_app_data_helper(
    insns: &[DecodedInsn],
    symbols: &BTreeMap<u64, String>,
) -> bool {
    let Some(target) = symbol_address(symbols, "vesc_clear_loopback_app_data_handler") else {
        return false;
    };
    let hex_target = format!("#0x{target:x}");
    let decimal_target = format!("#{target}");
    insns.iter().any(|insn| {
        insn.mnemonic.starts_with('b')
            && (insn.operands.contains(&hex_target) || insn.operands.contains(&decimal_target))
    })
}

fn assert_stop_clears_app_data_handler(semantics: &NativeLibSemantics, name: &str) {
    assert!(
        !semantics
            .stop_insns
            .iter()
            .any(|insn| insn.mnemonic == "udf"),
        "{name} should not trap when firmware stops the package: {}",
        semantic_report(semantics)
    );
    assert!(
        stop_insns_clear_app_data_slot(&semantics.stop_insns)
            || stop_insns_branch_to_clear_app_data_helper(
                &semantics.stop_insns,
                &semantics.symbols
            ),
        "{name} should clear app-data directly or tail-call the clear helper: {}",
        semantic_report(semantics)
    );
}
