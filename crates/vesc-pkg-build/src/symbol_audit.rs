use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::native_lib_link::{native_lib_link_plan, NativeLibLinkPlan};

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

pub fn rust_staticlib_path() -> PathBuf {
    native_lib_link_plan().rust_staticlib_path()
}

pub fn native_lib_elf_path() -> PathBuf {
    native_lib_link_plan().elf_path()
}

pub fn native_lib_bin_path() -> PathBuf {
    native_lib_link_plan().native_lib_bin_path()
}

pub fn package_lib_object_path() -> PathBuf {
    native_lib_link_plan().package_c_object_path()
}

pub fn build_rust_staticlib() {
    build_rust_staticlib_for(&native_lib_link_plan());
}

pub fn build_rust_staticlib_for(plan: &NativeLibLinkPlan) {
    build_rust_staticlib_unlocked(plan);
}

fn build_rust_staticlib_unlocked(plan: &NativeLibLinkPlan) {
    let rustflags = match std::env::var("RUSTFLAGS") {
        Ok(existing) if !existing.trim().is_empty() => {
            format!("{existing} -C relocation-model=pic")
        }
        _ => "-C relocation-model=pic".to_owned(),
    };

    let status = Command::new("cargo")
        .env("CARGO_TARGET_DIR", plan.cargo_target_dir())
        .env("RUSTFLAGS", rustflags)
        .args([
            "build",
            "--release",
            "--target",
            "thumbv7em-none-eabihf",
            "-p",
            "vesc-rust-poc",
        ])
        .status()
        .expect("cargo build for the Rust staticlib");

    assert!(status.success(), "cargo failed to build the Rust staticlib");
}

pub fn build_final_native_lib_elf() {
    build_final_native_lib_elf_for(&native_lib_link_plan());
}

pub fn build_final_native_lib_elf_for(plan: &NativeLibLinkPlan) {
    build_final_native_lib_elf_unlocked(plan);
}

fn native_lib_c_only_from_env() -> bool {
    std::env::var("VESC_NATIVE_LIB_C_ONLY").ok().as_deref() == Some("1")
}

fn build_final_native_lib_elf_unlocked(plan: &NativeLibLinkPlan) {
    let c_only = native_lib_c_only_from_env();
    if !c_only {
        build_rust_staticlib_unlocked(plan);
    }

    let link_plan = plan.clone();
    let elf_path = link_plan.elf_path();

    if let Some(parent) = elf_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.elf parent directory");
    }
    let stale_object_path = link_plan.package_c_object_path();
    if stale_object_path.exists() {
        fs::remove_file(&stale_object_path).expect("remove stale package_lib.o");
    }

    let compile_status = Command::new("arm-none-eabi-gcc")
        .args([
            "-fpic",
            "-Os",
            "-Wall",
            "-Wextra",
            "-Wundef",
            "-std=gnu99",
            "-I",
            link_plan
                .package_c_source_path()
                .parent()
                .expect("package C source parent")
                .to_str()
                .expect("utf-8 package C source parent"),
            "-fomit-frame-pointer",
            "-falign-functions=16",
            "-mthumb",
            "-fsingle-precision-constant",
            "-Wdouble-promotion",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-mcpu=cortex-m4",
            "-fdata-sections",
            "-ffunction-sections",
            "-DIS_VESC_LIB",
            "-c",
            link_plan
                .package_c_source_path()
                .to_str()
                .expect("utf-8 package C source path"),
            "-o",
            link_plan
                .package_c_object_path()
                .to_str()
                .expect("utf-8 package C object path"),
        ])
        .status()
        .expect("arm-none-eabi-gcc compile of package C shim");

    assert!(compile_status.success(), "failed to compile package C shim");

    let mut link_args = vec![
        "-nostartfiles".to_owned(),
        "-static".to_owned(),
        "-mcpu=cortex-m4".to_owned(),
        "-mthumb".to_owned(),
        "-mfloat-abi=hard".to_owned(),
        "-mfpu=fpv4-sp-d16".to_owned(),
        link_plan
            .package_c_object_path()
            .to_str()
            .expect("utf-8 package C object path")
            .to_owned(),
    ];
    if !c_only {
        link_args.push(
            link_plan
                .rust_staticlib_path()
                .to_str()
                .expect("utf-8 staticlib path")
                .to_owned(),
        );
    }
    link_args.extend([
        "-Wl,--gc-sections".to_owned(),
        "-Wl,--undefined=init".to_owned(),
        "-T".to_owned(),
        link_plan
            .linker_script_path()
            .to_str()
            .expect("utf-8 linker script path")
            .to_owned(),
        "-o".to_owned(),
        elf_path.to_str().expect("utf-8 ELF path").to_owned(),
    ]);

    let link_status = Command::new("arm-none-eabi-gcc")
        .args(link_args)
        .status()
        .expect("arm-none-eabi-gcc link of the final native-lib ELF");

    assert!(
        link_status.success(),
        "failed to link the final native-lib ELF"
    );
}

pub fn build_final_native_lib_binary(native_binary_path: &Path) {
    let plan = crate::native_lib_link::native_lib_link_plan_for_native_binary(native_binary_path);
    build_final_native_lib_binary_for(&plan, native_binary_path);
}

pub fn build_final_native_lib_binary_for(plan: &NativeLibLinkPlan, native_binary_path: &Path) {
    build_final_native_lib_elf_unlocked(plan);

    if let Some(parent) = native_binary_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.bin parent directory");
    }

    let objcopy_status = Command::new("arm-none-eabi-objcopy")
        .args([
            "-O",
            "binary",
            plan.elf_path().to_str().expect("utf-8 native-lib ELF path"),
            native_binary_path
                .to_str()
                .expect("utf-8 native-lib binary path"),
            "--gap-fill",
            "0x00",
        ])
        .status()
        .expect("arm-none-eabi-objcopy of the final native-lib ELF");

    assert!(
        objcopy_status.success(),
        "failed to objcopy the final native-lib ELF into the native binary"
    );
}

pub fn nm_output(path: &Path) -> String {
    let output = Command::new("arm-none-eabi-nm")
        .arg(path)
        .output()
        .expect("arm-none-eabi-nm execution");

    assert!(
        output.status.success(),
        "arm-none-eabi-nm failed for {path:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("nm output to be valid UTF-8")
}

pub fn audit_rust_staticlib_symbols() -> BTreeSet<String> {
    build_rust_staticlib();
    let output = nm_output(&rust_staticlib_path());

    unexpected_undefined_symbols(&output)
}

pub fn audit_final_native_lib_elf_symbols() -> BTreeSet<String> {
    build_final_native_lib_elf();
    let output = nm_output(&native_lib_elf_path());

    unexpected_final_native_lib_undefined_symbols(&output)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{
        build_final_native_lib_binary_for, build_final_native_lib_elf_for,
        build_rust_staticlib_for, defined_symbols, is_allowed_final_native_lib_symbol,
        is_allowed_runtime_symbol, nm_output, undefined_symbols,
        unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    };
    use crate::native_lib_link::NativeLibLinkPlan;
    use crate::test_support::NativeBuildWorkspace;
    use std::collections::BTreeSet;

    struct SymbolAuditFixture {
        workspace: NativeBuildWorkspace,
    }

    impl SymbolAuditFixture {
        fn new() -> Self {
            Self {
                workspace: NativeBuildWorkspace::new(),
            }
        }

        fn plan(&self) -> &NativeLibLinkPlan {
            self.workspace.plan()
        }

        fn elf(&self) -> PathBuf {
            self.workspace.native_lib_elf_path()
        }

        fn bin(&self) -> PathBuf {
            self.workspace.native_lib_bin_path()
        }

        fn staticlib(&self) -> PathBuf {
            self.workspace.rust_staticlib_path()
        }

        fn package_object(&self) -> PathBuf {
            self.workspace.package_lib_object_path()
        }

        fn build_elf(&self) {
            build_final_native_lib_elf_for(self.plan());
        }

        fn build_bin(&self) {
            build_final_native_lib_binary_for(self.plan(), &self.bin());
        }

        fn build_staticlib(&self) {
            build_rust_staticlib_for(self.plan());
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct SectionLayout {
        name: String,
        size: usize,
        vma: usize,
    }

    const DEVICE_PROVEN_PACKAGE_BINARY: &str = "THIS_FUCKING_RAN_AT_LEAST.bin.good";
    const DEVICE_PROVEN_INIT_OFFSET: usize = 4;
    const DEVICE_PROVEN_INIT_SIZE: usize = 59;

    fn align_section_vma(vma: usize, alignment: usize) -> usize {
        (vma + alignment - 1) & !(alignment - 1)
    }

    #[test]
    fn separates_defined_and_undefined_symbols() {
        let sample = "\
00000000 T rust_add
         U __aeabi_dadd
         U fma
         U lbm_add_extension
";

        assert_eq!(
            defined_symbols(sample),
            BTreeSet::from(["rust_add".to_owned()])
        );
        assert_eq!(
            undefined_symbols(sample),
            BTreeSet::from([
                "__aeabi_dadd".to_owned(),
                "fma".to_owned(),
                "lbm_add_extension".to_owned(),
            ])
        );
    }

    #[test]
    fn allows_runtime_underscore_symbols_but_flags_plain_external_names() {
        assert!(is_allowed_runtime_symbol("__aeabi_dadd"));
        assert!(is_allowed_runtime_symbol(
            "_RNvNtNtCseGTyb2smT0B_17compiler_builtins3mem6memcpy"
        ));
        assert!(is_allowed_runtime_symbol("fma"));
        assert!(is_allowed_runtime_symbol("lbm_add_extension"));
        assert!(is_allowed_runtime_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_runtime_symbol("lbm_enc_i"));

        let sample = "\
         U __aeabi_dadd
         U fma
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
         U plain_external
";

        assert_eq!(
            unexpected_undefined_symbols(sample),
            BTreeSet::from(["plain_external".to_owned()])
        );
    }

    #[test]
    fn rust_staticlib_has_no_unexpected_undefined_symbols() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_staticlib();
        let output = nm_output(&fixture.staticlib());
        assert!(
            unexpected_undefined_symbols(&output).is_empty(),
            "unexpected undefined symbols remain in the Rust staticlib"
        );
    }

    #[test]
    fn final_native_lib_elf_has_no_unexpected_undefined_symbols() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();
        let output = nm_output(&fixture.elf());
        assert!(
            unexpected_final_native_lib_undefined_symbols(&output).is_empty(),
            "unexpected undefined symbols remain in the final native-lib ELF"
        );
    }

    #[test]
    fn build_final_native_lib_binary_materializes_the_packageable_payload() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_bin();

        assert!(
            fixture.bin().exists(),
            "expected the final native-lib binary to be materialized"
        );
        let native_bin_size = fs::metadata(fixture.bin())
            .expect("native-lib binary metadata")
            .len();
        assert!(
            native_bin_size <= 640,
            "expected the native blob to stay compact, got {native_bin_size} bytes"
        );
    }

    #[test]
    fn rust_only_native_blob_stays_under_compactness_guard() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_bin();

        let native_bin_size = fs::metadata(fixture.bin())
            .expect("native-lib binary metadata")
            .len();
        assert!(
            native_bin_size <= 512,
            "expected the Rust-only native blob to stay compact, got {native_bin_size} bytes"
        );
    }

    #[test]
    fn native_blob_embeds_rust_owned_package_identity() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_bin();

        let blob = fs::read(fixture.bin()).expect("native-lib binary bytes");
        let rust_extension_name = b"ext-rust-probe-diag-v4\0";

        assert!(
            blob.windows(rust_extension_name.len())
                .any(|window| window == rust_extension_name),
            "Rust probe extension identity must be linked into the native blob:\n{rust_extension_name:?}"
        );
    }

    #[test]
    fn final_native_lib_elf_is_a_fully_linked_executable_image() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let header = command_stdout(
            "arm-none-eabi-readelf",
            [PathBuf::from("-h"), fixture.elf()],
        );
        assert!(
            header.contains("Type:                              EXEC"),
            "expected a final executable ELF, got:\n{header}"
        );

        let relocations = command_stdout(
            "arm-none-eabi-readelf",
            [PathBuf::from("-r"), fixture.elf()],
        );
        assert!(
            relocations.contains("There are no relocations in this file."),
            "expected no relocation records in the final native-lib ELF, got:\n{relocations}"
        );
    }

    #[test]
    fn native_blob_contains_linked_sections_at_their_load_offsets() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_bin();

        let blob = fs::read(fixture.bin()).expect("native-lib binary bytes");
        for section_name in [".program_ptr", ".init_fun", ".got", ".text"] {
            let section = section_layout(&fixture, section_name);
            let section_bytes = section_binary(&fixture, section_name);
            let end = section.vma + section.size;

            assert!(
                end <= blob.len(),
                "section {section_name} at 0x{:x}..0x{:x} exceeds {}-byte blob",
                section.vma,
                end,
                blob.len()
            );
            assert_eq!(
                &blob[section.vma..end],
                section_bytes.as_slice(),
                "section {section_name} bytes must appear at the linked load offset"
            );
        }
    }

    fn bounded_init_disassembly(disassembly: &str) -> &str {
        disassembly
            .split("<init>:")
            .nth(1)
            .expect("expected init in disassembly")
            .split("\n\n")
            .next()
            .expect("expected bounded init disassembly")
    }

    fn assert_rust_loader_init_uses_vesc_ffi(init_disassembly: &str) {
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

    #[test]
    fn loader_init_registers_probe_through_vesc_ffi() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_bin();

        let init_fun = section_layout(&fixture, ".init_fun");
        assert_eq!(
            init_fun.vma, DEVICE_PROVEN_INIT_OFFSET,
            "expected .init_fun to start at the device-proven offset"
        );
        assert!(
            init_fun.size >= 24,
            "expected Rust-owned init to retain loader entry and probe registration"
        );

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        assert_rust_loader_init_uses_vesc_ffi(bounded_init_disassembly(&disassembly));

        let current = fs::read(fixture.bin()).expect("current native-lib binary bytes");
        let proven = fs::read(crate::test_support::repo_root().join(DEVICE_PROVEN_PACKAGE_BINARY))
            .expect("device-proven package binary bytes");
        let proven_init_end = DEVICE_PROVEN_INIT_OFFSET + DEVICE_PROVEN_INIT_SIZE;
        assert_ne!(
            &current[init_fun.vma..init_fun.vma + init_fun.size.min(DEVICE_PROVEN_INIT_SIZE)],
            &proven[DEVICE_PROVEN_INIT_OFFSET..proven_init_end],
            "Rust-owned init should no longer match the legacy hand-asm bytes in {DEVICE_PROVEN_PACKAGE_BINARY}"
        );
    }

    #[test]
    fn final_native_lib_uses_the_vesc_entry_section_order() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let program_ptr = section_layout(&fixture, ".program_ptr");
        let init_fun = section_layout(&fixture, ".init_fun");
        let got = section_layout(&fixture, ".got");
        let text = section_layout(&fixture, ".text");

        assert_eq!(
            program_ptr,
            SectionLayout {
                name: ".program_ptr".to_owned(),
                size: 4,
                vma: 0,
            }
        );
        assert_eq!(init_fun.vma, program_ptr.vma + program_ptr.size);
        assert!(
            init_fun.size >= 24,
            "expected Rust-owned init to retain loader entry and probe registration"
        );
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

    #[test]
    fn native_build_materializes_the_package_loader_object() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        assert!(
            fixture.package_object().exists(),
            "native build must materialize the C loader shim at {:?}",
            fixture.package_object()
        );
    }

    #[test]
    fn native_artifact_keeps_the_rust_owned_package_symbols() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let final_symbols = nm_output(&fixture.elf());
        let final_defined = defined_symbols(&final_symbols);

        assert!(
            final_defined.contains("ext_rust_probe_diag_v4"),
            "final image must retain the Rust LispBM probe callback:\n{final_symbols}"
        );
        assert!(
            final_defined.contains("init")
                && final_defined.contains("prog_ptr")
                && final_defined.contains("package_lib_init"),
            "native image must keep loader entry and Rust package init:\n{final_symbols}"
        );
    }

    #[test]
    fn final_native_lib_retains_the_rust_owned_boundary_symbols() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let output = nm_output(&fixture.elf());
        let defined = defined_symbols(&output);
        let undefined = undefined_symbols(&output);

        assert!(
            defined.contains("init") && defined.contains("package_lib_init"),
            "expected final native image to retain loader and Rust package init:\n{output}"
        );
        assert!(
            defined.contains("ext_rust_probe_diag_v4"),
            "expected final native image to retain the Rust LispBM probe:\n{output}"
        );
        assert!(
            !defined.contains("ext_c_probe_v12"),
            "expected final native image to drop the C LispBM probe body:\n{output}"
        );
        assert!(
            !defined.contains("ext_c_probe_v6"),
            "expected final native image to drop the temporary C probe after Rust-owned registration:\n{output}"
        );
        assert!(
            undefined.is_empty(),
            "expected final native image to resolve the C-to-Rust boundary completely:\n{output}"
        );
    }

    #[test]
    fn final_native_lib_calls_lispbm_through_the_vesc_function_table() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let symbols = nm_output(&fixture.elf());
        assert!(
            undefined_symbols(&symbols).is_empty(),
            "expected no unresolved firmware calls in the final native-lib ELF:\n{symbols}"
        );

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        for offset in ["1000f800", "#596]"] {
            assert!(
                disassembly.contains(offset),
                "expected VESC_IF slot access {offset} in generated code:\n{disassembly}"
            );
        }
        assert!(
            !disassembly.contains("<vesc_send_app_data>")
                && !disassembly.contains("<vesc_set_app_data_handler>"),
            "expected direct VESC_IF calls without C wrapper stubs:\n{disassembly}"
        );
        for symbol in ["<init>", "<package_lib_init>", "<ext_rust_probe_diag_v4>"] {
            assert!(
                disassembly.contains(symbol),
                "expected native image to retain `{symbol}`:\n{disassembly}"
            );
        }
        let init_disassembly = disassembly
            .split("<init>:")
            .nth(1)
            .expect("expected init in disassembly")
            .split("\n\nDisassembly")
            .next()
            .expect("expected bounded init disassembly");
        assert!(
            init_disassembly.contains("<package_lib_init>"),
            "loader init should run Rust package init before registering the probe:\n{init_disassembly}"
        );
        assert!(
            init_disassembly.contains("1000f800")
                && (init_disassembly.contains("4798")
                    || init_disassembly.contains("4790")
                    || init_disassembly.contains("4710")),
            "Rust loader init should register the probe inline through VESC_IF like refloat:\n{init_disassembly}"
        );
        assert!(
            !disassembly.contains("<register_package_extensions_asm>"),
            "Rust init should register directly without a registration trampoline:\n{disassembly}"
        );
    }

    #[test]
    fn rust_probe_extension_returns_the_encoded_probe_value_directly() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        let probe_start = disassembly
            .find("<ext_rust_probe_diag_v4>:")
            .expect("expected ext_rust_probe_diag_v4 in final native image");
        let probe_rest = &disassembly[probe_start..];
        let probe_end = probe_rest.find("\n\n0000").unwrap_or(probe_rest.len());
        let probe_disassembly = &probe_rest[..probe_end];

        assert!(
            probe_disassembly.contains("#680") || probe_disassembly.contains("0x2a8"),
            "Rust probe extension should return the LispBM-encoded integer 42 directly:\n{probe_disassembly}"
        );
        assert!(
            !probe_disassembly.contains("1000f800"),
            "Rust probe extension should not reject valid hardware calls through fragile LispBM validation slots:\n{probe_disassembly}"
        );
    }

    #[test]
    fn final_native_lib_registers_the_rust_probe_from_rust_loader_init() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        let init_disassembly = disassembly
            .split("<init>:")
            .nth(1)
            .expect("expected init in disassembly")
            .split("\n\nDisassembly")
            .next()
            .expect("expected bounded init disassembly");

        assert_rust_loader_init_uses_vesc_ffi(bounded_init_disassembly(&disassembly));
        assert!(
            init_disassembly.contains("4620")
                || init_disassembly.contains("movs\tr0, #1")
                || init_disassembly.contains("4710"),
            "loader init should return lbm_add_extension's result after package init and registration:\n{init_disassembly}"
        );
    }

    #[test]
    fn native_artifact_uses_the_rust_probe_with_rust_package_init() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );

        assert!(
            disassembly.contains("<ext_rust_probe_diag_v4>"),
            "native artifact should include the Rust LispBM probe:\n{disassembly}"
        );
        assert!(
            disassembly
                .split("<init>:")
                .nth(1)
                .and_then(|init| init.split("\n\nDisassembly").next())
                .is_some_and(|init| init.contains("1000f800") && init.contains("<package_lib_init>")),
            "loader init should call Rust package init and register the probe inline:\n{disassembly}"
        );
    }

    #[test]
    fn stop_package_clears_app_data_handler_like_refloat() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        let stop_start = disassembly
            .find("stop_package")
            .expect("expected stop_package in final native image");
        let stop_disassembly = disassembly[stop_start..]
            .split("\n\n")
            .next()
            .expect("expected bounded stop_package disassembly");

        assert!(
            stop_disassembly.contains("1000f800") && stop_disassembly.contains("#596]"),
            "stop_package should clear app-data through direct VESC_IF + 596 load like refloat:\n{stop_disassembly}"
        );
        assert!(
            !stop_disassembly.contains("cbz"),
            "stop_package should not guard the VESC_IF app-data slot; refloat calls it directly:\n{stop_disassembly}"
        );
    }

    #[test]
    fn loader_init_does_not_dereference_extension_names_before_rebase() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        let init_disassembly = bounded_init_disassembly(&disassembly);

        assert_rust_loader_init_uses_vesc_ffi(init_disassembly);
        assert!(
            !init_disassembly.contains("addw\tr1, pc") && !init_disassembly.contains("0ff2 2901"),
            "loader init should pass the probe callback through vesc-ffi, not legacy hand-asm PC-relative registration:\n{init_disassembly}"
        );
    }

    #[test]
    fn loader_init_reports_success_after_delegating_to_rust_package_init() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), fixture.elf()],
        );
        let init_disassembly = disassembly
            .split("<init>:")
            .nth(1)
            .expect("expected init in disassembly")
            .split("\n\n")
            .next()
            .expect("expected bounded init disassembly");

        assert!(
            init_disassembly.contains("<package_lib_init>"),
            "loader init should run Rust package init:\n{init_disassembly}"
        );
        assert!(
            init_disassembly.contains("1000f800")
                && (init_disassembly.contains("4798")
                    || init_disassembly.contains("4790")
                    || init_disassembly.contains("4710")),
            "loader init should register the LispBM probe inline:\n{init_disassembly}"
        );
    }

    #[test]
    fn final_native_lib_elf_allows_only_the_expected_firmware_calls() {
        assert!(is_allowed_final_native_lib_symbol("lbm_add_extension"));
        assert!(is_allowed_final_native_lib_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_final_native_lib_symbol("lbm_enc_i"));
        assert!(!is_allowed_final_native_lib_symbol("rust_add"));

        let sample = "\
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
";

        assert_eq!(
            unexpected_final_native_lib_undefined_symbols(sample),
            BTreeSet::new()
        );
    }

    #[test]
    fn rust_staticlib_exports_the_package_init_entrypoint() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_staticlib();
        let output = nm_output(&fixture.staticlib());
        let defined = defined_symbols(&output);

        assert!(
            defined.contains("package_lib_init"),
            "expected the Rust staticlib to export package_lib_init"
        );
        assert!(defined.contains("init") && defined.contains("prog_ptr"));
    }

    #[test]
    fn rust_staticlib_exports_loader_entry_and_rust_probe_dependency() {
        let fixture = SymbolAuditFixture::new();
        fixture.build_staticlib();
        fixture.build_elf();

        let staticlib_symbols = nm_output(&fixture.staticlib());
        let staticlib_defined = defined_symbols(&staticlib_symbols);
        let staticlib_undefined = undefined_symbols(&staticlib_symbols);

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
        assert!(
            fixture.package_object().exists(),
            "package C object must be linked into the final native build"
        );
    }

    fn command_stdout(program: &str, args: impl IntoIterator<Item = impl AsRef<Path>>) -> String {
        let output = Command::new(program)
            .args(args.into_iter().map(|arg| arg.as_ref().to_owned()))
            .output()
            .unwrap_or_else(|error| panic!("{program} execution failed: {error}"));

        assert!(
            output.status.success(),
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("command stdout to be valid UTF-8")
    }

    fn section_layout(fixture: &SymbolAuditFixture, section_name: &str) -> SectionLayout {
        let sections = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-h"), fixture.elf()],
        );
        sections
            .lines()
            .filter_map(parse_section_layout)
            .find(|section| section.name == section_name)
            .unwrap_or_else(|| panic!("section {section_name} not found in:\n{sections}"))
    }

    fn parse_section_layout(line: &str) -> Option<SectionLayout> {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        let [_, name, size, vma, ..] = parts.as_slice() else {
            return None;
        };
        if !name.starts_with('.') {
            return None;
        }

        Some(SectionLayout {
            name: (*name).to_owned(),
            size: usize::from_str_radix(size, 16).ok()?,
            vma: usize::from_str_radix(vma, 16).ok()?,
        })
    }

    fn section_binary(fixture: &SymbolAuditFixture, section_name: &str) -> Vec<u8> {
        let output_path = section_binary_path(fixture, section_name);
        let status = Command::new("arm-none-eabi-objcopy")
            .args([
                "-O",
                "binary",
                "--only-section",
                section_name,
                fixture.elf().to_str().expect("utf-8 ELF path"),
                output_path.to_str().expect("utf-8 section binary path"),
            ])
            .status()
            .expect("arm-none-eabi-objcopy section extraction");
        assert!(
            status.success(),
            "failed to extract section {section_name} from native-lib ELF"
        );

        fs::read(output_path).expect("section binary bytes")
    }

    fn section_binary_path(fixture: &SymbolAuditFixture, section_name: &str) -> PathBuf {
        fixture.bin().with_file_name(format!(
            "native_lib_{}.bin",
            section_name.trim_start_matches('.')
        ))
    }
}
