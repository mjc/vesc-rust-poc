use vesc_pkg::native_lib_audit::semantic_snapshot_report;
use vesc_pkg::{ensure_repo_native_lib_artifacts, native_lib_link_plan};

#[test]
fn native_lib_semantics() {
    let plan = native_lib_link_plan();
    ensure_repo_native_lib_artifacts(plan.root());
    let report = semantic_snapshot_report(&plan.elf_path());
    insta::assert_snapshot!("native_lib_semantics", report);
}
