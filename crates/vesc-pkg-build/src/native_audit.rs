use std::collections::BTreeSet;

pub const DEVICE_PROVEN_PACKAGE_BINARY: &str = "fixtures/device-proven/legacy-init.bin";
pub const DEVICE_PROVEN_INIT_OFFSET: usize = 4;
pub const DEVICE_PROVEN_INIT_SIZE: usize = 59;

pub fn align_section_vma(vma: usize, alignment: usize) -> usize {
    (vma + alignment - 1) & !(alignment - 1)
}

pub fn undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output
        .lines()
        .filter_map(parse_undefined_symbol)
        .collect()
}

pub fn defined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output.lines().filter_map(parse_defined_symbol).collect()
}

pub fn unexpected_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_runtime_symbol(symbol))
        .collect()
}

pub fn is_allowed_runtime_symbol(symbol: &str) -> bool {
    symbol.starts_with('_')
        || symbol == "fma"
        || matches!(symbol, "lbm_add_extension" | "lbm_dec_as_i32" | "lbm_enc_i")
}

pub fn unexpected_final_native_lib_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_final_native_lib_symbol(symbol))
        .collect()
}

pub fn is_allowed_final_native_lib_symbol(symbol: &str) -> bool {
    is_allowed_runtime_symbol(symbol)
}

pub fn bounded_init_disassembly(disassembly: &str) -> &str {
    disassembly
        .split("<init>:")
        .nth(1)
        .expect("expected init in disassembly")
        .split("\n\n")
        .next()
        .expect("expected bounded init disassembly")
}

pub fn redact_disassembly_for_snapshot(text: &str) -> String {
    text.lines()
        .map(|line| {
            let Some(colon) = line.find(':') else {
                return line.to_owned();
            };
            let prefix = &line[..colon];
            if prefix
                .trim()
                .chars()
                .all(|character| character.is_ascii_hexdigit())
                && !prefix.trim().is_empty()
            {
                let indent = line.len() - line.trim_start().len();
                return format!("{}[ADDR]:{}", " ".repeat(indent), &line[colon + 1..]);
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn assert_rust_loader_init_uses_vesc_ffi(init_disassembly: &str) {
    assert!(
        init_disassembly.contains("1000f800"),
        "loader init should load the firmware VESC table base:\n{init_disassembly}"
    );
    assert!(
        init_disassembly.contains("4798")
            || init_disassembly.contains("4790")
            || init_disassembly.contains("4710"),
        "loader init should call lbm_add_extension through an indirect branch after loading slot 0:\n{init_disassembly}"
    );
    assert!(
        init_disassembly.contains("<package_lib_init>"),
        "loader init should delegate package setup to Rust before registering the probe:\n{init_disassembly}"
    );
}

fn parse_undefined_symbol(line: &str) -> Option<String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        ["U", symbol] => Some((*symbol).to_owned()),
        [_, "U", symbol] => Some((*symbol).to_owned()),
        _ => None,
    }
}

fn parse_defined_symbol(line: &str) -> Option<String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [_, kind, symbol] if *kind != "U" => Some((*symbol).to_owned()),
        _ => None,
    }
}
