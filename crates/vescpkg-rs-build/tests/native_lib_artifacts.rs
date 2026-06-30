//! Native library artifact audit integration tests.

use std::path::PathBuf;

use tempfile::TempDir;
use vescpkg_rs_build::{
    NATIVE_LIB_BIN, NATIVE_LIB_ELF, assert_native_lib_semantics, ensure_repo_native_lib_artifacts,
    native_lib_link_plan, semantic_snapshot_report,
};

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

#[test]
fn current_native_lib_preserves_known_good_loader_contract() {
    let plan = native_lib_link_plan();
    ensure_repo_native_lib_artifacts(plan.root());
    assert_native_lib_semantics(&plan.elf_path());
}
