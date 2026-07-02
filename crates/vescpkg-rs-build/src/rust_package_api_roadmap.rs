use std::fs;
use std::path::PathBuf;

/// Returns the checked-in roadmap document path for the Rust package API.
pub fn roadmap_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/rust-package-api-roadmap.md")
}

/// Reads the checked-in roadmap document.
pub fn roadmap_text() -> String {
    fs::read_to_string(roadmap_path()).expect("roadmap document contents")
}

/// Returns the body of a second-level markdown section named `heading`.
pub fn markdown_section_body<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let target = format!("## {heading}");
    let mut body_start = None;
    let mut line_start = 0;

    for line in text.split_inclusive('\n') {
        let heading_line = line.trim_end_matches(['\r', '\n']).trim_end();
        if let Some(start) = body_start {
            if heading_line.starts_with("## ") {
                return Some(text[start..line_start].trim());
            }
        } else if heading_line == target {
            body_start = Some(line_start + line.len());
        }
        line_start += line.len();
    }

    body_start.map(|start| text[start..].trim())
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
        assert!(guardrail.contains("`vesc`, `vesc-api`, or `vesc-comm`"));

        let api_surface =
            markdown_section_body(&text, "Package-Author API Surface").expect("api surface");
        assert!(api_surface.contains("vescpkg_rs::prelude::*"));
        assert!(api_surface.contains("AppDataHandlerRegistrationError"));

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

    #[test]
    fn markdown_section_body_matches_exact_second_level_headings() {
        let text = "# Doc\n\n## Contractual\nwrong\n\n## Contract\nright\n\n## Next\ndone\n";

        assert_eq!(markdown_section_body(text, "Contract"), Some("right"));
        assert_eq!(markdown_section_body(text, "Missing"), None);
    }
}
