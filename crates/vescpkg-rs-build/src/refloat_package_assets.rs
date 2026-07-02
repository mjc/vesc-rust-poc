use std::path::{Path, PathBuf};

use crate::{Package, PackageError};

/// Fixed build metadata used when rendering Refloat's generated package assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatBuildInfo {
    build_date: String,
    git_commit: String,
}

impl RefloatBuildInfo {
    /// Create build metadata matching Refloat's Makefile-generated README fields.
    pub fn new(build_date: impl Into<String>, git_commit: impl Into<String>) -> Self {
        Self {
            build_date: build_date.into(),
            git_commit: git_commit.into(),
        }
    }
}

/// Refloat source-tree asset generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatSourceAssets {
    source_root: PathBuf,
}

impl RefloatSourceAssets {
    /// Create a generator rooted at a Refloat source checkout.
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self {
            source_root: source_root.into(),
        }
    }

    /// Render and write Refloat's generated package README and UI files.
    pub fn materialize_generated_inputs(
        &self,
        build_info: &RefloatBuildInfo,
    ) -> Result<RefloatGeneratedAssets, PackageError> {
        let generated = RefloatGeneratedAssets::new(&self.source_root);
        std::fs::write(generated.readme_path(), self.render_readme(build_info)?)?;
        std::fs::write(generated.ui_path(), self.render_ui()?)?;
        Ok(generated)
    }

    /// Materialize generated inputs and write the Refloat `.vescpkg` artifact.
    pub fn write_package(&self, build_info: &RefloatBuildInfo) -> Result<PathBuf, PackageError> {
        self.materialize_generated_inputs(build_info)?;
        Package::write_from_manifest(self.source_root.join("pkgdesc.qml"))
    }

    fn render_readme(&self, build_info: &RefloatBuildInfo) -> Result<String, PackageError> {
        let readme = self.read_text("package_README.md")?;
        let version = self.read_trimmed("version")?;

        Ok(format!(
            "{readme}\n### Build Info\n- Version: {version}\n- Build Date: {}\n- Git Commit: #{}\n",
            build_info.build_date, build_info.git_commit
        ))
    }

    fn render_ui(&self) -> Result<String, PackageError> {
        let template = self.read_text("ui.qml.in")?;
        let package_name = truncate_chars(&self.read_trimmed("package_name")?, 20);
        let version = self.read_trimmed("version")?;

        let rendered = template
            .replace("{{PACKAGE_NAME}}", &package_name)
            .replace("{{VERSION}}", &version);
        Ok(minify_qml(&rendered))
    }

    fn read_text(&self, relative_path: &str) -> Result<String, PackageError> {
        std::fs::read_to_string(self.source_root.join(relative_path)).map_err(Into::into)
    }

    fn read_trimmed(&self, relative_path: &str) -> Result<String, PackageError> {
        Ok(self.read_text(relative_path)?.trim().to_owned())
    }
}

/// Paths written by Refloat source asset generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatGeneratedAssets {
    source_root: PathBuf,
}

impl RefloatGeneratedAssets {
    fn new(source_root: impl AsRef<Path>) -> Self {
        Self {
            source_root: source_root.as_ref().to_path_buf(),
        }
    }

    /// Return the generated package README path.
    pub fn readme_path(&self) -> PathBuf {
        self.source_root.join("package_README-gen.md")
    }

    /// Return the generated QML UI path.
    pub fn ui_path(&self) -> PathBuf {
        self.source_root.join("ui.qml")
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn minify_qml(input: &str) -> String {
    let lines: Vec<_> = input.lines().filter_map(minify_qml_line).collect();

    lines
        .iter()
        .enumerate()
        .fold(String::new(), |mut output, (index, line)| {
            output.push_str(line);
            if should_keep_linebreak(line, lines.get(index + 1).map(String::as_str)) {
                output.push('\n');
            }
            output
        })
}

fn minify_qml_line(line: &str) -> Option<String> {
    let line = strip_line_comment(line).trim().to_owned();
    (!line.is_empty()).then(|| compact_qml_spaces(&line))
}

fn strip_line_comment(line: &str) -> String {
    let mut output = String::new();
    let mut chars = line.chars().peekable();
    let mut string_delimiter = None;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        match (string_delimiter, escaped, ch, chars.peek().copied()) {
            (None, _, '/', Some('/')) => break,
            (None, _, '"' | '\'', _) => {
                string_delimiter = Some(ch);
                output.push(ch);
            }
            (Some(delimiter), false, current, _) if current == delimiter => {
                string_delimiter = None;
                output.push(ch);
            }
            (Some(_), false, '\\', _) => {
                escaped = true;
                output.push(ch);
            }
            (Some(_), true, _, _) => {
                escaped = false;
                output.push(ch);
            }
            _ => output.push(ch),
        }
    }

    output
}

fn compact_qml_spaces(line: &str) -> String {
    line.chars()
        .fold(
            (String::new(), false, None, false),
            |(mut output, pending_space, string_delimiter, escaped), ch| {
                let punctuation = is_qml_punctuation(ch);
                match (string_delimiter, escaped, ch) {
                    (Some(delimiter), false, current) if current == delimiter => {
                        output.push(ch);
                        (output, false, None, false)
                    }
                    (Some(delimiter), false, '\\') => {
                        output.push(ch);
                        (output, false, Some(delimiter), true)
                    }
                    (Some(delimiter), _, _) => {
                        output.push(ch);
                        (output, false, Some(delimiter), false)
                    }
                    (None, _, '"' | '\'') => {
                        if pending_space
                            && !output.ends_with("return")
                            && output
                                .chars()
                                .last()
                                .is_some_and(|previous| !is_qml_punctuation(previous))
                        {
                            output.push(' ');
                        }
                        output.push(ch);
                        (output, false, Some(ch), false)
                    }
                    (None, _, current) if current.is_whitespace() => (output, true, None, false),
                    (None, _, current) if punctuation => {
                        if output.ends_with(' ') {
                            output.pop();
                        }
                        output.push(current);
                        (output, false, None, false)
                    }
                    (None, _, current) => {
                        if pending_space
                            && output
                                .chars()
                                .last()
                                .is_some_and(|previous| !is_qml_punctuation(previous))
                        {
                            output.push(' ');
                        }
                        output.push(current);
                        (output, false, None, false)
                    }
                }
            },
        )
        .0
}

fn is_qml_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '{' | '}'
            | '('
            | ')'
            | '['
            | ']'
            | ':'
            | ';'
            | ','
            | '='
            | '!'
            | '<'
            | '>'
            | '+'
            | '-'
            | '*'
            | '/'
            | '?'
    )
}

fn should_keep_linebreak(current: &str, next: Option<&str>) -> bool {
    current.starts_with("import ")
        || current.starts_with("id:")
        || current.starts_with("property ") && !matches!(next, Some("}"))
        || current == "}"
            && next.is_some_and(|next| {
                next.starts_with("return ")
                    || next.starts_with("property ")
                    || next.starts_with("function ")
            })
}

#[cfg(test)]
mod tests {
    use super::{RefloatBuildInfo, RefloatSourceAssets};
    use crate::Package;
    use crate::package_wire::parse_lisp_imports;
    use crate::test_support::PackageTestHarness;

    #[test]
    fn materializes_refloat_makefile_generated_readme_and_ui() {
        let harness = PackageTestHarness::new()
            .write_text(
                "package_README.md",
                "# Refloat\n\nGenerated package documentation.\n",
            )
            .write_text("package_name", "Refloat Long Package Name\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "Item {\n    property string title: \"{{PACKAGE_NAME}}\"\n    property string version: \"{{VERSION}}\"\n}\n",
            );
        let root = harness.root();

        let generated = RefloatSourceAssets::new(root)
            .materialize_generated_inputs(&RefloatBuildInfo::new(
                "2026-07-02 06:00:00-06:00",
                "0ef6e99",
            ))
            .expect("generated inputs");

        assert_eq!(generated.readme_path(), root.join("package_README-gen.md"));
        assert_eq!(generated.ui_path(), root.join("ui.qml"));
        assert_eq!(
            std::fs::read_to_string(generated.readme_path()).expect("generated readme"),
            "# Refloat\n\nGenerated package documentation.\n\n### Build Info\n- Version: 1.2.1\n- Build Date: 2026-07-02 06:00:00-06:00\n- Git Commit: #0ef6e99\n"
        );
        assert_eq!(
            std::fs::read_to_string(generated.ui_path()).expect("generated ui"),
            "Item{property string title:\"Refloat Long Package\"\nproperty string version:\"1.2.1\"}"
        );
    }

    #[test]
    fn materializes_refloat_qml_with_makefile_default_minification() {
        let harness = PackageTestHarness::new()
            .write_text("package_README.md", "# Refloat\n")
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "import QtQuick 2.15\n\nItem {\n    id: mainItem\n    // generated title\n    property string title: \"{{PACKAGE_NAME}} {{VERSION}}\"\n    function round(num) {\n        if (num != num) {\n            return \"--\";\n        }\n        return Math.round(num);\n    }\n}\n",
            );
        let root = harness.root();

        let generated = RefloatSourceAssets::new(root)
            .materialize_generated_inputs(&RefloatBuildInfo::new(
                "2026-07-02 06:00:00-06:00",
                "0ef6e99",
            ))
            .expect("generated inputs");

        assert_eq!(
            std::fs::read_to_string(generated.ui_path()).expect("generated ui"),
            "import QtQuick 2.15\nItem{id:mainItem\nproperty string title:\"Refloat 1.2.1\"\nfunction round(num){if(num!=num){return\"--\";}\nreturn Math.round(num);}}"
        );
    }

    #[test]
    fn writes_refloat_package_from_generated_assets_and_existing_native_payload() {
        let harness = PackageTestHarness::new()
            .write_text("package_README.md", "# Refloat\n")
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "ui.qml.in",
                "Item { property string title: \"{{PACKAGE_NAME}} {{VERSION}}\" }\n",
            )
            .write_text(
                "pkgdesc.qml",
                "import QtQuick 2.15\n\nItem {\n    property string pkgName: \"Refloat\"\n    property string pkgDescriptionMd: \"package_README-gen.md\"\n    property string pkgLisp: \"lisp/package.lisp\"\n    property string pkgQml: \"ui.qml\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"refloat.vescpkg\"\n}\n",
            )
            .write_text(
                "lisp/package.lisp",
                "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n",
            )
            .write_bytes("src/package_lib.bin", b"refloat-native\0");
        let root = harness.root();

        let output = RefloatSourceAssets::new(root)
            .write_package(&RefloatBuildInfo::new(
                "2026-07-02 06:00:00-06:00",
                "0ef6e99",
            ))
            .expect("refloat package");

        assert_eq!(output, root.join("refloat.vescpkg"));
        let package = Package::read(&output).expect("written package");
        assert_eq!(package.name, "Refloat");
        assert!(package.description_md.contains("- Version: 1.2.1"));
        assert_eq!(
            package.qml_file,
            "Item{property string title:\"Refloat 1.2.1\"}"
        );
        let (_code, imports) = parse_lisp_imports(&package.lisp_data).expect("lisp imports");
        assert_eq!(imports[0].payload, b"refloat-native\0");
    }
}
