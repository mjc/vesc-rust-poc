use std::fs;
use std::path::PathBuf;

pub fn roadmap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/rust-package-api-roadmap.md")
}

pub fn roadmap_text() -> String {
    fs::read_to_string(roadmap_path()).expect("roadmap document contents")
}

#[cfg(test)]
mod tests {
    use super::roadmap_text;

    #[test]
    fn roadmap_captures_the_staged_migration_ladder() {
        let text = roadmap_text();

        for needle in [
            "nix develop -c make check",
            "vesc-rust-poc",
            "vesc-pkg-build",
            "vesc-protocol",
            "vesc-host-cli",
            "Rust pure computation behind C shim",
            "Rust handles primitive logic while C decodes LispBM values",
            "Rust calls one VESC_IF function through the shim",
            "Rust receives a VESC_IF pointer/raw binding",
            "safe wrapper crate",
            "xtask",
            "cargo vescpkg build",
            "Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely",
        ] {
            assert!(
                text.contains(needle),
                "roadmap document is missing required guidance: {needle}"
            );
        }
    }
}
