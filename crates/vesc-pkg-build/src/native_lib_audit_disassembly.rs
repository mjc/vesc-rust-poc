#[test]
fn native_lib_disassembly_matches_firmware_patterns() {
    let fixture = SymbolAuditFixture::new();
    fixture.build_bin();

    let semantics = crate::native_elf_semantics::analyze_native_lib_elf(&fixture.elf());
    crate::native_elf_semantics::assert_native_lib_semantics(&fixture.elf());

    insta::assert_snapshot!(
        "native_lib_semantics",
        crate::native_elf_semantics::semantic_report(&semantics)
    );

    if std::env::var("VESC_PKG_DISASM").ok().as_deref() == Some("1") {
        let disassembly = crate::native_disasm::elf_disassembly(&fixture.elf());
        eprintln!("{disassembly}");
    }
}
