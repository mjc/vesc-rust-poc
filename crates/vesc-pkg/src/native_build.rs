use std::path::{Path, PathBuf};

use crate::BLE_LOOPBACK_PACKAGE_NAME;
use crate::cargo_vescpkg_command::DEFAULT_PACKAGE_VERSION;
use crate::native_lib_link::{NativeLibLinkPlan, native_lib_link_plan};
use crate::package_conversion::PackageBinaryConversionPlan;
use crate::package_runner::ensure_native_lib_artifacts;

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

fn repo_conversion_plan(plan: &NativeLibLinkPlan) -> PackageBinaryConversionPlan {
    PackageBinaryConversionPlan::new(
        plan.root(),
        BLE_LOOPBACK_PACKAGE_NAME,
        DEFAULT_PACKAGE_VERSION,
    )
}

fn is_repo_native_build_plan(plan: &NativeLibLinkPlan) -> bool {
    plan.root() == native_lib_link_plan().root()
}

fn ensure_repo_plan_artifacts(plan: &NativeLibLinkPlan) {
    ensure_native_lib_artifacts(&repo_conversion_plan(plan));
}

pub fn build_rust_staticlib() {
    build_rust_staticlib_for(&native_lib_link_plan());
}

pub fn build_rust_staticlib_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_plan_artifacts(plan);
    } else {
        crate::native_lib_materialize::build_rust_staticlib(plan);
    }
}

pub fn build_final_native_lib_elf() {
    build_final_native_lib_elf_for(&native_lib_link_plan());
}

pub fn build_final_native_lib_elf_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_plan_artifacts(plan);
    } else {
        crate::native_lib_materialize::build_final_native_lib_elf(plan);
    }
}

pub fn build_final_native_lib_binary(native_binary_path: &Path) {
    let plan = crate::native_lib_link::native_lib_link_plan_for_native_binary(native_binary_path);
    build_final_native_lib_binary_for(&plan, native_binary_path);
}

pub fn build_final_native_lib_binary_for(plan: &NativeLibLinkPlan, native_binary_path: &Path) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_plan_artifacts(plan);
    } else {
        crate::native_lib_materialize::materialize_native_lib_binary(plan, native_binary_path);
    }
}
