use std::fs;
use std::path::PathBuf;

pub fn roadmap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/rust-package-api-roadmap.md")
}

pub fn roadmap_text() -> String {
    fs::read_to_string(roadmap_path()).expect("roadmap document contents")
}

pub fn markdown_section_body<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("## {heading}");
    let after_heading = text.split_once(&marker)?.1;
    let body = after_heading
        .split_once("\n## ")
        .map_or(after_heading, |(body, _)| body);
    Some(body.trim())
}

#[cfg(test)]
mod tests {
    use super::{markdown_section_body, roadmap_text};

    fn assert_markdown_sections(text: &str, sections: &[&str]) {
        for section in sections {
            assert!(
                text.contains(section),
                "roadmap document is missing required section: {section}"
            );
        }
    }

    #[test]
    fn roadmap_captures_the_current_rust_boundary_and_next_migration_ladder() {
        let text = roadmap_text();

        assert_markdown_sections(
            &text,
            &[
                "## Current workspace shape",
                "## Validation",
                "## Current Rust-Owned Boundary",
                "## Next Migration Ladder",
                "## Guardrail",
            ],
        );

        let guardrail = markdown_section_body(&text, "Guardrail").expect("guardrail section");
        assert!(guardrail.contains("no_std"));
        assert!(guardrail.contains("no-alloc"));

        let validation = markdown_section_body(&text, "Validation").expect("validation section");
        for command in [
            "nix develop -c make check",
            "nix develop -c make check-full",
        ] {
            assert!(
                validation.contains(command),
                "validation section is missing command: {command}"
            );
        }
    }
}
