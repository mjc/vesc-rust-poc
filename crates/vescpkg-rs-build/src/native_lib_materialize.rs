use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use crate::native_inspect::elf_to_flat_binary;
use crate::native_lib_link::NativeLibLinkPlan;
use crate::native_lib_toolchain::{NativeLibToolchain, RealNativeLibToolchain};

pub(crate) fn artifact_is_up_to_date(output: &Path, inputs: &[&Path]) -> bool {
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
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(modified) = entry.metadata().and_then(|meta| meta.modified())
            {
                newest = Some(newest.map_or(modified, |current: SystemTime| current.max(modified)));
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
    for crate_dir in [
        plan.rust_package_source_path(),
        root.join("crates/vescpkg-rs"),
    ] {
        if newest_rs_tree_mtime(&crate_dir.join("src")).is_some_and(|mtime| mtime > output_modified)
        {
            return false;
        }
    }

    [
        root.join("Cargo.lock"),
        plan.rust_package_source_path().join("Cargo.toml"),
        root.join("crates/vescpkg-rs/Cargo.toml"),
    ]
    .iter()
    .all(|input| {
        fs::metadata(input)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified <= output_modified)
    })
}

pub(crate) fn build_rust_staticlib_unlocked(plan: &NativeLibLinkPlan) -> Result<(), String> {
    if rust_staticlib_is_up_to_date(plan) {
        return Ok(());
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
        ])
        .arg(plan.rust_package_name())
        .status()
        .map_err(|error| format!("cargo build for the Rust staticlib: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("cargo failed to build the Rust staticlib".to_owned())
    }
}

pub(crate) fn build_final_native_lib_elf_unlocked(
    plan: &NativeLibLinkPlan,
    toolchain: &impl NativeLibToolchain,
) -> Result<(), String> {
    let link_plan = plan.clone();
    let elf_path = link_plan.elf_path();
    let linker_script_path = link_plan.linker_script_path();
    let rust_staticlib_path = link_plan.rust_staticlib_path();

    build_rust_staticlib_unlocked(plan)?;

    let elf_inputs = [linker_script_path.as_path(), rust_staticlib_path.as_path()];
    if artifact_is_up_to_date(&elf_path, &elf_inputs) {
        return Ok(());
    }

    if let Some(parent) = elf_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.elf parent directory");
    }
    let stale_object_path = link_plan.package_c_object_path();
    if stale_object_path.exists() {
        fs::remove_file(&stale_object_path).expect("remove stale package_lib.o");
    }

    let staticlib_path = link_plan.rust_staticlib_path();
    let linker_script = link_plan.linker_script_path();
    let elf_path_str = elf_path.to_str().expect("utf-8 ELF path");

    let mut link_args = vec![
        "-nostartfiles",
        "-static",
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        staticlib_path.to_str().expect("utf-8 staticlib path"),
    ];
    link_args.extend([
        "-Wl,--gc-sections",
        "-Wl,--undefined=init",
        "-T",
        linker_script.to_str().expect("utf-8 linker script path"),
        "-o",
        elf_path_str,
    ]);

    toolchain.run("arm-none-eabi-gcc", &link_args)
}

pub(crate) fn materialize_native_lib_binary_unlocked(
    plan: &NativeLibLinkPlan,
    native_binary_path: &Path,
    toolchain: &impl NativeLibToolchain,
) -> Result<(), String> {
    build_final_native_lib_elf_unlocked(plan, toolchain)?;

    if artifact_is_up_to_date(native_binary_path, &[plan.elf_path().as_path()]) {
        return Ok(());
    }

    if let Some(parent) = native_binary_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.bin parent directory");
    }

    let blob = elf_to_flat_binary(&plan.elf_path());
    fs::write(native_binary_path, blob).map_err(|error| error.to_string())
}

pub(crate) fn materialize_native_lib_binary(plan: &NativeLibLinkPlan, native_binary_path: &Path) {
    materialize_native_lib_binary_unlocked(plan, native_binary_path, &RealNativeLibToolchain)
        .expect("materialize native-lib binary");
}

pub(crate) fn build_final_native_lib_elf(plan: &NativeLibLinkPlan) {
    build_final_native_lib_elf_unlocked(plan, &RealNativeLibToolchain)
        .expect("build native-lib ELF");
}

pub(crate) fn build_rust_staticlib(plan: &NativeLibLinkPlan) {
    build_rust_staticlib_unlocked(plan).expect("build rust staticlib");
}
