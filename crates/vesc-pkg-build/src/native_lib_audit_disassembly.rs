#[test]
fn native_lib_disassembly_matches_firmware_patterns() {
    let fixture = SymbolAuditFixture::new();
    fixture.build_bin();

    let disassembly = crate::native_disasm::elf_disassembly(&fixture.elf());

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
        init_disassembly.contains("4620")
            || init_disassembly.contains("movs\tr0, #1")
            || init_disassembly.contains("4710"),
        "loader init should return lbm_add_extension's result after package init and registration:\n{init_disassembly}"
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

    let bounded_init = bounded_init_disassembly(&disassembly);
    assert!(
        !bounded_init.contains("addw\tr1, pc") && !bounded_init.contains("0ff2 2901"),
        "loader init should pass the probe callback through vesc-ffi, not legacy hand-asm PC-relative registration:\n{bounded_init}"
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

    insta::assert_snapshot!("init_disassembly", redact_disassembly_for_snapshot(bounded_init));
    insta::assert_snapshot!(
        "probe_disassembly",
        redact_disassembly_for_snapshot(probe_disassembly)
    );
    insta::assert_snapshot!(
        "stop_disassembly",
        redact_disassembly_for_snapshot(stop_disassembly)
    );
}
