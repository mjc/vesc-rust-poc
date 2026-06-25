use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    symbol.starts_with('_') || symbol == "fma"
}

pub fn rust_staticlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target/thumbv7em-none-eabihf/release/libvesc_rust_poc.a")
}

pub fn native_lib_elf_path() -> PathBuf {
    crate::native_lib_link::native_lib_elf_path()
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
    build_rust_staticlib();

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
            "-std=c11",
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
            "-r",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            object_path.to_str().expect("utf-8 object path"),
            rust_staticlib_path()
                .to_str()
                .expect("utf-8 staticlib path"),
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
    matches!(symbol, "lbm_add_extension" | "lbm_dec_as_i32" | "lbm_enc_i")
        || is_allowed_runtime_symbol(symbol)
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
    use super::{
        audit_final_native_lib_elf_symbols, audit_rust_staticlib_symbols, defined_symbols,
        is_allowed_final_native_lib_symbol, is_allowed_runtime_symbol, undefined_symbols,
        unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    };
    use std::collections::BTreeSet;

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
        assert!(!is_allowed_runtime_symbol("lbm_add_extension"));

        let sample = "\
         U __aeabi_dadd
         U fma
         U lbm_add_extension
";

        assert_eq!(
            unexpected_undefined_symbols(sample),
            BTreeSet::from(["lbm_add_extension".to_owned()])
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
}
