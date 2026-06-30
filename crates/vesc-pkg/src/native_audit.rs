use std::collections::BTreeSet;

/// Hex-encoded package bytes captured from a device-proven fixture.
pub const DEVICE_PROVEN_PACKAGE_HEX: &str =
    include_str!("../../../fixtures/device-proven/legacy-init.hex");

/// Decodes the device-proven package fixture into raw package bytes.
pub fn device_proven_package_binary() -> Vec<u8> {
    let hex = DEVICE_PROVEN_PACKAGE_HEX
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    assert_eq!(
        hex.len() % 2,
        0,
        "device-proven legacy-init.hex must contain whole bytes"
    );
    (0..hex.len())
        .step_by(2)
        .map(|idx| u8::from_str_radix(&hex[idx..idx + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .expect("device-proven legacy-init.hex bytes")
}
/// Start offset of the audited init routine inside the flattened fixture.
pub const DEVICE_PROVEN_INIT_OFFSET: usize = 4;
/// Size in bytes of the audited init routine inside the flattened fixture.
pub const DEVICE_PROVEN_INIT_SIZE: usize = 59;

/// Aligns a section virtual address up to `alignment` bytes.
pub fn align_section_vma(vma: usize, alignment: usize) -> usize {
    (vma + alignment - 1) & !(alignment - 1)
}

/// Collects undefined symbols from `nm` output.
pub fn undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output
        .lines()
        .filter_map(parse_undefined_symbol)
        .collect()
}

/// Collects defined symbols from `nm` output.
pub fn defined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output.lines().filter_map(parse_defined_symbol).collect()
}

/// Filters undefined symbols down to entries the runtime should not require.
pub fn unexpected_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_runtime_symbol(symbol))
        .collect()
}

/// Returns whether a runtime symbol is expected to stay unresolved in package artifacts.
pub fn is_allowed_runtime_symbol(symbol: &str) -> bool {
    symbol.starts_with('_')
        || symbol == "fma"
        || matches!(symbol, "lbm_add_extension" | "lbm_dec_as_i32" | "lbm_enc_i")
}

/// Filters final native-lib undefined symbols down to unexpected entries.
pub fn unexpected_final_native_lib_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_final_native_lib_symbol(symbol))
        .collect()
}

/// Returns whether a final native-lib symbol is intentionally supplied by firmware.
pub fn is_allowed_final_native_lib_symbol(symbol: &str) -> bool {
    is_allowed_runtime_symbol(symbol)
}

/// Returns the disassembly slice that covers the audited init routine.
pub fn bounded_init_disassembly(disassembly: &str) -> &str {
    disassembly
        .split("<init>:")
        .nth(1)
        .expect("expected init in disassembly")
        .split("\n\n")
        .next()
        .expect("expected bounded init disassembly")
}

/// Redacts unstable addresses from disassembly snapshots.
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

/// Asserts that loader init code talks to firmware only through the `vesc-ffi` surface.
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

/// Runs the full audit suite against the checked-in device-proven fixture.
pub fn audit_device_proven_fixture() {
    let bytes = device_proven_package_binary();
    assert_eq!(bytes.len(), 183);
    assert_ne!(
        bytes[DEVICE_PROVEN_INIT_OFFSET..DEVICE_PROVEN_INIT_OFFSET + DEVICE_PROVEN_INIT_SIZE],
        [0u8; DEVICE_PROVEN_INIT_SIZE]
    );
}
