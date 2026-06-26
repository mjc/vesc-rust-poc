use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

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
        || symbol == "ext_c_probe_v6"
        || matches!(symbol, "lbm_add_extension" | "lbm_dec_as_i32" | "lbm_enc_i")
}

pub fn rust_staticlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/thumbv7em-none-eabihf/release/libvesc_rust_poc.a")
}

pub fn native_lib_elf_path() -> PathBuf {
    crate::native_lib_link::native_lib_elf_path()
}

pub fn native_lib_bin_path() -> PathBuf {
    native_lib_elf_path().with_file_name("native_lib.bin")
}

pub fn package_lib_c_path() -> PathBuf {
    crate::native_lib_baseline_root()
        .input_paths()
        .find(|path| path.ends_with("src/package_lib.c"))
        .expect("native-lib baseline package_lib.c")
}

pub fn package_lib_object_path() -> PathBuf {
    crate::native_lib_link::native_lib_link_plan().shim_object_path()
}

pub fn build_rust_staticlib() {
    let _guard = native_build_lock().lock().expect("native build lock");
    build_rust_staticlib_unlocked();
}

fn build_rust_staticlib_unlocked() {
    let status = Command::new("cargo")
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
    let _guard = native_build_lock().lock().expect("native build lock");
    build_final_native_lib_elf_unlocked();
}

fn build_final_native_lib_elf_unlocked() {
    build_rust_staticlib_unlocked();

    let c_path = package_lib_c_path();
    let object_path = package_lib_object_path();
    let elf_path = native_lib_elf_path();
    let include_dir = c_path
        .parent()
        .expect("package_lib.c parent directory")
        .to_path_buf();

    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent).expect("create package_lib.o parent directory");
    }
    if let Some(parent) = elf_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.elf parent directory");
    }

    let compile_status = Command::new("arm-none-eabi-gcc")
        .args([
            "-c",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-fpic",
            "-Os",
            "-ffunction-sections",
            "-fdata-sections",
            "-fomit-frame-pointer",
            "-std=c11",
            "-DIS_VESC_LIB",
            "-I",
            include_dir.to_str().expect("utf-8 include directory"),
            c_path.to_str().expect("utf-8 C source path"),
            "-o",
            object_path.to_str().expect("utf-8 object path"),
        ])
        .status()
        .expect("arm-none-eabi-gcc compile of package_lib.c");

    assert!(
        compile_status.success(),
        "failed to compile the C shim into package_lib.o"
    );

    let link_status = Command::new("arm-none-eabi-gcc")
        .args([
            "-nostartfiles",
            "-static",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            object_path.to_str().expect("utf-8 object path"),
            rust_staticlib_path()
                .to_str()
                .expect("utf-8 staticlib path"),
            "-Wl,--gc-sections",
            "-Wl,--undefined=init",
            "-T",
            crate::native_lib_link::native_lib_link_plan()
                .linker_script_path()
                .to_str()
                .expect("utf-8 linker script path"),
            "-o",
            elf_path.to_str().expect("utf-8 ELF path"),
        ])
        .status()
        .expect("arm-none-eabi-gcc link of the final native-lib ELF");

    assert!(
        link_status.success(),
        "failed to link the final native-lib ELF"
    );
}

fn native_build_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn build_final_native_lib_binary(native_binary_path: &Path) {
    let _guard = native_build_lock().lock().expect("native build lock");
    build_final_native_lib_elf_unlocked();

    if let Some(parent) = native_binary_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.bin parent directory");
    }

    let objcopy_status = Command::new("arm-none-eabi-objcopy")
        .args([
            "-O",
            "binary",
            native_lib_elf_path()
                .to_str()
                .expect("utf-8 native-lib ELF path"),
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
        audit_final_native_lib_elf_symbols, audit_rust_staticlib_symbols,
        build_final_native_lib_binary, build_final_native_lib_elf, build_rust_staticlib,
        defined_symbols, is_allowed_final_native_lib_symbol, is_allowed_runtime_symbol,
        native_lib_bin_path, native_lib_elf_path, nm_output, package_lib_object_path,
        rust_staticlib_path, undefined_symbols, unexpected_final_native_lib_undefined_symbols,
        unexpected_undefined_symbols,
    };
    use std::collections::BTreeSet;

    #[derive(Debug, PartialEq, Eq)]
    struct SectionLayout {
        name: String,
        size: usize,
        vma: usize,
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
        assert!(
            audit_rust_staticlib_symbols().is_empty(),
            "unexpected undefined symbols remain in the Rust staticlib"
        );
    }

    #[test]
    fn final_native_lib_elf_has_no_unexpected_undefined_symbols() {
        assert!(
            audit_final_native_lib_elf_symbols().is_empty(),
            "unexpected undefined symbols remain in the final native-lib ELF"
        );
    }

    #[test]
    fn build_final_native_lib_binary_materializes_the_packageable_payload() {
        build_final_native_lib_binary(&native_lib_bin_path());

        assert!(
            native_lib_bin_path().exists(),
            "expected the final native-lib binary to be materialized"
        );
        let native_bin_size = fs::metadata(native_lib_bin_path())
            .expect("native-lib binary metadata")
            .len();
        assert!(
            native_bin_size <= 640,
            "expected the native blob to stay compact, got {native_bin_size} bytes"
        );
    }

    #[test]
    fn final_native_lib_elf_is_a_fully_linked_executable_image() {
        build_final_native_lib_elf();

        let header = command_stdout(
            "arm-none-eabi-readelf",
            [PathBuf::from("-h"), native_lib_elf_path()],
        );
        assert!(
            header.contains("Type:                              EXEC"),
            "expected a final executable ELF, got:\n{header}"
        );

        let relocations = command_stdout(
            "arm-none-eabi-readelf",
            [PathBuf::from("-r"), native_lib_elf_path()],
        );
        assert!(
            relocations.contains("There are no relocations in this file."),
            "expected no relocation records in the final native-lib ELF, got:\n{relocations}"
        );
    }

    #[test]
    fn native_blob_contains_linked_sections_at_their_load_offsets() {
        build_final_native_lib_binary(&native_lib_bin_path());

        let blob = fs::read(native_lib_bin_path()).expect("native-lib binary bytes");
        for section_name in [".program_ptr", ".init_fun", ".got", ".text"] {
            let section = section_layout(section_name);
            let section_bytes = section_binary(section_name);
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

    #[test]
    fn final_native_lib_uses_the_vesc_entry_section_order() {
        build_final_native_lib_elf();

        let program_ptr = section_layout(".program_ptr");
        let init_fun = section_layout(".init_fun");
        let got = section_layout(".got");
        let text = section_layout(".text");

        assert_eq!(
            program_ptr,
            SectionLayout {
                name: ".program_ptr".to_owned(),
                size: 4,
                vma: 0,
            }
        );
        assert_eq!(
            init_fun,
            SectionLayout {
                name: ".init_fun".to_owned(),
                size: 44,
                vma: 4,
            }
        );
        assert_eq!(
            got,
            SectionLayout {
                name: ".got".to_owned(),
                size: 0,
                vma: 48,
            }
        );
        assert_eq!(
            text,
            SectionLayout {
                name: ".text".to_owned(),
                size: 433,
                vma: 48,
            }
        );
        assert_eq!(init_fun.vma, program_ptr.vma + program_ptr.size);
        assert_eq!(got.vma, init_fun.vma + init_fun.size);
        assert!(
            text.vma >= got.vma + got.size,
            "expected .text to load after .got"
        );
        assert_eq!(
            text.vma % 16,
            0,
            "expected .text to keep VESC's 16-byte function alignment"
        );
    }

    #[test]
    fn c_shim_object_exposes_only_the_current_loader_and_probe_contract() {
        build_final_native_lib_elf();

        let output = nm_output(&package_lib_object_path());
        let defined = defined_symbols(&output);

        assert!(
            defined.contains("ext_c_probe_v6"),
            "expected C shim object to define the temporary C probe symbol:\n{output}"
        );
        assert!(
            !defined.contains("init"),
            "Rust, not the C shim object, should define the loader init symbol:\n{output}"
        );
        assert!(
            !defined.contains("package_lib_init"),
            "Rust, not the C shim object, should define package_lib_init:\n{output}"
        );
        assert!(
            !defined.iter().any(|symbol| symbol.starts_with("ext_rust")),
            "C shim object must not define Rust-owned extension symbols:\n{output}"
        );
    }

    #[test]
    fn final_native_lib_retains_the_current_c_shim_boundary_symbols() {
        build_final_native_lib_elf();

        let output = nm_output(&native_lib_elf_path());
        let defined = defined_symbols(&output);
        let undefined = undefined_symbols(&output);

        for symbol in ["init", "ext_c_probe_v6", "package_lib_init"] {
            assert!(
                defined.contains(symbol),
                "expected final native image to retain current boundary symbol `{symbol}`:\n{output}"
            );
        }
        assert!(
            undefined.is_empty(),
            "expected final native image to resolve the C-to-Rust boundary completely:\n{output}"
        );
    }

    #[test]
    fn final_native_lib_calls_lispbm_through_the_vesc_function_table() {
        build_final_native_lib_elf();

        let symbols = nm_output(&native_lib_elf_path());
        assert!(
            undefined_symbols(&symbols).is_empty(),
            "expected no unresolved firmware calls in the final native-lib ELF:\n{symbols}"
        );

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), native_lib_elf_path()],
        );
        assert!(
            disassembly.contains("1000f800"),
            "expected generated package code to call through VESC_IF at 0x1000f800:\n{disassembly}"
        );
        for offset in ["1000f840", "1000fa50", "[r6, #360]", "[r6, #0]", "#596]"] {
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
        assert!(
            disassembly.contains("<init>"),
            "expected a VESC init entrypoint in .init_fun:\n{disassembly}"
        );
        assert!(
            disassembly.contains("<package_lib_init>"),
            "expected init to retain the Rust package entrypoint:\n{disassembly}"
        );
        let package_init_disassembly = disassembly
            .split("<package_lib_init>:")
            .nth(1)
            .expect("expected package_lib_init in disassembly");
        let app_data_register = package_init_disassembly
            .find("#596]")
            .expect("expected app-data handler registration in package_lib_init");
        let rust_extension_register = package_init_disassembly[app_data_register..]
            .find("#0]")
            .map(|offset| app_data_register + offset)
            .expect("expected Rust extension registration through lbm_add_extension");
        assert!(
            app_data_register < rust_extension_register,
            "expected package_lib_init to mirror VESC packages by registering app-data before LispBM extensions:\n{package_init_disassembly}"
        );
    }

    #[test]
    fn final_native_lib_rebases_rust_callbacks_from_the_loaded_image_base() {
        build_final_native_lib_elf();

        let disassembly = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-d"), native_lib_elf_path()],
        );
        let package_init_disassembly = disassembly
            .split("<package_lib_init>:")
            .nth(1)
            .expect("expected package_lib_init in disassembly");

        assert!(
            package_init_disassembly.contains("[r0, #8]"),
            "expected Rust package init to load lib_info.base_addr before registering Rust-owned pointers:\n{package_init_disassembly}"
        );
        for rebase_step in ["add\tr1, r0", "add\tr0, r1", "add\tr1, r2"] {
            assert!(
                package_init_disassembly.contains(rebase_step),
                "expected Rust-owned image pointer rebase step `{rebase_step}` before use:\n{package_init_disassembly}"
            );
        }
        assert!(
            package_init_disassembly.contains("str\tr1, [r4, #0]"),
            "expected the stop hook to store a rebased function pointer:\n{package_init_disassembly}"
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
         U rust_add
";

        assert_eq!(
            unexpected_final_native_lib_undefined_symbols(sample),
            BTreeSet::from(["rust_add".to_owned()])
        );
    }

    #[test]
    fn rust_staticlib_exports_the_package_init_entrypoint() {
        build_rust_staticlib();
        let output = nm_output(&rust_staticlib_path());
        let defined = defined_symbols(&output);

        assert!(
            defined.contains("package_lib_init"),
            "expected the Rust staticlib to export package_lib_init"
        );
        assert!(
            defined.contains("init"),
            "expected the Rust staticlib to export the loader init trampoline"
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

    fn section_layout(section_name: &str) -> SectionLayout {
        let sections = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-h"), native_lib_elf_path()],
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

    fn section_binary(section_name: &str) -> Vec<u8> {
        let output_path = section_binary_path(section_name);
        let status = Command::new("arm-none-eabi-objcopy")
            .args([
                "-O",
                "binary",
                "--only-section",
                section_name,
                native_lib_elf_path().to_str().expect("utf-8 ELF path"),
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

    fn section_binary_path(section_name: &str) -> PathBuf {
        native_lib_bin_path().with_file_name(format!(
            "native_lib_{}.bin",
            section_name.trim_start_matches('.')
        ))
    }
}
