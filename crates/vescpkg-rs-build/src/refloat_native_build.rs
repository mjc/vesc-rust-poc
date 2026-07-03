use std::path::{Path, PathBuf};

use crate::PackageError;
use crate::native_lib_toolchain::NativeLibToolchain;

/// Refloat native payload build plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatNativeBuildPlan {
    source_root: PathBuf,
    vesc_tool: String,
    git_hash: RefloatGitHash,
}

/// Hex commit prefix rendered into Refloat's generated config header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefloatGitHash(String);

impl RefloatGitHash {
    /// Parse a bare hexadecimal git commit prefix.
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        if Self::is_valid(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(PackageError::Build(format!(
                "Refloat git hash must be hexadecimal digits without a 0x prefix: {value}"
            )))
        }
    }

    /// Return whether `value` can be used inside `0x{{GIT_HASH}}`.
    pub fn is_valid(value: &str) -> bool {
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RefloatGitHash {
    fn default() -> Self {
        Self("0".to_owned())
    }
}

impl RefloatNativeBuildPlan {
    /// Create a Refloat native build plan rooted at a Refloat source checkout.
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self {
            source_root: source_root.into(),
            vesc_tool: "vesc_tool".to_owned(),
            git_hash: RefloatGitHash::default(),
        }
    }

    /// Set the `VESC_TOOL` executable passed to Refloat's native Makefile.
    pub fn with_vesc_tool(mut self, vesc_tool: impl Into<String>) -> Self {
        self.vesc_tool = vesc_tool.into();
        self
    }

    /// Set the git hash embedded into generated Refloat config headers.
    pub fn with_git_hash(mut self, git_hash: impl AsRef<str>) -> Result<Self, PackageError> {
        self.git_hash = RefloatGitHash::parse(git_hash.as_ref())?;
        Ok(self)
    }

    /// Generate Refloat native inputs and run the native payload build.
    pub fn build_with(&self, toolchain: &impl NativeLibToolchain) -> Result<PathBuf, PackageError> {
        self.write_conf_general()?;
        let src = self.source_root.join("src");
        run_toolchain(
            toolchain,
            "make",
            &[
                "-C".to_owned(),
                display_path(&src),
                format!("VESC_TOOL={}", self.vesc_tool),
            ],
        )?;
        Ok(src.join("package_lib.bin"))
    }

    fn write_conf_general(&self) -> Result<(), PackageError> {
        let template = read_text(self.source_root.join("src/conf/conf_general.h.in"))?;
        let package_name =
            truncate_chars(&read_trimmed(self.source_root.join("package_name"))?, 20);
        let version = read_trimmed(self.source_root.join("version"))?;
        let parts = RefloatVersionParts::parse(&version);
        let rendered = template
            .replace("{{PACKAGE_NAME}}", &package_name)
            .replace("{{VERSION}}", &version)
            .replace("{{MAJOR_VERSION}}", parts.major)
            .replace("{{MINOR_VERSION}}", parts.minor)
            .replace("{{PATCH_VERSION}}", parts.patch)
            .replace("{{VERSION_SUFFIX}}", parts.suffix)
            .replace("{{GIT_HASH}}", self.git_hash.as_str());
        std::fs::write(self.source_root.join("src/conf/conf_general.h"), rendered)?;
        Ok(())
    }
}

struct RefloatVersionParts<'a> {
    major: &'a str,
    minor: &'a str,
    patch: &'a str,
    suffix: &'a str,
}

impl<'a> RefloatVersionParts<'a> {
    fn parse(version: &'a str) -> Self {
        let mut dot_parts = version.split('.');
        let major = dot_parts.next().unwrap_or_default();
        let minor = dot_parts.next().unwrap_or_default();
        let patch_and_suffix = dot_parts.next().unwrap_or_default();
        let (patch, suffix) = patch_and_suffix
            .split_once('-')
            .unwrap_or((patch_and_suffix, patch_and_suffix));

        Self {
            major,
            minor,
            patch,
            suffix,
        }
    }
}

fn run_toolchain(
    toolchain: &impl NativeLibToolchain,
    program: &str,
    args: &[String],
) -> Result<(), PackageError> {
    let borrowed: Vec<_> = args.iter().map(String::as_str).collect();
    toolchain
        .run(program, &borrowed)
        .map_err(PackageError::Build)
}

fn read_text(path: impl AsRef<Path>) -> Result<String, PackageError> {
    std::fs::read_to_string(path).map_err(Into::into)
}

fn read_trimmed(path: impl AsRef<Path>) -> Result<String, PackageError> {
    Ok(read_text(path)?.trim().to_owned())
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::RefloatNativeBuildPlan;
    use crate::native_lib_toolchain::RecordingNativeLibToolchain;
    use crate::test_support::PackageTestHarness;

    #[test]
    fn native_build_generates_config_header_and_runs_refloat_make_steps() {
        let harness = PackageTestHarness::new()
            .write_text("package_name", "Refloat Long Package Name\n")
            .write_text("version", "1.2.1-beta\n")
            .write_text(
                "src/conf/conf_general.h.in",
                "#define APP_NAME \"{{PACKAGE_NAME}}\"\n#define VERSION \"{{VERSION}}\"\n#define MAJOR {{MAJOR_VERSION}}\n#define MINOR {{MINOR_VERSION}}\n#define PATCH {{PATCH_VERSION}}\n#define SUFFIX \"{{VERSION_SUFFIX}}\"\n#define GIT 0x{{GIT_HASH}}\n",
            )
            .write_text("src/conf/settings.xml", "<config />\n");
        let root = harness.root();
        let toolchain = RecordingNativeLibToolchain::default();

        let output = RefloatNativeBuildPlan::new(root)
            .with_vesc_tool("vesc_tool")
            .with_git_hash("0ef6e99d")
            .expect("valid git hash")
            .build_with(&toolchain)
            .expect("native build plan");

        assert_eq!(output, root.join("src/package_lib.bin"));
        assert_eq!(
            std::fs::read_to_string(root.join("src/conf/conf_general.h"))
                .expect("generated conf_general.h"),
            "#define APP_NAME \"Refloat Long Package\"\n#define VERSION \"1.2.1-beta\"\n#define MAJOR 1\n#define MINOR 2\n#define PATCH 1\n#define SUFFIX \"beta\"\n#define GIT 0x0ef6e99d\n"
        );
        assert_eq!(
            toolchain.calls.borrow().as_slice(),
            &[(
                "make".to_owned(),
                vec![
                    "-C".to_owned(),
                    root.join("src").display().to_string(),
                    "VESC_TOOL=vesc_tool".to_owned()
                ]
            )]
        );
    }

    #[test]
    fn native_build_defaults_git_hash_to_valid_hex_zero() {
        let harness = PackageTestHarness::new()
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text("src/conf/conf_general.h.in", "#define GIT 0x{{GIT_HASH}}\n")
            .write_text("src/conf/settings.xml", "<config />\n");
        let root = harness.root();
        let toolchain = RecordingNativeLibToolchain::default();

        RefloatNativeBuildPlan::new(root)
            .build_with(&toolchain)
            .expect("native build plan");

        assert_eq!(
            std::fs::read_to_string(root.join("src/conf/conf_general.h"))
                .expect("generated conf_general.h"),
            "#define GIT 0x0\n"
        );
    }

    #[test]
    fn native_build_matches_refloat_makefile_version_suffix_for_release_tags() {
        let harness = PackageTestHarness::new()
            .write_text("package_name", "Refloat\n")
            .write_text("version", "1.2.1\n")
            .write_text(
                "src/conf/conf_general.h.in",
                "#define VERSION_SUFFIX \"{{VERSION_SUFFIX}}\"\n",
            );
        let root = harness.root();
        let toolchain = RecordingNativeLibToolchain::default();

        RefloatNativeBuildPlan::new(root)
            .build_with(&toolchain)
            .expect("native build plan");

        assert_eq!(
            std::fs::read_to_string(root.join("src/conf/conf_general.h"))
                .expect("generated conf_general.h"),
            "#define VERSION_SUFFIX \"1\"\n"
        );
    }
}
