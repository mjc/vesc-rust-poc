//! Shared Cargo build-script support for VESC package examples.

use std::env;
use std::fs;
use std::path::Path;

/// Copy package assets and configure the ARM package linker when requested.
pub fn build_package(manifest_dir: &Path) {
    let out_dir = env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR");
    let assets = Path::new(&out_dir).join("vescpkg");
    copy_package_assets(manifest_dir, &assets);

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

fn copy_package_assets(manifest_dir: &Path, assets: &Path) {
    if assets.exists() {
        fs::remove_dir_all(assets).expect("clear package asset directory");
    }
    fs::create_dir_all(assets.join("src")).expect("create package asset directory");
    for name in ["README.md", "pkgdesc.qml", "code.lisp"] {
        let source = manifest_dir.join("package").join(name);
        if source.exists() {
            fs::copy(&source, assets.join(name)).expect("copy package asset");
        }
        println!("cargo::rerun-if-changed={}", source.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_package_assets_do_not_survive_a_rebuild() {
        let root = env::temp_dir().join(format!("vescpkg-build-support-{}", std::process::id()));
        let manifest = root.join("manifest");
        let source = manifest.join("package/README.md");
        let assets = root.join("assets");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(source.parent().expect("package directory")).expect("create package");
        fs::write(&source, "old description").expect("write package asset");

        copy_package_assets(&manifest, &assets);
        fs::remove_file(source).expect("remove package asset");
        copy_package_assets(&manifest, &assets);

        assert!(!assets.join("README.md").exists());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
