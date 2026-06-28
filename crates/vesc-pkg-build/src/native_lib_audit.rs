use std::fs;
use std::path::{Path, PathBuf};

use crate::native_audit::{
    align_section_vma, defined_symbols, device_proven_package_binary, undefined_symbols,
    unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    DEVICE_PROVEN_INIT_OFFSET, DEVICE_PROVEN_INIT_SIZE,
};
use crate::native_elf_semantics::{analyze_native_lib_elf, assert_native_lib_semantics};
use crate::native_inspect::{
    all_section_layouts, elf_has_no_relocations, elf_is_executable, elf_to_flat_binary, nm_output,
    section_from, SectionLayout,
};
use crate::native_lib_link::NativeLibLinkPlan;

#[derive(Debug, Clone)]
pub struct NativeLibArtifactPaths {
    pub elf: PathBuf,
    pub bin: PathBuf,
    pub staticlib: PathBuf,
    pub package_object: PathBuf,
}

impl NativeLibArtifactPaths {
    pub fn from_link_plan(plan: &NativeLibLinkPlan) -> Self {
        Self {
            elf: plan.elf_path(),
            bin: plan.native_lib_bin_path(),
            staticlib: plan.rust_staticlib_path(),
            package_object: plan.package_c_object_path(),
        }
    }
}

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
    assert!(
        paths.package_object.exists(),
        "native build must materialize the C loader shim at {:?}",
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

    for section_name in [".program_ptr", ".init_fun", ".got", ".text"] {
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

pub fn audit_native_lib_flat_binary(elf: &Path, bin: &Path) {
    let flat = elf_to_flat_binary(elf);
    let materialized = fs::read(bin).expect("materialized native bin");
    assert_eq!(
        flat, materialized,
        "in-process ELF flattening must match materialized bin"
    );
}

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

pub fn audit_native_lib_artifacts(paths: &NativeLibArtifactPaths) -> String {
    audit_native_lib_symbols(paths);
    audit_native_lib_layout(paths);
    audit_native_lib_flat_binary(&paths.elf, &paths.bin);
    assert_native_lib_semantics(&paths.elf);
    semantic_snapshot_report(&paths.elf)
}
