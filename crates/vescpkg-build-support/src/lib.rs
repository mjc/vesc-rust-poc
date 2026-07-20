//! Shared Cargo build-script support for VESC package examples.

use std::env;
use std::fs;
use std::path::Path;

/// Copy package assets and configure the ARM package linker when requested.
pub fn build_package(manifest_dir: &Path) {
    prepare_package_assets(manifest_dir);

    if env::var("TARGET").is_ok_and(|target| target == "thumbv7em-none-eabihf") {
        let linker_script = manifest_dir.join("../vescpkg-link.ld");
        println!("cargo::rerun-if-changed={}", linker_script.display());
        for arg in [
            "-static",
            "--emit-relocs",
            "--gc-sections",
            "--undefined=init",
            "--entry=init",
        ] {
            println!("cargo::rustc-link-arg={arg}");
        }
        println!("cargo::rustc-link-arg=-T{}", linker_script.display());
    }
}

/// Prepare a clean generated-asset directory and copy static package assets into it.
pub fn prepare_package_assets(manifest_dir: &Path) -> std::path::PathBuf {
    let out_dir = env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR");
    let assets = Path::new(&out_dir).join("vescpkg");
    copy_package_assets(manifest_dir, &assets);
    assets
}

fn copy_package_assets(manifest_dir: &Path, assets: &Path) {
    let package = manifest_dir.join("package");
    println!("cargo::rerun-if-changed={}", package.display());
    if assets.exists() {
        fs::remove_dir_all(assets).expect("clear package asset directory");
    }
    fs::create_dir_all(assets).expect("create package asset directory");
    if package.exists() {
        copy_asset_tree(&package, assets, &package).expect("copy package assets");
    }
}

fn copy_asset_tree(source: &Path, destination: &Path, root: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source = entry.path();
        let relative = source
            .strip_prefix(root)
            .expect("asset is below package root");
        if relative == Path::new("src/package_lib.bin") {
            return Err(std::io::Error::other(
                "package asset `src/package_lib.bin` conflicts with the native payload",
            ));
        }
        let destination = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&destination)?;
            copy_asset_tree(&source, &destination, root)?;
        } else if file_type.is_file() {
            fs::copy(source, destination)?;
        } else {
            return Err(std::io::Error::other(format!(
                "package asset `{}` must be a regular file or directory",
                source.display()
            )));
        }
    }
    Ok(())
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

    #[test]
    fn nested_package_assets_are_copied_and_deleted() {
        let root = env::temp_dir().join(format!(
            "vescpkg-build-support-nested-{}",
            std::process::id()
        ));
        let manifest = root.join("manifest");
        let source = manifest.join("package/lib/config/defaults.lisp");
        let assets = root.join("assets");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(source.parent().expect("package directory")).expect("create package");
        fs::write(&source, "(define x 1)\n").expect("write package asset");

        copy_package_assets(&manifest, &assets);
        assert_eq!(
            fs::read_to_string(assets.join("lib/config/defaults.lisp"))
                .expect("copied nested asset"),
            "(define x 1)\n"
        );

        fs::remove_file(source).expect("remove nested package asset");
        copy_package_assets(&manifest, &assets);
        assert!(!assets.join("lib/config/defaults.lisp").exists());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
