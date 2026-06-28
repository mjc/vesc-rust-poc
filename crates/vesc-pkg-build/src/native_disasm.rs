use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use capstone::prelude::*;
use object::read::File as ObjectFile;
use object::{Object, ObjectSection, ObjectSymbol};

fn symbol_vma(address: u64) -> u64 {
    address & !1
}

fn is_mapping_symbol(name: &str) -> bool {
    name.starts_with('$')
}

fn format_insn_bytes(bytes: &[u8]) -> String {
    if bytes.len() <= 2 {
        format!("{:02x}{:02x}", bytes[1], bytes[0])
    } else {
        format!(
            "{:02x}{:02x} {:02x}{:02x}",
            bytes[1], bytes[0], bytes[3], bytes[2]
        )
    }
}

fn branch_target_address(op_str: &str) -> Option<u64> {
    let hash = op_str.find('#')?;
    let digits = op_str[hash + 1..]
        .trim_start_matches("0x")
        .split(|ch: char| !ch.is_ascii_hexdigit())
        .next()?;
    u64::from_str_radix(digits, 16).ok()
}

fn format_operands(op_str: &str, mnemonic: &str, symbols: &BTreeMap<u64, String>) -> String {
    let mut op = op_str.replace("#0x254", "#596").replace("#0x2a8", "#680");
    if matches!(mnemonic, "bl" | "b" | "bx" | "b.w") {
        if let Some(target) = branch_target_address(&op) {
            if let Some(name) = symbols.get(&target) {
                op.push_str(&format!(" <{name}>"));
            }
        }
    }
    op
}

fn format_address(addr: u64) -> String {
    format!("{addr:>4x}:")
}

fn format_word_line(addr: u64, word: u32) -> String {
    let hex = format!("{word:08x}");
    format!("{}\t{hex}\t.word\t0x{hex}\n", format_address(addr))
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

fn literal_pool_ranges(insns: &capstone::Instructions) -> Vec<(u64, u64)> {
    let mut targets = BTreeSet::new();
    for insn in insns.iter() {
        if let Some(target) = pc_relative_literal_target(insn) {
            targets.insert(target);
        }
    }

    let mut ranges = targets
        .into_iter()
        .map(|target| (target, target + 4))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| range.0);

    let mut merged = Vec::new();
    for (start, end) in ranges {
        if let Some((_, last_end)) = merged.last_mut() {
            if start <= *last_end {
                *last_end = (*last_end).max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn in_literal_pool(addr: u64, ranges: &[(u64, u64)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| (*start..*end).contains(&addr))
}

pub fn elf_disassembly(elf: &Path) -> String {
    let bytes =
        fs::read(elf).unwrap_or_else(|error| panic!("read ELF {elf:?} for disassembly: {error}"));
    let object = ObjectFile::parse(&bytes[..])
        .unwrap_or_else(|error| panic!("parse ELF {elf:?} for disassembly: {error}"));

    let cs = Capstone::new()
        .arm()
        .mode(arch::arm::ArchMode::Thumb)
        .build()
        .expect("capstone ARM thumb");

    let mut symbols: BTreeMap<u64, String> = BTreeMap::new();
    for symbol in object.symbols() {
        if symbol.is_undefined() {
            continue;
        }
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() || is_mapping_symbol(name) {
            continue;
        }
        symbols
            .entry(symbol_vma(symbol.address()))
            .or_insert_with(|| name.to_owned());
    }

    let mut output = String::new();

    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        if !matches!(name, ".text" | ".init_fun") {
            continue;
        }

        let address = section.address();
        let Ok(data) = section.data() else {
            continue;
        };
        if data.is_empty() {
            continue;
        }

        output.push_str(&format!("\nDisassembly of section {name}:\n\n"));

        let section_end = address.saturating_add(data.len() as u64);
        let section_symbols: BTreeMap<u64, String> = symbols
            .range(address..section_end)
            .map(|(addr, sym)| (*addr, sym.clone()))
            .collect();

        let insns = cs
            .disasm_all(data, address)
            .unwrap_or_else(|error| panic!("disassemble {name} in {elf:?}: {error}"));
        let literal_ranges = literal_pool_ranges(&insns);

        let mut insn_starts: BTreeMap<u64, &capstone::Insn> = BTreeMap::new();
        for insn in insns.iter() {
            insn_starts.insert(insn.address(), insn);
        }

        let mut offset = 0usize;
        while offset < data.len() {
            let addr = address + offset as u64;
            if let Some(sym_name) = section_symbols.get(&addr) {
                if offset > 0 {
                    output.push_str("\n\n");
                }
                output.push_str(&format!("{addr:08x} <{sym_name}>:\n"));
            }

            if in_literal_pool(addr, &literal_ranges) && offset + 4 <= data.len() {
                let word = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                output.push_str(&format_word_line(addr, word));
                offset += 4;
                continue;
            }

            if let Some(insn) = insn_starts.get(&addr) {
                let bytes_str = format_insn_bytes(insn.bytes());
                let mnemonic = insn.mnemonic().unwrap_or("");
                let op_str = format_operands(insn.op_str().unwrap_or(""), mnemonic, &symbols);
                output.push_str(&format!(
                    "{}\t{bytes_str}\t{mnemonic}\t{op_str}\n",
                    format_address(addr)
                ));
                offset += insn.bytes().len();
                continue;
            }

            if offset + 4 <= data.len() && addr.is_multiple_of(4) {
                let word = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                output.push_str(&format_word_line(addr, word));
                offset += 4;
                continue;
            }

            offset += 2;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::elf_disassembly;
    use crate::native_lib_link::native_lib_link_plan;
    use crate::package_runner::ensure_repo_native_lib_artifacts;
    use crate::test_support::repo_root;

    #[test]
    fn elf_disassembly_finds_loader_symbols() {
        ensure_repo_native_lib_artifacts(&repo_root());
        let elf = native_lib_link_plan().elf_path();
        let disassembly = elf_disassembly(&elf);
        assert!(disassembly.contains("<init>:"));
        assert!(disassembly.contains("<package_lib_init>"));
    }
}
