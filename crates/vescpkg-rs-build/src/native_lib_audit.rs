use std::fs;
use std::path::{Path, PathBuf};

use crate::native_audit::{
    DEVICE_PROVEN_INIT_OFFSET, DEVICE_PROVEN_INIT_SIZE, align_section_vma, defined_symbols,
    device_proven_package_binary, undefined_symbols, unexpected_final_native_lib_undefined_symbols,
    unexpected_undefined_symbols,
};
use crate::native_elf_semantics::{
    analyze_native_lib_elf, assert_loader_init_returns_package_result, assert_native_lib_semantics,
    assert_refloat_registration_tail,
};
use crate::native_inspect::{
    SectionLayout, all_section_layouts, elf_has_no_relocations, elf_is_executable,
    elf_to_flat_binary, nm_output, section_from,
};
use crate::native_lib_link::NativeLibLinkPlan;

/// Concrete artifact paths consumed by the native-lib audit helpers.
pub struct NativeLibArtifactPaths {
    /// Linked native ELF path.
    pub elf: PathBuf,
    /// Flattened native binary path.
    pub bin: PathBuf,
    /// Rust static library input path.
    pub staticlib: PathBuf,
    /// Legacy package C shim object path that must not be linked into the final ELF.
    pub package_object: PathBuf,
}

impl NativeLibArtifactPaths {
    /// Builds artifact paths from a link plan.
    pub fn from_link_plan(plan: &NativeLibLinkPlan) -> Self {
        Self {
            elf: plan.elf_path(),
            bin: plan.native_lib_bin_path(),
            staticlib: plan.rust_staticlib_path(),
            package_object: plan.package_c_object_path(),
        }
    }
}

/// Audits symbol-level constraints for the native-lib build outputs.
pub fn audit_native_lib_symbols(paths: &NativeLibArtifactPaths) {
    let elf_symbols = nm_output(&paths.elf);
    let staticlib_symbols = nm_output(&paths.staticlib);
    let staticlib_defined = defined_symbols(&staticlib_symbols);
    let staticlib_undefined = undefined_symbols(&staticlib_symbols);
    let elf_defined = defined_symbols(&elf_symbols);
    let elf_undefined = undefined_symbols(&elf_symbols);

    assert!(
        unexpected_undefined_symbols(&staticlib_symbols).is_empty(),
        "unexpected undefined symbols remain in the Rust staticlib"
    );
    assert!(
        unexpected_final_native_lib_undefined_symbols(&elf_symbols).is_empty(),
        "unexpected undefined symbols remain in the final native-lib ELF"
    );
    assert_no_forbidden_runtime_symbols(&elf_symbols, "final native-lib ELF");
    assert!(
        !paths.package_object.exists(),
        "native build must not materialize package-specific C shim object {:?}",
        paths.package_object
    );
    assert!(
        elf_defined.contains("ext_rust_probe_diag_v4"),
        "final image must retain the Rust LispBM probe callback:\n{elf_symbols}"
    );
    assert!(
        elf_defined.contains("init")
            && elf_defined.contains("prog_ptr")
            && elf_defined.contains("package_lib_init"),
        "native image must keep loader entry and Rust package init:\n{elf_symbols}"
    );
    assert!(
        !elf_defined.contains("ext_c_probe_v12"),
        "expected final native image to drop the C LispBM probe body:\n{elf_symbols}"
    );
    assert!(
        !elf_defined.contains("ext_c_probe_v6"),
        "expected final native image to drop the temporary C probe after Rust-owned registration:\n{elf_symbols}"
    );
    assert!(
        elf_undefined.is_empty(),
        "expected final native image to resolve the C-to-Rust boundary completely:\n{elf_symbols}"
    );
    assert!(
        staticlib_defined.contains("package_lib_init"),
        "expected the Rust staticlib to export package_lib_init"
    );
    assert!(
        staticlib_defined.contains("init") && staticlib_defined.contains("prog_ptr"),
        "expected the Rust staticlib to export loader entry symbols"
    );
    for symbol in [
        "package_lib_init",
        "ext_rust_probe_diag_v4",
        "init",
        "prog_ptr",
        "loopback_handle_app_data",
        "vesc_register_loopback_app_data_handler",
        "vesc_clear_loopback_app_data_handler",
    ] {
        assert!(
            staticlib_defined.contains(symbol),
            "Rust staticlib must own symbol `{symbol}`:\n{staticlib_symbols}"
        );
    }
    assert!(
        !staticlib_defined.contains("rust_add"),
        "rust_add must stay test-only and out of the firmware staticlib:\n{staticlib_symbols}"
    );
    assert!(
        !staticlib_undefined.contains("register_c_probe"),
        "Rust package init should not depend on a separate C probe registrar:\n{staticlib_symbols}"
    );
}

/// Audits section layout and flash-budget constraints for the native-lib outputs.
pub fn audit_native_lib_layout(paths: &NativeLibArtifactPaths) {
    let blob = fs::read(&paths.bin).expect("native-lib binary bytes");
    let sections = all_section_layouts(&paths.elf);

    assert!(
        paths.bin.exists(),
        "expected the final native-lib binary to be materialized"
    );

    let native_bin_size = fs::metadata(&paths.bin)
        .expect("native-lib binary metadata")
        .len();
    assert!(
        native_bin_size <= 640,
        "expected the native blob to stay compact, got {native_bin_size} bytes"
    );
    assert!(
        native_bin_size <= 512,
        "expected the Rust-only native blob to stay compact, got {native_bin_size} bytes"
    );

    let rust_extension_name = b"ext-rust-probe-diag-v4\0";
    assert!(
        blob.windows(rust_extension_name.len())
            .any(|window| window == rust_extension_name),
        "Rust probe extension identity must be linked into the native blob"
    );

    assert!(
        elf_is_executable(&paths.elf),
        "expected a final executable ELF at {:?}",
        paths.elf
    );
    assert!(
        elf_has_no_relocations(&paths.elf),
        "expected no relocation records in the final native-lib ELF at {:?}",
        paths.elf
    );

    for section_name in [".program_ptr", ".init_fun", ".data", ".got", ".text"] {
        let section = section_from(&sections, section_name);
        let end = section.vma + section.size;
        assert!(
            end <= blob.len(),
            "section {section_name} at 0x{:x}..0x{:x} exceeds {}-byte blob",
            section.vma,
            end,
            blob.len()
        );
    }

    let init_fun = section_from(&sections, ".init_fun");
    assert_eq!(
        init_fun.vma, DEVICE_PROVEN_INIT_OFFSET,
        "expected .init_fun to start at the device-proven offset"
    );
    assert!(
        init_fun.size >= 24,
        "expected Rust-owned init to retain loader entry and probe registration"
    );
    let proven = device_proven_package_binary();
    let proven_init_end = DEVICE_PROVEN_INIT_OFFSET + DEVICE_PROVEN_INIT_SIZE;
    assert_ne!(
        &blob[init_fun.vma..init_fun.vma + init_fun.size.min(DEVICE_PROVEN_INIT_SIZE)],
        &proven[DEVICE_PROVEN_INIT_OFFSET..proven_init_end],
        "Rust-owned init should no longer match the legacy hand-asm bytes in fixtures/device-proven/legacy-init.hex"
    );

    let program_ptr = section_from(&sections, ".program_ptr");
    let got = section_from(&sections, ".got");
    let text = section_from(&sections, ".text");
    assert_eq!(
        *program_ptr,
        SectionLayout {
            name: ".program_ptr".to_owned(),
            size: 4,
            vma: 0,
        }
    );
    assert_eq!(init_fun.vma, program_ptr.vma + program_ptr.size);
    assert_eq!(
        got.vma,
        align_section_vma(init_fun.vma + init_fun.size, 4),
        "expected .got to follow .init_fun with VESC's 4-byte section alignment"
    );
    assert!(
        text.vma >= got.vma + got.size,
        "expected .text to load after .got"
    );
    assert_eq!(
        text.vma % 16,
        0,
        "expected .text to keep VESC's 16-byte function alignment"
    );
    assert!(
        text.size >= 64,
        "expected .text to retain the probe callback and stop hook"
    );
}

/// Audits that the flattened binary matches the linked ELF layout.
pub fn audit_native_lib_flat_binary(elf: &Path, bin: &Path) {
    let flat = elf_to_flat_binary(elf);
    let materialized = fs::read(bin).expect("materialized native bin");
    assert_eq!(
        flat, materialized,
        "in-process ELF flattening must match materialized bin"
    );
}

/// Builds a redacted semantic snapshot report for `elf`.
pub fn semantic_snapshot_report(elf: &Path) -> String {
    let semantics = analyze_native_lib_elf(elf);
    let stable_symbols = semantics
        .symbols
        .values()
        .filter(|name| {
            matches!(
                name.as_str(),
                "init" | "prog_ptr" | "package_lib_init" | "ext_rust_probe_diag_v4"
            ) || name.contains("stop_package")
                || name.contains("stop_refloat_app_data")
        })
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "symbols: {stable_symbols}\nliteral_pools: {}\ninit: {}\nprobe: {}\nstop: {}",
        semantics
            .literal_pools
            .iter()
            .map(|(addr, word)| format!("{addr:#x}=0x{word:08x}"))
            .collect::<Vec<_>>()
            .join(", "),
        semantics
            .init_insns
            .iter()
            .map(|insn| format!("{} {}", insn.mnemonic, insn.operands))
            .collect::<Vec<_>>()
            .join("; "),
        semantics
            .probe_insns
            .iter()
            .map(|insn| format!("{} {}", insn.mnemonic, insn.operands))
            .collect::<Vec<_>>()
            .join("; "),
        semantics
            .stop_insns
            .iter()
            .map(|insn| format!("{} {}", insn.mnemonic, insn.operands))
            .collect::<Vec<_>>()
            .join("; ")
    )
}

/// Runs the full native-lib audit suite and returns the semantic snapshot text.
pub fn audit_native_lib_artifacts(paths: &NativeLibArtifactPaths) -> String {
    audit_native_lib_symbols(paths);
    audit_native_lib_layout(paths);
    audit_native_lib_flat_binary(&paths.elf, &paths.bin);
    assert_native_lib_semantics(&paths.elf);
    semantic_snapshot_report(&paths.elf)
}
/// Runs the Refloat native-lib audit suite and returns the semantic snapshot text.
pub fn audit_refloat_native_lib_artifacts(paths: &NativeLibArtifactPaths) -> String {
    audit_refloat_native_lib_symbols(paths);
    audit_refloat_native_lib_layout(paths);
    audit_native_lib_flat_binary(&paths.elf, &paths.bin);
    assert_loader_init_returns_package_result(&paths.elf);
    assert_refloat_registration_tail(&paths.elf);
    semantic_snapshot_report(&paths.elf)
}

fn audit_refloat_native_lib_symbols(paths: &NativeLibArtifactPaths) {
    let elf_symbols = nm_output(&paths.elf);
    let staticlib_symbols = nm_output(&paths.staticlib);
    let staticlib_defined = defined_symbols(&staticlib_symbols);
    let elf_defined = defined_symbols(&elf_symbols);
    let elf_undefined = undefined_symbols(&elf_symbols);

    assert!(
        unexpected_undefined_symbols(&staticlib_symbols).is_empty(),
        "unexpected undefined symbols remain in the Refloat Rust staticlib"
    );
    assert!(
        unexpected_final_native_lib_undefined_symbols(&elf_symbols).is_empty(),
        "unexpected undefined symbols remain in the Refloat native-lib ELF"
    );
    assert!(
        !paths.package_object.exists(),
        "Refloat native build must not materialize package-specific C shim object {:?}",
        paths.package_object
    );

    for symbol in ["init", "prog_ptr", "package_lib_init"] {
        assert!(
            elf_defined.contains(symbol),
            "Refloat native image must keep loader symbol `{symbol}`:\n{elf_symbols}"
        );
        assert!(
            staticlib_defined.contains(symbol),
            "Refloat Rust staticlib must own symbol `{symbol}`:\n{staticlib_symbols}"
        );
    }

    assert!(
        elf_defined.contains("refloat_handle_app_data")
            && elf_defined
                .iter()
                .any(|name| name.contains("stop_refloat_app_data")),
        "Refloat native image must retain app-data and stop entrypoints; upstream v1.2.1 (0ef6e99d8701) wires app-data at src/main.c:2143 and 2457 and stop cleanup at src/main.c:2398-2412:\n{elf_symbols}"
    );
    assert!(
        !elf_defined.contains("ext_rust_probe_diag_v4"),
        "Refloat native image should not carry loopback probe extension symbols:\n{elf_symbols}"
    );
    assert!(
        elf_undefined.is_empty(),
        "expected Refloat native image to resolve the Rust package boundary completely:\n{elf_symbols}"
    );
}

fn assert_no_forbidden_runtime_symbols(elf_symbols: &str, label: &str) {
    let forbidden_runtime_fragments = [
        "__rust_alloc",
        "__rg_alloc",
        "malloc",
        "free",
        "alloc::",
        "std::",
        "std_",
        "panic",
        "eh_personality",
        "unwind",
        "__aeabi_memcpy",
        "memcpy",
    ];
    let forbidden_hits: Vec<&str> = elf_symbols
        .lines()
        .filter(|line| {
            forbidden_runtime_fragments
                .iter()
                .any(|fragment| line.contains(fragment))
        })
        .collect();
    assert!(
        forbidden_hits.is_empty(),
        "{label} must not contain allocator/std/panic/memcpy runtime symbols:\n{}\n\nfull symbols:\n{elf_symbols}",
        forbidden_hits.join("\n")
    );
}

fn audit_refloat_native_lib_layout(paths: &NativeLibArtifactPaths) {
    let blob = fs::read(&paths.bin).expect("Refloat native-lib binary bytes");
    let sections = all_section_layouts(&paths.elf);

    assert!(
        paths.bin.exists(),
        "expected the Refloat native-lib binary to be materialized"
    );

    let native_bin_size = fs::metadata(&paths.bin)
        .expect("Refloat native-lib binary metadata")
        .len();
    assert!(
        native_bin_size <= 46 * 1024,
        "expected the Refloat native blob with generated config XML to stay below 46 KiB, got {native_bin_size} bytes"
    );

    assert!(
        elf_is_executable(&paths.elf),
        "expected a final executable Refloat ELF at {:?}",
        paths.elf
    );
    assert!(
        elf_has_no_relocations(&paths.elf),
        "expected no relocation records in the final Refloat native-lib ELF at {:?}",
        paths.elf
    );

    for section_name in [".program_ptr", ".init_fun", ".data", ".got", ".text"] {
        let section = section_from(&sections, section_name);
        let end = section.vma + section.size;
        assert!(
            end <= blob.len(),
            "Refloat section {section_name} at 0x{:x}..0x{:x} exceeds {}-byte blob",
            section.vma,
            end,
            blob.len()
        );
    }

    let program_ptr = section_from(&sections, ".program_ptr");
    let init_fun = section_from(&sections, ".init_fun");
    let data = section_from(&sections, ".data");
    let got = section_from(&sections, ".got");
    let text = section_from(&sections, ".text");

    assert_eq!(
        *program_ptr,
        SectionLayout {
            name: ".program_ptr".to_owned(),
            size: 4,
            vma: 0,
        }
    );
    assert_eq!(init_fun.vma, program_ptr.vma + program_ptr.size);
    assert!(
        init_fun.size >= 4,
        "expected Refloat .init_fun to retain the loader entry"
    );
    assert!(
        data.vma >= init_fun.vma + init_fun.size,
        "expected Refloat .data to load after .init_fun"
    );
    assert!(
        got.vma >= data.vma + data.size,
        "expected Refloat .got to load after package-owned .data"
    );
    assert!(
        text.vma >= got.vma + got.size,
        "expected Refloat .text to load after .got"
    );
    assert_eq!(
        text.vma % 16,
        0,
        "expected Refloat .text to keep VESC's 16-byte function alignment"
    );
}
