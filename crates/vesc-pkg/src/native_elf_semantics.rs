use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use capstone::prelude::*;
use object::read::File as ObjectFile;
use object::{Object, ObjectSection, ObjectSymbol};

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
    let stop_start = symbols
        .iter()
        .find_map(|(addr, name)| name.contains("stop_package").then_some(*addr));

    if let Some(start) = probe_start {
        let end = package_init_start
            .or(stop_start)
            .or_else(|| next_symbol_address(&symbols, start));
        probe_insns = decode_symbol_insns(&object, &cs, start, end);
    }
    if let Some(start) = stop_start {
        stop_insns = decode_symbol_insns(&object, &cs, start, next_symbol_address(&symbols, start));
    }

    NativeLibSemantics {
        symbols,
        literal_pools,
        init_insns,
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
    lines.push(format!("probe: {}", insn_summary(&semantics.probe_insns)));
    lines.push(format!("stop: {}", insn_summary(&semantics.stop_insns)));
    lines.join("\n")
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
        init_insns_call(&semantics.init_insns, "package_lib_init"),
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

    assert!(
        stop_insns_clear_app_data_slot(&semantics.stop_insns, &semantics.literal_pools)
            || stop_insns_branch_to_clear_app_data_helper(
                &semantics.stop_insns,
                &semantics.symbols
            ),
        "stop_package should clear app-data directly or tail-call the clear helper: {}",
        semantic_report(&semantics)
    );
    assert!(
        !semantics
            .stop_insns
            .iter()
            .any(|insn| insn.mnemonic.starts_with("cbz") || insn.mnemonic.starts_with("cbnz")),
        "stop_package should not guard the VESC_IF app-data slot; refloat calls it directly: {}",
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

fn init_insns_call(insns: &[DecodedInsn], target: &str) -> bool {
    insns.iter().any(|insn| {
        insn.mnemonic == "bl" && (insn.operands.contains(target) || insn.operands.contains("0x60"))
    })
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

fn stop_insns_clear_app_data_slot(insns: &[DecodedInsn], literals: &BTreeMap<u64, u32>) -> bool {
    literals.values().any(|word| *word == VESC_IF_TABLE_BASE)
        && insns.iter().any(|insn| {
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
