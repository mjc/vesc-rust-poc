#[test]
fn native_lib_artifacts_match_firmware_expectations() {
    let fixture = SymbolAuditFixture::new();
    fixture.build_bin();

    let blob = fs::read(fixture.bin()).expect("native-lib binary bytes");
    let elf_symbols = nm_output(&fixture.elf());
    let staticlib_symbols = nm_output(&fixture.staticlib());
    let staticlib_defined = defined_symbols(&staticlib_symbols);
    let staticlib_undefined = undefined_symbols(&staticlib_symbols);
    let elf_defined = defined_symbols(&elf_symbols);
    let elf_undefined = undefined_symbols(&elf_symbols);
    let sections = all_section_layouts(&fixture.elf());
    let disassembly = command_stdout(
        "arm-none-eabi-objdump",
        [PathBuf::from("-d"), fixture.elf()],
    );
    let readelf = command_stdout(
        "arm-none-eabi-readelf",
        [PathBuf::from("-h"), PathBuf::from("-r"), fixture.elf()],
    );

    assert!(
        unexpected_undefined_symbols(&staticlib_symbols).is_empty(),
        "unexpected undefined symbols remain in the Rust staticlib"
    );
    assert!(
        unexpected_final_native_lib_undefined_symbols(&elf_symbols).is_empty(),
        "unexpected undefined symbols remain in the final native-lib ELF"
    );
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
        readelf.contains("Type:                              EXEC"),
        "expected a final executable ELF, got:\n{readelf}"
    );
    assert!(
        readelf.contains("There are no relocations in this file."),
        "expected no relocation records in the final native-lib ELF, got:\n{readelf}"
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
    let proven = fs::read(crate::test_support::repo_root().join(DEVICE_PROVEN_PACKAGE_BINARY))
        .expect("device-proven package binary bytes");
    let proven_init_end = DEVICE_PROVEN_INIT_OFFSET + DEVICE_PROVEN_INIT_SIZE;
    assert_ne!(
        &blob[init_fun.vma..init_fun.vma + init_fun.size.min(DEVICE_PROVEN_INIT_SIZE)],
        &proven[DEVICE_PROVEN_INIT_OFFSET..proven_init_end],
        "Rust-owned init should no longer match the legacy hand-asm bytes in {DEVICE_PROVEN_PACKAGE_BINARY}"
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

    for offset in ["1000f800", "#596]"] {
        assert!(
            disassembly.contains(offset),
            "expected VESC_IF slot access {offset} in generated code:\n{disassembly}"
        );
    }
    assert!(
        !disassembly.contains("<vesc_send_app_data>")
            && !disassembly.contains("<vesc_set_app_data_handler>"),
        "expected direct VESC_IF calls without C wrapper stubs:\n{disassembly}"
    );
    for symbol in ["<init>", "<package_lib_init>", "<ext_rust_probe_diag_v4>"] {
        assert!(
            disassembly.contains(symbol),
            "expected native image to retain `{symbol}`:\n{disassembly}"
        );
    }
    let init_disassembly = disassembly
        .split("<init>:")
        .nth(1)
        .expect("expected init in disassembly")
        .split("\n\nDisassembly")
        .next()
        .expect("expected bounded init disassembly");
    assert!(
        init_disassembly.contains("<package_lib_init>"),
        "loader init should run Rust package init before registering the probe:\n{init_disassembly}"
    );
    assert!(
        init_disassembly.contains("1000f800")
            && (init_disassembly.contains("4798")
                || init_disassembly.contains("4790")
                || init_disassembly.contains("4710")),
        "Rust loader init should register the probe inline through VESC_IF like refloat:\n{init_disassembly}"
    );
    assert!(
        !disassembly.contains("<register_package_extensions_asm>"),
        "Rust init should register directly without a registration trampoline:\n{disassembly}"
    );

    let probe_start = disassembly
        .find("<ext_rust_probe_diag_v4>:")
        .expect("expected ext_rust_probe_diag_v4 in final native image");
    let probe_rest = &disassembly[probe_start..];
    let probe_end = probe_rest.find("\n\n0000").unwrap_or(probe_rest.len());
    let probe_disassembly = &probe_rest[..probe_end];
    assert!(
        probe_disassembly.contains("#680") || probe_disassembly.contains("0x2a8"),
        "Rust probe extension should return the LispBM-encoded integer 42 directly:\n{probe_disassembly}"
    );
    assert!(
        !probe_disassembly.contains("1000f800"),
        "Rust probe extension should not reject valid hardware calls through fragile LispBM validation slots:\n{probe_disassembly}"
    );

    assert_rust_loader_init_uses_vesc_ffi(bounded_init_disassembly(&disassembly));
    assert!(
        init_disassembly.contains("4620")
            || init_disassembly.contains("movs\tr0, #1")
            || init_disassembly.contains("4710"),
        "loader init should return lbm_add_extension's result after package init and registration:\n{init_disassembly}"
    );
    assert!(
        disassembly
            .split("<init>:")
            .nth(1)
            .and_then(|init| init.split("\n\nDisassembly").next())
            .is_some_and(|init| init.contains("1000f800") && init.contains("<package_lib_init>")),
        "loader init should call Rust package init and register the probe inline:\n{disassembly}"
    );

    let stop_start = disassembly
        .find("stop_package")
        .expect("expected stop_package in final native image");
    let stop_disassembly = disassembly[stop_start..]
        .split("\n\n")
        .next()
        .expect("expected bounded stop_package disassembly");
    assert!(
        stop_disassembly.contains("1000f800") && stop_disassembly.contains("#596]"),
        "stop_package should clear app-data through direct VESC_IF + 596 load like refloat:\n{stop_disassembly}"
    );
    assert!(
        !stop_disassembly.contains("cbz"),
        "stop_package should not guard the VESC_IF app-data slot; refloat calls it directly:\n{stop_disassembly}"
    );

    let bounded_init = bounded_init_disassembly(&disassembly);
    assert!(
        !bounded_init.contains("addw\tr1, pc") && !bounded_init.contains("0ff2 2901"),
        "loader init should pass the probe callback through vesc-ffi, not legacy hand-asm PC-relative registration:\n{bounded_init}"
    );
    let short_init = disassembly
        .split("<init>:")
        .nth(1)
        .expect("expected init in disassembly")
        .split("\n\n")
        .next()
        .expect("expected bounded init disassembly");
    assert!(
        short_init.contains("<package_lib_init>"),
        "loader init should run Rust package init:\n{short_init}"
    );
    assert!(
        short_init.contains("1000f800")
            && (short_init.contains("4798")
                || short_init.contains("4790")
                || short_init.contains("4710")),
        "loader init should register the LispBM probe inline:\n{short_init}"
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
