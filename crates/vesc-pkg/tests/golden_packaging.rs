use tempfile::TempDir;

use vesc_pkg::golden::{
    FINGERPRINTS_TOML, LISP_DATA, PACKAGE_LIB, pack_lisp_data, payload_contains_probe_extension,
};
use vesc_pkg::native_lib_baseline::fingerprint_bytes;
use vesc_pkg::package_format_decode::{
    parse_lisp_imports, payload_matches_native_with_only_nul_tail,
};

fn lisp_import_summary(lisp_data: &[u8]) -> String {
    let (code, imports) = parse_lisp_imports(lisp_data);
    let mut lines = vec![
        format!("code_len={}", code.len()),
        format!("imports={}", imports.len()),
    ];
    for (index, import) in imports.iter().enumerate() {
        lines.push(format!(
            "import[{index}]: tag={} offset={} size={} payload_fp={}",
            import.tag,
            import.offset,
            import.size,
            fingerprint_bytes(&import.payload)
        ));
    }
    lines.join("\n")
}

#[test]
fn golden_fingerprints() {
    insta::assert_snapshot!("golden_fingerprints", FINGERPRINTS_TOML);
}

#[test]
fn package_lib_fixture_metadata() {
    insta::assert_snapshot!(
        "package_lib_fixture",
        format!(
            "len={}\nfingerprint={}\nprobe_extension={}",
            PACKAGE_LIB.len(),
            fingerprint_bytes(PACKAGE_LIB),
            payload_contains_probe_extension(PACKAGE_LIB)
        )
    );
}

#[test]
fn lisp_data_fixture_metadata() {
    insta::assert_snapshot!(
        "lisp_data_fixture",
        format!(
            "len={}\nfingerprint={}",
            LISP_DATA.len(),
            fingerprint_bytes(LISP_DATA)
        )
    );
}

#[test]
fn lisp_data_matches_fixture() {
    let workspace = TempDir::new().expect("temp workspace");
    let packed = pack_lisp_data(PACKAGE_LIB, workspace.path()).expect("pack lispData");
    insta::assert_snapshot!("packed_lisp_data_fingerprint", fingerprint_bytes(&packed));
    assert_eq!(
        packed, LISP_DATA,
        "packed lispData must match fixture bytes"
    );
}

#[test]
fn lisp_data_embeds_fixture_native_import() {
    let workspace = TempDir::new().expect("temp workspace");
    let lisp_data = pack_lisp_data(PACKAGE_LIB, workspace.path()).expect("pack lispData");
    insta::assert_snapshot!("lisp_import_summary", lisp_import_summary(&lisp_data));
    let (_, imports) = parse_lisp_imports(&lisp_data);

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].tag, "package-lib");
    assert!(payload_matches_native_with_only_nul_tail(
        &imports[0].payload,
        PACKAGE_LIB
    ));
}

#[test]
fn lisp_packing_is_idempotent() {
    let workspace = TempDir::new().expect("temp workspace");
    let first = pack_lisp_data(PACKAGE_LIB, workspace.path()).expect("first lispData");
    let second = pack_lisp_data(PACKAGE_LIB, workspace.path()).expect("second lispData");
    assert_eq!(first, second, "lisp packing must be deterministic");
}
