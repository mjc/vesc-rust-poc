pub use crate::native_audit::{
    DEVICE_PROVEN_INIT_OFFSET, DEVICE_PROVEN_INIT_SIZE, align_section_vma, defined_symbols,
    device_proven_package_binary, is_allowed_final_native_lib_symbol, is_allowed_runtime_symbol,
    undefined_symbols, unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
};
pub use crate::native_build::{
    build_final_native_lib_binary, build_final_native_lib_binary_for, build_final_native_lib_elf,
    build_final_native_lib_elf_for, build_rust_staticlib, build_rust_staticlib_for,
    native_lib_bin_path, native_lib_elf_path, package_lib_object_path, rust_staticlib_path,
};
pub use crate::native_inspect::nm_output;
pub use crate::native_lib_audit::{
    NativeLibArtifactPaths, audit_native_lib_artifacts, audit_native_lib_flat_binary,
    audit_native_lib_layout, audit_native_lib_symbols, semantic_snapshot_report,
};

/// Returns the symbol set exported by the built Rust static library.
pub fn audit_rust_staticlib_symbols() -> std::collections::BTreeSet<String> {
    build_rust_staticlib();
    let output = nm_output(&rust_staticlib_path());
    unexpected_undefined_symbols(&output)
}

/// Returns the symbol set exported by the final linked native ELF.
pub fn audit_final_native_lib_elf_symbols() -> std::collections::BTreeSet<String> {
    build_final_native_lib_elf();
    let output = nm_output(&native_lib_elf_path());
    unexpected_final_native_lib_undefined_symbols(&output)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        defined_symbols, is_allowed_final_native_lib_symbol, is_allowed_runtime_symbol,
        undefined_symbols, unexpected_final_native_lib_undefined_symbols,
        unexpected_undefined_symbols,
    };

    #[test]
    fn symbol_audit_helpers_classify_nm_output() {
        let sample = "\
00000000 T rust_add
         U __aeabi_dadd
         U fma
         U lbm_add_extension
";

        assert_eq!(
            defined_symbols(sample),
            BTreeSet::from(["rust_add".to_owned()])
        );
        assert_eq!(
            undefined_symbols(sample),
            BTreeSet::from([
                "__aeabi_dadd".to_owned(),
                "fma".to_owned(),
                "lbm_add_extension".to_owned(),
            ])
        );

        assert!(is_allowed_runtime_symbol("__aeabi_dadd"));
        assert!(is_allowed_runtime_symbol(
            "_RNvNtNtCseGTyb2smT0B_17compiler_builtins3mem6memcpy"
        ));
        assert!(is_allowed_runtime_symbol("fma"));
        assert!(is_allowed_runtime_symbol("lbm_add_extension"));
        assert!(is_allowed_runtime_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_runtime_symbol("lbm_enc_i"));

        let unexpected_sample = "\
         U __aeabi_dadd
         U fma
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
         U plain_external
";

        assert_eq!(
            unexpected_undefined_symbols(unexpected_sample),
            BTreeSet::from(["plain_external".to_owned()])
        );

        assert!(is_allowed_final_native_lib_symbol("lbm_add_extension"));
        assert!(is_allowed_final_native_lib_symbol("lbm_dec_as_i32"));
        assert!(is_allowed_final_native_lib_symbol("lbm_enc_i"));
        assert!(!is_allowed_final_native_lib_symbol("rust_add"));

        let final_sample = "\
         U lbm_add_extension
         U lbm_dec_as_i32
         U lbm_enc_i
";

        assert_eq!(
            unexpected_final_native_lib_undefined_symbols(final_sample),
            BTreeSet::new()
        );
    }
}
