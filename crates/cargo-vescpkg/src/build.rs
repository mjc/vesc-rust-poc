use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use crate::package_format::{VescPackageInput, build_vesc_package};

const DEFAULT_LOADER: &str = concat!(
    "(import \"src/package_lib.bin\" 'package-lib)\n",
    "(print \"vesc-rust-load-v7\")\n",
    "(print (load-native-lib package-lib))\n",
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildOptions {
    pub(crate) package: String,
    pub(crate) manifest_path: Option<PathBuf>,
    pub(crate) target: String,
    pub(crate) profile: String,
    pub(crate) features: Option<String>,
}

#[derive(Debug)]
pub(crate) struct BuildError(String);

impl std::fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BuildError {}

impl From<std::io::Error> for BuildError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

#[derive(Debug)]
struct PackageMetadata {
    id: String,
    name: String,
    target_name: String,
    version: String,
    display_name: String,
}

#[derive(Debug)]
struct CargoArtifacts {
    elf: PathBuf,
    out_dir: Option<PathBuf>,
}

pub(crate) fn build_package(root: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError> {
    let metadata = cargo_metadata(root, options)?;
    let package = select_package(&metadata, &options.package)?;
    let artifacts = cargo_build(root, options, &package)?;
    let package_slug = package_slug(&package.display_name);
    let artifact_name = format!("{package_slug}-{}", package.version);
    let output_dir = metadata_target_dir(&metadata)?
        .join("vescpkg")
        .join(&artifact_name);

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(output_dir.join("src"))?;
    write_flattened_elf(&artifacts.elf, &output_dir.join("src/package_lib.bin"))?;
    let generated = artifacts.out_dir.as_ref().map(|path| path.join("vescpkg"));
    let read = |name: &str| {
        generated
            .as_ref()
            .map(|path| path.join(name))
            .filter(|path| path.is_file())
            .map(fs::read_to_string)
            .transpose()
    };
    let description_md = read("README.md")?
        .unwrap_or_else(|| format!("{} {}\n", package.display_name, package.version));
    let loader = read("code.lisp")?.unwrap_or_else(|| DEFAULT_LOADER.to_owned());
    let descriptor = read("pkgdesc.qml")?.unwrap_or_else(|| {
        format!(
            "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"{artifact_name}.vescpkg\"\n}}\n",
            package.display_name
        )
    });
    let output = output_dir.join(format!("{artifact_name}.vescpkg"));
    let bytes = build_vesc_package(&VescPackageInput {
        name: &package.display_name,
        description_md: &description_md,
        lisp_source: &loader,
        lisp_editor_path: &output_dir,
        qml_file: "",
        pkg_desc_qml: &descriptor,
        qml_is_fullscreen: false,
    })?;
    fs::write(&output, bytes)?;

    Ok(output)
}

fn command_output(command: &mut Command) -> Result<Output, BuildError> {
    let output = command.output()?;
    if !output.stderr.is_empty() {
        let _ = std::io::stderr().write_all(&output.stderr);
    }
    match output.status.success() {
        true => Ok(output),
        false => Err(BuildError(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
    }
}

fn cargo_metadata(root: &Path, options: &BuildOptions) -> Result<Value, BuildError> {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(path) = &options.manifest_path {
        command.args([
            "--manifest-path",
            path.to_str()
                .ok_or_else(|| BuildError("manifest path is not valid UTF-8".to_owned()))?,
        ]);
    }
    let output = command_output(&mut command)?;
    serde_json::from_slice(&output.stdout)
        .map_err(|error| BuildError(format!("invalid Cargo metadata JSON: {error}")))
}

#[must_use]
fn is_binary_target(target: &Value) -> bool {
    target["kind"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
}

fn select_package(metadata: &Value, requested: &str) -> Result<PackageMetadata, BuildError> {
    let package = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"].as_str() == Some(requested))
        })
        .ok_or_else(|| BuildError(format!("Cargo package `{requested}` was not found")))?;
    let name = package["name"]
        .as_str()
        .ok_or_else(|| BuildError("package has no name".to_owned()))?;
    let id = package["id"]
        .as_str()
        .ok_or_else(|| BuildError(format!("package `{name}` has no package ID")))?;
    let version = package["version"]
        .as_str()
        .ok_or_else(|| BuildError(format!("package `{name}` has no version")))?;
    let binary_targets = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|target| is_binary_target(target))
        .filter_map(|target| target["name"].as_str())
        .collect::<Vec<_>>();
    let [target_name] = binary_targets.as_slice() else {
        return Err(BuildError(format!(
            "package `{name}` must have exactly one binary target (found {})",
            binary_targets.len()
        )));
    };
    let display_name = package["metadata"]["vescpkg"]["name"]
        .as_str()
        .unwrap_or(name)
        .to_owned();

    Ok(PackageMetadata {
        id: id.to_owned(),
        name: name.to_owned(),
        target_name: (*target_name).to_owned(),
        version: version.to_owned(),
        display_name,
    })
}

fn metadata_target_dir(metadata: &Value) -> Result<PathBuf, BuildError> {
    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| BuildError("Cargo metadata has no target directory".to_owned()))
}

fn cargo_build(
    root: &Path,
    options: &BuildOptions,
    package: &PackageMetadata,
) -> Result<CargoArtifacts, BuildError> {
    let mut command = Command::new("cargo");
    command.current_dir(root).args([
        "build",
        "--message-format=json-render-diagnostics",
        "--target",
        &options.target,
        "--profile",
        &options.profile,
        "--package",
        &package.name,
    ]);
    if let Some(path) = &options.manifest_path {
        command.args([
            "--manifest-path",
            path.to_str()
                .ok_or_else(|| BuildError("manifest path is not valid UTF-8".to_owned()))?,
        ]);
    }
    if let Some(features) = &options.features {
        command.args(["--features", features]);
    }
    let output = command_output(&mut command)?;
    let (elf, out_dir) = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .fold((None, None), |(elf, out_dir), message| {
            let (message_elf, message_out_dir) = cargo_message_artifacts(&message, package);
            (message_elf.or(elf), message_out_dir.or(out_dir))
        });

    let elf = elf.ok_or_else(|| {
        BuildError(format!(
            "Cargo produced no final binary for `{}`",
            package.name
        ))
    })?;
    Ok(CargoArtifacts { elf, out_dir })
}

#[must_use]
fn cargo_message_artifacts(
    message: &Value,
    package: &PackageMetadata,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let out_dir = (message["reason"] == "build-script-executed"
        && message["package_id"].as_str() == Some(package.id.as_str()))
    .then(|| message["out_dir"].as_str())
    .flatten()
    .map(PathBuf::from);
    let elf = (message["reason"] == "compiler-artifact"
        && message["package_id"].as_str() == Some(package.id.as_str())
        && message["target"]["name"].as_str() == Some(package.target_name.as_str())
        && is_binary_target(&message["target"]))
    .then(|| message["executable"].as_str())
    .flatten()
    .map(PathBuf::from);
    (elf, out_dir)
}

fn write_flattened_elf(elf: &Path, output: &Path) -> Result<(), BuildError> {
    let status = Command::new("rust-objcopy")
        .arg(elf)
        .args(["-O", "binary"])
        .arg(output)
        .status()?;
    status.success().then_some(()).ok_or_else(|| {
        BuildError(format!(
            "rust-objcopy failed for {} with {status}",
            elf.display()
        ))
    })
}

fn package_slug(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PackageMetadata, cargo_message_artifacts, package_slug, select_package};
    use serde_json::json;

    #[test]
    fn package_slug_matches_existing_artifact_names() {
        assert_eq!(package_slug("A minimal package"), "A-minimal-package");
    }

    #[test]
    fn selects_any_single_binary_package_from_cargo_metadata() {
        let metadata = json!({
            "packages": [{
                "name": "minimal-package",
                "id": "path+file:///tmp/minimal-package#0.1.0",
                "version": "0.1.0",
                "targets": [{"name": "minimal-package", "kind": ["bin"]}],
                "metadata": {"vescpkg": {"name": "Minimal package"}}
            }]
        });

        let package = select_package(&metadata, "minimal-package").expect("package metadata");

        assert_eq!(package.name, "minimal-package");
        assert_eq!(package.target_name, "minimal-package");
        assert_eq!(package.display_name, "Minimal package");
    }

    #[test]
    fn selects_only_matching_build_artifacts() {
        let package = PackageMetadata {
            id: "path+file:///tmp/minimal-package#0.1.0".to_owned(),
            name: "minimal-package".to_owned(),
            target_name: "minimal-package".to_owned(),
            version: "0.1.0".to_owned(),
            display_name: "Minimal package".to_owned(),
        };
        let artifact = json!({
            "reason": "compiler-artifact",
            "package_id": "path+file:///tmp/minimal-package#0.1.0",
            "target": {"name": "minimal-package", "kind": ["bin"]},
            "executable": "/tmp/minimal-package"
        });
        let build_script = json!({
            "reason": "build-script-executed",
            "package_id": "path+file:///tmp/minimal-package#0.1.0",
            "out_dir": "/tmp/out"
        });
        let unrelated = json!({
            "reason": "compiler-artifact",
            "package_id": "path+file:///tmp/other#0.1.0",
            "target": {"name": "minimal-package", "kind": ["bin"]},
            "executable": "/tmp/other"
        });

        assert_eq!(
            cargo_message_artifacts(&artifact, &package),
            (Some("/tmp/minimal-package".into()), None)
        );
        assert_eq!(
            cargo_message_artifacts(&build_script, &package),
            (None, Some("/tmp/out".into()))
        );
        assert_eq!(cargo_message_artifacts(&unrelated, &package), (None, None));
    }
}
