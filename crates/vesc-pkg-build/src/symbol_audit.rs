use std::collections::BTreeSet;

pub use crate::native_build::{
    build_final_native_lib_binary, build_final_native_lib_binary_for, build_final_native_lib_elf,
    build_final_native_lib_elf_for, build_rust_staticlib, build_rust_staticlib_for,
    native_lib_bin_path, native_lib_elf_path, package_lib_object_path, rust_staticlib_path,
};
pub use crate::native_inspect::nm_output;

pub fn undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output
        .lines()
        .filter_map(parse_undefined_symbol)
        .collect()
}

pub fn defined_symbols(nm_output: &str) -> BTreeSet<String> {
    nm_output.lines().filter_map(parse_defined_symbol).collect()
}

pub fn unexpected_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_runtime_symbol(symbol))
        .collect()
}

pub fn is_allowed_runtime_symbol(symbol: &str) -> bool {
    symbol.starts_with('_')
        || symbol == "fma"
        || matches!(symbol, "lbm_add_extension" | "lbm_dec_as_i32" | "lbm_enc_i")
}

pub fn audit_rust_staticlib_symbols() -> BTreeSet<String> {
    build_rust_staticlib();
    let output = nm_output(&rust_staticlib_path());

    unexpected_undefined_symbols(&output)
}

pub fn audit_final_native_lib_elf_symbols() -> BTreeSet<String> {
    build_final_native_lib_elf();
    let output = nm_output(&native_lib_elf_path());

    unexpected_final_native_lib_undefined_symbols(&output)
}

pub fn unexpected_final_native_lib_undefined_symbols(nm_output: &str) -> BTreeSet<String> {
    undefined_symbols(nm_output)
        .into_iter()
        .filter(|symbol| !is_allowed_final_native_lib_symbol(symbol))
        .collect()
}

pub fn is_allowed_final_native_lib_symbol(symbol: &str) -> bool {
    is_allowed_runtime_symbol(symbol)
}

fn parse_undefined_symbol(line: &str) -> Option<String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        ["U", symbol] => Some((*symbol).to_owned()),
        [_, "U", symbol] => Some((*symbol).to_owned()),
        _ => None,
    }
}

fn parse_defined_symbol(line: &str) -> Option<String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [_, kind, symbol] if *kind != "U" => Some((*symbol).to_owned()),
        _ => None,
    }
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

    const DEVICE_PROVEN_PACKAGE_BINARY: &str = "THIS_FUCKING_RAN_AT_LEAST.bin.good";
    const DEVICE_PROVEN_INIT_OFFSET: usize = 4;
    const DEVICE_PROVEN_INIT_SIZE: usize = 59;

    fn align_section_vma(vma: usize, alignment: usize) -> usize {
        (vma + alignment - 1) & !(alignment - 1)
    }

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

    fn bounded_init_disassembly(disassembly: &str) -> &str {
        disassembly
            .split("<init>:")
            .nth(1)
            .expect("expected init in disassembly")
            .split("\n\n")
            .next()
            .expect("expected bounded init disassembly")
    }

    fn assert_rust_loader_init_uses_vesc_ffi(init_disassembly: &str) {
        assert!(
            init_disassembly.contains("1000f800"),
            "loader init should load the firmware VESC table base:\n{init_disassembly}"
        );
        assert!(
            init_disassembly.contains("4798")
                || init_disassembly.contains("4790")
                || init_disassembly.contains("4710"),
            "loader init should call lbm_add_extension through an indirect branch after loading slot 0:\n{init_disassembly}"
        );
        assert!(
            init_disassembly.contains("<package_lib_init>"),
            "loader init should delegate package setup to Rust before registering the probe:\n{init_disassembly}"
        );
    }

    include!("native_lib_audit.rs");
}
