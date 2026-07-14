//! Shared Cargo build-script support for VESC package examples.

use std::env;
use std::fs;
use std::path::Path;

/// Copy package assets and configure the ARM package linker when requested.
pub fn build_package(manifest_dir: &Path) {
    let out_dir = env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR");
    let assets = Path::new(&out_dir).join("vescpkg");
    fs::create_dir_all(assets.join("src")).expect("create package asset directory");
    for name in ["README.md", "pkgdesc.qml", "code.lisp"] {
        let source = manifest_dir.join("package").join(name);
        if source.exists() {
            fs::copy(&source, assets.join(name)).expect("copy package asset");
        }
        println!("cargo::rerun-if-changed={}", source.display());
    }

    if env::var("TARGET").is_ok_and(|target| target == "thumbv7em-none-eabihf") {
        let linker_script = manifest_dir.join("../vescpkg-link.ld");
        println!("cargo::rerun-if-changed={}", linker_script.display());
        for arg in [
            "-nostartfiles",
            "-static",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-Wl,--gc-sections",
            "-Wl,--undefined=init",
        ] {
            println!("cargo::rustc-link-arg={arg}");
        }
        println!("cargo::rustc-link-arg=-T{}", linker_script.display());
    }
}
