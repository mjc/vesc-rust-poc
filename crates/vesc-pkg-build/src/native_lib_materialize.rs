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
            "vesc-ble-loopback",
        ])
        .status()
        .map_err(|error| format!("cargo build for the Rust staticlib: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("cargo failed to build the Rust staticlib".to_owned())
    }
}

fn native_lib_c_only_from_env() -> bool {
    std::env::var("VESC_NATIVE_LIB_C_ONLY").ok().as_deref() == Some("1")
}

fn compile_package_c_shim(
    plan: &NativeLibLinkPlan,
    toolchain: &impl NativeLibToolchain,
) -> Result<(), String> {
    let source = plan.package_c_source_path();
    let object = plan.package_c_object_path();
    let include = source
        .parent()
        .expect("package C source parent")
        .to_str()
        .expect("utf-8 package C source parent");

    toolchain.run(
        "arm-none-eabi-gcc",
        &[
            "-c",
            source.to_str().expect("utf-8 package C source path"),
            "-o",
            object.to_str().expect("utf-8 package C object path"),
            "-fpic",
            "-Os",
            "-Wall",
            "-Wextra",
            "-Wundef",
            "-std=gnu99",
            "-I",
            include,
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
        ],
    )
}

pub(crate) fn build_final_native_lib_elf_unlocked(
    plan: &NativeLibLinkPlan,
    toolchain: &impl NativeLibToolchain,
) -> Result<(), String> {
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
        return Ok(());
    }

    if !c_only {
        build_rust_staticlib_unlocked(plan)?;
    }

    if let Some(parent) = elf_path.parent() {
        fs::create_dir_all(parent).expect("create native_lib.elf parent directory");
    }
    let stale_object_path = link_plan.package_c_object_path();
    if stale_object_path.exists() {
        fs::remove_file(&stale_object_path).expect("remove stale package_lib.o");
    }

    compile_package_c_shim(plan, toolchain)?;

    let object_path = link_plan.package_c_object_path();
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
        object_path.to_str().expect("utf-8 package C object path"),
    ];
    if !c_only {
        link_args.push(staticlib_path.to_str().expect("utf-8 staticlib path"));
    }
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
