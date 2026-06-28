use std::fs;
use std::path::PathBuf;

pub fn roadmap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/rust-package-api-roadmap.md")
}

pub fn roadmap_text() -> String {
    fs::read_to_string(roadmap_path()).expect("roadmap document contents")
}

#[cfg(test)]
mod tests {
    use super::roadmap_text;

    #[test]
    fn roadmap_captures_the_current_rust_boundary_and_next_migration_ladder() {
        let text = roadmap_text();

        for needle in [
            "nix develop -c make check",
            "nix develop -c make symbol-check",
            "nix develop -c make check-full",
            "fast host tier",
            "embedded native-lib audit tier",
            "nix develop -c make package",
            "vesc-ble-loopback",
            "vesc-pkg-build",
            "vesc-protocol",
            "vesc-host-cli",
            "Rust exports `prog_ptr` and `init`",
            "Rust owns LispBM extension table registration",
            "Rust owns BLE app-data and stop-hook lifecycle setup",
            "generic VESC linker and conversion references",
            "Hardware-validate install, `lisp-probe`, and `loopback`",
            "safe wrapper crate",
            "cargo vescpkg build",
            "no_std",
            "no-alloc",
            "Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely",
        ] {
            assert!(
                text.contains(needle),
                "roadmap document is missing required guidance: {needle}"
            );
        }
    }
}
