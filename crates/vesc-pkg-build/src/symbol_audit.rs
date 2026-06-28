pub use crate::native_audit::{
    align_section_vma, assert_rust_loader_init_uses_vesc_ffi, bounded_init_disassembly,
    defined_symbols, is_allowed_final_native_lib_symbol, is_allowed_runtime_symbol,
    undefined_symbols, unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    DEVICE_PROVEN_INIT_OFFSET, DEVICE_PROVEN_INIT_SIZE, DEVICE_PROVEN_PACKAGE_BINARY,
};
pub use crate::native_build::{
    build_final_native_lib_binary, build_final_native_lib_binary_for, build_final_native_lib_elf,
    build_final_native_lib_elf_for, build_rust_staticlib, build_rust_staticlib_for,
    native_lib_bin_path, native_lib_elf_path, package_lib_object_path, rust_staticlib_path,
};
pub use crate::native_inspect::nm_output;

pub fn audit_rust_staticlib_symbols() -> std::collections::BTreeSet<String> {
    build_rust_staticlib();
    let output = nm_output(&rust_staticlib_path());
    unexpected_undefined_symbols(&output)
}

pub fn audit_final_native_lib_elf_symbols() -> std::collections::BTreeSet<String> {
    build_final_native_lib_elf();
    let output = nm_output(&native_lib_elf_path());
    unexpected_final_native_lib_undefined_symbols(&output)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{
        build_final_native_lib_binary_for, defined_symbols, is_allowed_final_native_lib_symbol,
        is_allowed_runtime_symbol, undefined_symbols,
        unexpected_final_native_lib_undefined_symbols, unexpected_undefined_symbols,
    };
    use crate::native_inspect::{
        all_section_layouts, command_stdout, nm_output, section_from, SectionLayout,
    };
    use crate::native_lib_link::NativeLibLinkPlan;
    use crate::test_support::NativeBuildWorkspace;
    use std::collections::BTreeSet;

    struct SymbolAuditFixture {
        workspace: &'static NativeBuildWorkspace,
    }

    impl SymbolAuditFixture {
        fn new() -> Self {
            Self {
                workspace: NativeBuildWorkspace::shared(),
            }
        }

        fn plan(&self) -> &NativeLibLinkPlan {
            self.workspace.plan()
        }

        fn elf(&self) -> PathBuf {
            self.workspace.native_lib_elf_path()
        }

        fn bin(&self) -> PathBuf {
            self.workspace.native_lib_bin_path()
        }

        fn staticlib(&self) -> PathBuf {
            self.workspace.rust_staticlib_path()
        }

        fn package_object(&self) -> PathBuf {
            self.workspace.package_lib_object_path()
        }

        fn build_bin(&self) {
            build_final_native_lib_binary_for(self.plan(), &self.bin());
        }
    }

    use super::{
        align_section_vma, assert_rust_loader_init_uses_vesc_ffi, bounded_init_disassembly,
        DEVICE_PROVEN_INIT_OFFSET, DEVICE_PROVEN_INIT_SIZE, DEVICE_PROVEN_PACKAGE_BINARY,
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

    include!("native_lib_audit_symbols.rs");
    include!("native_lib_audit_layout.rs");
    include!("native_lib_audit_disassembly.rs");
}
