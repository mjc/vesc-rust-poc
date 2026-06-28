use std::path::PathBuf;

use tempfile::TempDir;
use vesc_pkg::golden::{NATIVE_LIB_BIN, NATIVE_LIB_ELF};
use vesc_pkg::native_lib_audit::semantic_snapshot_report;

fn write_fixture_artifacts(dir: &std::path::Path) -> PathBuf {
    let elf = dir.join("native_lib.elf");
    std::fs::write(&elf, NATIVE_LIB_ELF).expect("write fixture elf");
    std::fs::write(dir.join("native_lib.bin"), NATIVE_LIB_BIN).expect("write fixture bin");
    elf
}

#[test]
fn native_lib_semantics() {
    let workspace = TempDir::new().expect("temp workspace");
    let elf = write_fixture_artifacts(workspace.path());
    let report = semantic_snapshot_report(&elf);
    insta::assert_snapshot!("native_lib_semantics", report);
}
