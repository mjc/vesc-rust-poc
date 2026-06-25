use std::fs;
use std::path::PathBuf;

pub fn command_design_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/cargo-vescpkg-command.md")
}

pub fn command_design_text() -> String {
    fs::read_to_string(command_design_path()).expect("cargo vescpkg command design")
}

#[cfg(test)]
mod tests {
    use super::command_design_text;

    #[test]
    fn command_design_mentions_the_expected_contract() {
        let text = command_design_text();

        for needle in [
            "cargo vescpkg build",
            "cargo vescpkg build --package-only",
            "cargo vescpkg build --target thumbv7em-none-eabihf",
            "crates/vesc-pkg-build",
            "target/vescpkg",
            "nix develop -c make check",
            "package-size guard",
            "symbol checks",
            "xtask",
            "Predictable artifact path",
        ] {
            assert!(
                text.contains(needle),
                "command design document is missing required guidance: {needle}"
            );
        }
    }
}
