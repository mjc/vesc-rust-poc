use std::path::{Path, PathBuf};

use crate::native_lib_link::{native_lib_link_plan, NativeLibLinkPlan};
use crate::native_lib_materialize::{
    build_final_native_lib_elf_unlocked, build_rust_staticlib_unlocked,
    materialize_native_lib_binary_unlocked,
};
use crate::package_runner::ensure_repo_native_lib_artifacts;

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

fn is_repo_native_build_plan(plan: &NativeLibLinkPlan) -> bool {
    plan.root() == native_lib_link_plan().root()
}

pub fn build_rust_staticlib() {
    build_rust_staticlib_for(&native_lib_link_plan());
}

pub fn build_rust_staticlib_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_native_lib_artifacts(plan.root());
    } else {
        build_rust_staticlib_unlocked(plan);
    }
}

pub fn build_final_native_lib_elf() {
    build_final_native_lib_elf_for(&native_lib_link_plan());
}

pub fn build_final_native_lib_elf_for(plan: &NativeLibLinkPlan) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_native_lib_artifacts(plan.root());
    } else {
        build_final_native_lib_elf_unlocked(plan);
    }
}

pub fn build_final_native_lib_binary(native_binary_path: &Path) {
    let plan = crate::native_lib_link::native_lib_link_plan_for_native_binary(native_binary_path);
    build_final_native_lib_binary_for(&plan, native_binary_path);
}

pub fn build_final_native_lib_binary_for(plan: &NativeLibLinkPlan, native_binary_path: &Path) {
    if is_repo_native_build_plan(plan) {
        ensure_repo_native_lib_artifacts(plan.root());
    } else {
        materialize_native_lib_binary_unlocked(plan, native_binary_path);
    }
}
