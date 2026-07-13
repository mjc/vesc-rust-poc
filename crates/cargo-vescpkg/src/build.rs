use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::package_format::{VescPackageInput, write_vesc_package};

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

impl BuildOptions {
    pub(crate) fn new(
        package: String,
        manifest_path: Option<PathBuf>,
        target: String,
        profile: String,
        features: Option<String>,
    ) -> Self {
        Self {
            package,
            manifest_path,
            target,
            profile,
            features,
        }
    }
}

#[derive(Debug)]
pub(crate) enum BuildError {
    Io(std::io::Error),
    Cargo(String),
    Metadata(String),
    Package(String),
    Artifact(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Cargo(error) => write!(formatter, "Cargo build failed: {error}"),
            Self::Metadata(error) => write!(formatter, "Cargo metadata failed: {error}"),
            Self::Package(error) => write!(formatter, "invalid package configuration: {error}"),
            Self::Artifact(error) => write!(formatter, "invalid Cargo artifact: {error}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<std::io::Error> for BuildError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
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

#[derive(Debug)]
struct PackageAssets {
    description_md: String,
    loader: String,
    descriptor: String,
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
    let assets = package_assets(&artifacts, &package, &artifact_name)?;
    let output = output_dir.join(format!("{artifact_name}.vescpkg"));
    write_vesc_package(
        &output,
        &VescPackageInput {
            name: &package.display_name,
            description_md: &assets.description_md,
            lisp_source: &assets.loader,
            lisp_editor_path: &output_dir,
            qml_file: "",
            pkg_desc_qml: &assets.descriptor,
            qml_is_fullscreen: false,
        },
    )?;

    Ok(output)
}

fn cargo_metadata(root: &Path, options: &BuildOptions) -> Result<Value, BuildError> {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(path) = &options.manifest_path {
        command.args([
            "--manifest-path",
            path.to_str().ok_or_else(|| {
                BuildError::Metadata("manifest path is not valid UTF-8".to_owned())
            })?,
        ]);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(BuildError::Metadata(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| BuildError::Metadata(format!("invalid Cargo metadata JSON: {error}")))
}

fn select_package(metadata: &Value, requested: &str) -> Result<PackageMetadata, BuildError> {
    let package = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"].as_str() == Some(requested))
        })
        .ok_or_else(|| BuildError::Package(format!("Cargo package `{requested}` was not found")))?;
    let name = package["name"]
        .as_str()
        .ok_or_else(|| BuildError::Package("package has no name".to_owned()))?;
    let id = package["id"]
        .as_str()
        .ok_or_else(|| BuildError::Package(format!("package `{name}` has no package ID")))?;
    let version = package["version"]
        .as_str()
        .ok_or_else(|| BuildError::Package(format!("package `{name}` has no version")))?;
    let binary_targets = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .filter_map(|target| target["name"].as_str())
        .collect::<Vec<_>>();
    let [target_name] = binary_targets.as_slice() else {
        return Err(BuildError::Package(format!(
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
        .ok_or_else(|| BuildError::Metadata("Cargo metadata has no target directory".to_owned()))
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
                .ok_or_else(|| BuildError::Cargo("manifest path is not valid UTF-8".to_owned()))?,
        ]);
    }
    if let Some(features) = &options.features {
        command.args(["--features", features]);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(BuildError::Cargo(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let mut elf = None;
    let mut out_dir = None;
    for message in output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
    {
        if message["reason"] == "build-script-executed"
            && message["package_id"].as_str() == Some(package.id.as_str())
        {
            out_dir = message["out_dir"].as_str().map(PathBuf::from);
        }
        if message["reason"] != "compiler-artifact"
            || message["package_id"].as_str() != Some(package.id.as_str())
            || message["target"]["name"].as_str() != Some(package.target_name.as_str())
            || !message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        {
            continue;
        }
        elf = message["executable"].as_str().map(PathBuf::from);
    }

    let elf = elf.ok_or_else(|| {
        BuildError::Cargo(format!(
            "Cargo produced no final binary for `{}`",
            package.name
        ))
    })?;
    Ok(CargoArtifacts { elf, out_dir })
}

fn package_assets(
    artifacts: &CargoArtifacts,
    package: &PackageMetadata,
    artifact_name: &str,
) -> Result<PackageAssets, BuildError> {
    let generated = artifacts.out_dir.as_ref().map(|path| path.join("vescpkg"));
    let read = |name: &str| {
        generated
            .as_ref()
            .map(|path| path.join(name))
            .filter(|path| path.is_file())
            .map(fs::read_to_string)
            .transpose()
    };
    Ok(PackageAssets {
        description_md: read("README.md")?.unwrap_or_else(|| {
            format!("{} {}\n", package.display_name, package.version)
        }),
        loader: read("code.lisp")?.unwrap_or_else(|| DEFAULT_LOADER.to_owned()),
        descriptor: read("pkgdesc.qml")?.unwrap_or_else(|| {
            format!(
                "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"{artifact_name}.vescpkg\"\n}}\n",
                package.display_name
            )
        }),
    })
}

fn write_flattened_elf(elf: &Path, output: &Path) -> Result<(), BuildError> {
    let status = Command::new("rust-objcopy")
        .arg(elf)
        .args(["-O", "binary"])
        .arg(output)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::Artifact(format!(
            "rust-objcopy failed for {} with {status}",
            elf.display()
        )))
    }
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
    use super::package_slug;

    #[test]
    fn package_slug_matches_existing_artifact_names() {
        assert_eq!(package_slug("A minimal package"), "A-minimal-package");
    }
}
