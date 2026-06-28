use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, OnceLock};
use std::time::SystemTime;

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

fn artifact_is_up_to_date(output: &Path, inputs: &[&Path]) -> bool {
    let Ok(output_meta) = fs::metadata(output) else {
        return false;
    };
    let Ok(output_modified) = output_meta.modified() else {
        return false;
    };

    inputs.iter().all(|input| {
        fs::metadata(input)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified <= output_modified)
    })
}

fn newest_rs_tree_mtime(dir: &Path) -> Option<SystemTime> {
    let mut stack = vec![dir.to_path_buf()];
    let mut newest = None;

    while let Some(path) = stack.pop() {
        let Ok(read_dir) = fs::read_dir(path) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
                    newest =
                        Some(newest.map_or(modified, |current: SystemTime| current.max(modified)));
                }
            }
        }
    }

    newest
}

fn rust_staticlib_is_up_to_date(plan: &NativeLibLinkPlan) -> bool {
    let staticlib = plan.rust_staticlib_path();
    let Ok(output_modified) = fs::metadata(&staticlib).and_then(|meta| meta.modified()) else {
        return false;
    };

    let root = plan.root();
    for crate_dir in ["crates/vesc-ble-loopback", "crates/vesc-package"] {
        if newest_rs_tree_mtime(&root.join(crate_dir).join("src"))
            .is_some_and(|mtime| mtime > output_modified)
        {
            return false;
        }
    }

    [
        root.join("Cargo.lock"),
        root.join("crates/vesc-ble-loopback/Cargo.toml"),
        root.join("crates/vesc-package/Cargo.toml"),
    ]
    .iter()
    .all(|input| {
        fs::metadata(input)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified <= output_modified)
    })
}

fn is_repo_native_build_plan(plan: &NativeLibLinkPlan) -> bool {
    plan.root() == native_lib_link_plan().root()
}

struct RepoNativeBuildCache {
    staticlib: OnceLock<()>,
    elf: OnceLock<()>,
    bin: OnceLock<()>,
}

impl RepoNativeBuildCache {
    const fn new() -> Self {
        Self {
            staticlib: OnceLock::new(),
            elf: OnceLock::new(),
            bin: OnceLock::new(),
        }
    }

    fn ensure_staticlib(&self, plan: &NativeLibLinkPlan) {
        self.staticlib
            .get_or_init(|| build_rust_staticlib_unlocked(plan));
    }

    fn ensure_elf(&self, plan: &NativeLibLinkPlan) {
        self.elf
            .get_or_init(|| build_final_native_lib_elf_unlocked(plan));
    }

    fn ensure_bin(&self, plan: &NativeLibLinkPlan, native_binary_path: &Path) {
        self.bin.get_or_init(|| {
            build_final_native_lib_binary_unlocked(plan, native_binary_path);
        });
    }
}

static REPO_NATIVE_BUILD: LazyLock<RepoNativeBuildCache> = LazyLock::new(RepoNativeBuildCache::new);

pub fn build_rust_staticlib_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        REPO_NATIVE_BUILD.ensure_staticlib(plan);
    } else {
        build_rust_staticlib_unlocked(plan);
    }
}

fn build_rust_staticlib_unlocked(plan: &NativeLibLinkPlan) {
    if rust_staticlib_is_up_to_date(plan) {
        return;
    }

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
            "vesc-ble-loopback",
        ])
        .status()
        .expect("cargo build for the Rust staticlib");

    assert!(status.success(), "cargo failed to build the Rust staticlib");
}

pub fn build_final_native_lib_elf() {
    build_final_native_lib_elf_for(&native_lib_link_plan());
}

pub fn build_final_native_lib_elf_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        REPO_NATIVE_BUILD.ensure_elf(plan);
    } else {
        build_final_native_lib_elf_unlocked(plan);
    }
}

fn native_lib_c_only_from_env() -> bool {
    std::env::var("VESC_NATIVE_LIB_C_ONLY").ok().as_deref() == Some("1")
}

fn build_final_native_lib_elf_unlocked(plan: &NativeLibLinkPlan) {
    let c_only = native_lib_c_only_from_env();
    let link_plan = plan.clone();
    let elf_path = link_plan.elf_path();
    let package_c_source_path = link_plan.package_c_source_path();
    let linker_script_path = link_plan.linker_script_path();
    let rust_staticlib_path = link_plan.rust_staticlib_path();
    let mut elf_inputs = vec![
        package_c_source_path.as_path(),
        linker_script_path.as_path(),
    ];
    if !c_only {
        elf_inputs.push(rust_staticlib_path.as_path());
    }
    if artifact_is_up_to_date(&elf_path, &elf_inputs) {
        return;
    }

    if !c_only {
        build_rust_staticlib_for(plan);
    }

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
    if is_repo_native_build_plan(plan) {
        REPO_NATIVE_BUILD.ensure_bin(plan, native_binary_path);
    } else {
        build_final_native_lib_binary_unlocked(plan, native_binary_path);
    }
}

fn build_final_native_lib_binary_unlocked(plan: &NativeLibLinkPlan, native_binary_path: &Path) {
    build_final_native_lib_elf_for(plan);

    if artifact_is_up_to_date(native_binary_path, &[plan.elf_path().as_path()]) {
        return;
    }

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
        build_final_native_lib_binary_for, defined_symbols, is_allowed_final_native_lib_symbol,
        is_allowed_runtime_symbol, nm_output, undefined_symbols,
        unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    };
    use crate::native_lib_link::NativeLibLinkPlan;
    use crate::test_support::NativeBuildWorkspace;
    use std::collections::{BTreeMap, BTreeSet};

    struct SymbolAuditFixture {
        workspace: &'static NativeBuildWorkspace,
    }

    impl SymbolAuditFixture {
        fn new() -> Self {
            Self {
                workspace: NativeBuildWorkspace::shared(),
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

        fn build_bin(&self) {
            build_final_native_lib_binary_for(self.plan(), &self.bin());
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
    fn symbol_audit_helpers_classify_nm_output() {
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

        assert!(is_allowed_runtime_symbol("__aeabi_dadd"));
        assert!(is_allowed_runtime_symbol(
            "_RNvNtNtCseGTyb2smT0B_17compiler_builtins3mem6memcpy"
        ));
        assert!(is_allowed_runtime_symbol("fma"));
        assert!(is_allowed_runtime_symbol("lbm_add_extension"));
        assert!(is_allowed_runtime_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_runtime_symbol("lbm_enc_i"));

        let unexpected_sample = "\
         U __aeabi_dadd
         U fma
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
         U plain_external
";

        assert_eq!(
            unexpected_undefined_symbols(unexpected_sample),
            BTreeSet::from(["plain_external".to_owned()])
        );

        assert!(is_allowed_final_native_lib_symbol("lbm_add_extension"));
        assert!(is_allowed_final_native_lib_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_final_native_lib_symbol("lbm_enc_i"));
        assert!(!is_allowed_final_native_lib_symbol("rust_add"));

        let final_sample = "\
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
";

        assert_eq!(
            unexpected_final_native_lib_undefined_symbols(final_sample),
            BTreeSet::new()
        );
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

    fn all_section_layouts(fixture: &SymbolAuditFixture) -> BTreeMap<String, SectionLayout> {
        let sections = command_stdout(
            "arm-none-eabi-objdump",
            [PathBuf::from("-h"), fixture.elf()],
        );
        sections
            .lines()
            .filter_map(parse_section_layout)
            .map(|section| (section.name.clone(), section))
            .collect()
    }

    fn section_from<'a>(
        sections: &'a BTreeMap<String, SectionLayout>,
        section_name: &str,
    ) -> &'a SectionLayout {
        sections
            .get(section_name)
            .unwrap_or_else(|| panic!("section {section_name} not found in native-lib ELF headers"))
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

    include!("native_lib_audit.rs");
}
