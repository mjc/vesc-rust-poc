#[test]
fn native_lib_layout_matches_linker_script() {
    let fixture = SymbolAuditFixture::new();
    fixture.build_bin();

    let blob = fs::read(fixture.bin()).expect("native-lib binary bytes");
    let sections = all_section_layouts(&fixture.elf());

    assert!(
        fixture.bin().exists(),
        "expected the final native-lib binary to be materialized"
    );

    let native_bin_size = fs::metadata(fixture.bin())
        .expect("native-lib binary metadata")
        .len();
    assert!(
        native_bin_size <= 640,
        "expected the native blob to stay compact, got {native_bin_size} bytes"
    );
    assert!(
        native_bin_size <= 512,
        "expected the Rust-only native blob to stay compact, got {native_bin_size} bytes"
    );

    let rust_extension_name = b"ext-rust-probe-diag-v4\0";
    assert!(
        blob.windows(rust_extension_name.len())
            .any(|window| window == rust_extension_name),
        "Rust probe extension identity must be linked into the native blob"
    );

    assert!(
        crate::native_inspect::elf_is_executable(&fixture.elf()),
        "expected a final executable ELF at {:?}",
        fixture.elf()
    );
    assert!(
        crate::native_inspect::elf_has_no_relocations(&fixture.elf()),
        "expected no relocation records in the final native-lib ELF at {:?}",
        fixture.elf()
    );

    for section_name in [".program_ptr", ".init_fun", ".got", ".text"] {
        let section = section_from(&sections, section_name);
        let end = section.vma + section.size;
        assert!(
            end <= blob.len(),
            "section {section_name} at 0x{:x}..0x{:x} exceeds {}-byte blob",
            section.vma,
            end,
            blob.len()
        );
    }

    let init_fun = section_from(&sections, ".init_fun");
    assert_eq!(
        init_fun.vma, DEVICE_PROVEN_INIT_OFFSET,
        "expected .init_fun to start at the device-proven offset"
    );
    assert!(
        init_fun.size >= 24,
        "expected Rust-owned init to retain loader entry and probe registration"
    );
    let proven = crate::native_audit::device_proven_package_binary();
    let proven_init_end = DEVICE_PROVEN_INIT_OFFSET + DEVICE_PROVEN_INIT_SIZE;
    assert_ne!(
        &blob[init_fun.vma..init_fun.vma + init_fun.size.min(DEVICE_PROVEN_INIT_SIZE)],
        &proven[DEVICE_PROVEN_INIT_OFFSET..proven_init_end],
        "Rust-owned init should no longer match the legacy hand-asm bytes in fixtures/device-proven/legacy-init.hex"
    );

    let program_ptr = section_from(&sections, ".program_ptr");
    let got = section_from(&sections, ".got");
    let text = section_from(&sections, ".text");
    assert_eq!(
        *program_ptr,
        SectionLayout {
            name: ".program_ptr".to_owned(),
            size: 4,
            vma: 0,
        }
    );
    assert_eq!(init_fun.vma, program_ptr.vma + program_ptr.size);
    assert_eq!(
        got.vma,
        align_section_vma(init_fun.vma + init_fun.size, 4),
        "expected .got to follow .init_fun with VESC's 4-byte section alignment"
    );
    assert!(
        text.vma >= got.vma + got.size,
        "expected .text to load after .got"
    );
    assert_eq!(text.vma % 16, 0, "expected .text to keep VESC's 16-byte function alignment");
    assert!(
        text.size >= 64,
        "expected .text to retain the probe callback and stop hook"
    );
}
