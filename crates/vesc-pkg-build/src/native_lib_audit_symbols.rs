#[test]
fn native_lib_symbols_are_fully_resolved() {
    let fixture = SymbolAuditFixture::new();
    fixture.build_bin();

    let elf_symbols = nm_output(&fixture.elf());
    let staticlib_symbols = nm_output(&fixture.staticlib());
    let staticlib_defined = defined_symbols(&staticlib_symbols);
    let staticlib_undefined = undefined_symbols(&staticlib_symbols);
    let elf_defined = defined_symbols(&elf_symbols);
    let elf_undefined = undefined_symbols(&elf_symbols);

    assert!(
        unexpected_undefined_symbols(&staticlib_symbols).is_empty(),
        "unexpected undefined symbols remain in the Rust staticlib"
    );
    assert!(
        unexpected_final_native_lib_undefined_symbols(&elf_symbols).is_empty(),
        "unexpected undefined symbols remain in the final native-lib ELF"
    );
    assert!(
        fixture.package_object().exists(),
        "native build must materialize the C loader shim at {:?}",
        fixture.package_object()
    );
    assert!(
        elf_defined.contains("ext_rust_probe_diag_v4"),
        "final image must retain the Rust LispBM probe callback:\n{elf_symbols}"
    );
    assert!(
        elf_defined.contains("init")
            && elf_defined.contains("prog_ptr")
            && elf_defined.contains("package_lib_init"),
        "native image must keep loader entry and Rust package init:\n{elf_symbols}"
    );
    assert!(
        !elf_defined.contains("ext_c_probe_v12"),
        "expected final native image to drop the C LispBM probe body:\n{elf_symbols}"
    );
    assert!(
        !elf_defined.contains("ext_c_probe_v6"),
        "expected final native image to drop the temporary C probe after Rust-owned registration:\n{elf_symbols}"
    );
    assert!(
        elf_undefined.is_empty(),
        "expected final native image to resolve the C-to-Rust boundary completely:\n{elf_symbols}"
    );
    assert!(
        staticlib_defined.contains("package_lib_init"),
        "expected the Rust staticlib to export package_lib_init"
    );
    assert!(
        staticlib_defined.contains("init") && staticlib_defined.contains("prog_ptr"),
        "expected the Rust staticlib to export loader entry symbols"
    );
    for symbol in [
        "package_lib_init",
        "ext_rust_probe_diag_v4",
        "init",
        "prog_ptr",
    ] {
        assert!(
            staticlib_defined.contains(symbol),
            "Rust staticlib must own symbol `{symbol}`:\n{staticlib_symbols}"
        );
    }
    assert!(
        !staticlib_defined.contains("rust_add"),
        "rust_add must stay test-only and out of the firmware staticlib:\n{staticlib_symbols}"
    );
    assert!(
        !staticlib_undefined.contains("register_c_probe"),
        "Rust package init should not depend on a separate C probe registrar:\n{staticlib_symbols}"
    );
}
