use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, OnceLock};
use std::time::SystemTime;

use crate::native_lib_link::{native_lib_link_plan, NativeLibLinkPlan};

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
    if is_repo_native_build_plan(plan) {
        REPO_NATIVE_BUILD.ensure_staticlib(plan);
    } else {
        build_rust_staticlib_unlocked(plan);
    }
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
