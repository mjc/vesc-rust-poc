//! Regenerates checked-in golden package fixture payloads and fingerprints.

use std::fs;

use vesc_pkg::native_lib_baseline::fingerprint_bytes;
use vesc_pkg::package_format::build_lisp_data;
use vesc_pkg::package_golden::{
    GOLDEN_FINGERPRINTS_TOML, GOLDEN_LISP_DATA_BIN, GOLDEN_PACKAGE_LIB_BIN,
    build_and_copy_package_lib_bin, golden_fixture_root, package_lib_output_path, repo_root,
};
use vesc_pkg::{BLE_LOOPBACK_PACKAGE_NAME, PackageAssets, PackageLayout, PackageProvenance};

fn main() {
    let root = repo_root();
    let fixture_root = golden_fixture_root();
    fs::create_dir_all(&fixture_root).expect("golden fixture directory");

    let package_lib = build_and_copy_package_lib_bin(&root);
    let package_lib_path = package_lib_output_path(&root);
    let native_bin_path = root.join("target/native-lib-baseline/native_lib.bin");
    let native_elf_path = root.join("target/native-lib-baseline/native_lib.elf");

    fs::copy(&package_lib_path, fixture_root.join(GOLDEN_PACKAGE_LIB_BIN))
        .expect("copy package_lib.bin into golden fixtures");
    fs::copy(&native_bin_path, fixture_root.join("native_lib.bin"))
        .expect("copy native_lib.bin into golden fixtures");
    fs::copy(&native_elf_path, fixture_root.join("native_lib.elf"))
        .expect("copy native_lib.elf into golden fixtures");

    let assets = PackageAssets::new(
        PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, "0.1.0"),
        PackageProvenance::empty(),
    );
    let staging_dir = root.join(assets.staging_dir());
    fs::create_dir_all(staging_dir.join("src")).expect("staging src directory");
    fs::copy(&package_lib_path, staging_dir.join("src/package_lib.bin"))
        .expect("stage package_lib.bin for lisp packing");

    let lisp_data =
        build_lisp_data(&assets.render_loader(), &staging_dir).expect("build lispData bytes");
    fs::write(fixture_root.join(GOLDEN_LISP_DATA_BIN), &lisp_data).expect("write lisp_data.bin");

    let native_lib = fs::read(&native_bin_path).expect("native_lib.bin bytes");
    let fingerprints = format!(
        "package_lib.bin = \"{}\"\n\
         native_lib.bin = \"{}\"\n\
         native_lib.elf = \"{}\"\n\
         lisp_data.bin = \"{}\"\n",
        fingerprint_bytes(&package_lib),
        fingerprint_bytes(&native_lib),
        fingerprint_bytes(
            &fs::read(&native_elf_path).expect("native_lib.elf bytes for fingerprint")
        ),
        fingerprint_bytes(&lisp_data),
    );
    fs::write(fixture_root.join(GOLDEN_FINGERPRINTS_TOML), fingerprints)
        .expect("write fingerprints.toml");

    eprintln!("updated golden fixtures in {}", fixture_root.display());
    eprintln!("  {} ({} bytes)", GOLDEN_PACKAGE_LIB_BIN, package_lib.len());
    eprintln!("  native_lib.bin ({} bytes)", native_lib.len());
    eprintln!(
        "  native_lib.elf ({} bytes)",
        fs::metadata(&native_elf_path)
            .expect("native_lib.elf metadata")
            .len()
    );
    eprintln!("  {} ({} bytes)", GOLDEN_LISP_DATA_BIN, lisp_data.len());
    eprintln!("  {}", GOLDEN_FINGERPRINTS_TOML);
}
