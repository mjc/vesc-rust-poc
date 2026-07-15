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
const VESC_TARGET: &str = "thumbv7em-none-eabihf";

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
    payload_kind: PayloadKind,
    force_c_float_math: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayloadKind {
    Binary,
    Staticlib,
}

#[derive(Debug)]
struct CargoArtifacts {
    elf: PathBuf,
    out_dir: Option<PathBuf>,
}

pub(crate) fn build_package(root: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError> {
    validate_target(&options.target)?;
    let metadata = cargo_metadata(root, options)?;
    let package = select_package(&metadata, &options.package)?;
    let workspace_root = metadata_workspace_root(&metadata)?;
    let artifacts = cargo_build(root, &workspace_root, options, &package)?;
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
    if let Some(generated) = generated.as_ref() {
        stage_generated_assets(generated, &output_dir)?;
    }
    let read = |name: &str| {
        let path = output_dir.join(name);
        path.is_file()
            .then_some(path)
            .map(fs::read_to_string)
            .transpose()
    };
    let description_md = read("README.md")?
        .unwrap_or_else(|| format!("{} {}\n", package.display_name, package.version));
    let loader = read("code.lisp")?.unwrap_or_else(|| DEFAULT_LOADER.to_owned());
    let qml = read("ui.qml")?.unwrap_or_default();
    let qml_path = if qml.is_empty() { "" } else { "ui.qml" };
    let descriptor = read("pkgdesc.qml")?.unwrap_or_else(|| {
        format!(
            "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"{qml_path}\"\n    property bool pkgQmlIsFullscreen: false\n    property string pkgOutput: \"{artifact_name}.vescpkg\"\n}}\n",
            package.display_name,
        )
    });
    let output = output_dir.join(format!("{artifact_name}.vescpkg"));
    let bytes = build_vesc_package(&VescPackageInput {
        name: &package.display_name,
        description_md: &description_md,
        lisp_source: &loader,
        lisp_editor_path: &output_dir,
        qml_file: &qml,
        pkg_desc_qml: &descriptor,
        qml_is_fullscreen: false,
    })?;
    fs::write(&output, bytes)?;

    Ok(output)
}

fn validate_target(target: &str) -> Result<(), BuildError> {
    (target == VESC_TARGET)
        .then_some(())
        .ok_or_else(|| BuildError(format!("unsupported VESC package target `{target}`")))
}

fn stage_generated_assets(generated: &Path, output: &Path) -> Result<(), BuildError> {
    for name in [
        "README.md",
        "pkgdesc.qml",
        "code.lisp",
        "bms.lisp",
        "ui.qml",
    ] {
        let source = generated.join(name);
        if source.is_file() {
            fs::copy(source, output.join(name))?;
        }
    }
    Ok(())
}

fn command_output(command: &mut Command) -> Result<Output, BuildError> {
    let output = command.output()?;
    if !output.stderr.is_empty() {
        let _ = std::io::stderr().write_all(&output.stderr);
    }
    match output.status.success() {
        true => Ok(output),
        false => Err(BuildError(command_failure_message(
            &output.stdout,
            &output.stderr,
        ))),
    }
}

fn command_failure_message(stdout: &[u8], stderr: &[u8]) -> String {
    let rendered = cargo_rendered_diagnostics(stdout);
    if rendered.is_empty() {
        String::from_utf8_lossy(stderr).into_owned()
    } else {
        rendered
    }
}

fn cargo_rendered_diagnostics(stdout: &[u8]) -> String {
    stdout
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-message")
        .filter_map(|message| message["message"]["rendered"].as_str().map(str::to_owned))
        .collect()
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

#[must_use]
fn is_staticlib_target(target: &Value) -> bool {
    target["crate_types"]
        .as_array()
        .is_some_and(|types| types.iter().any(|kind| kind == "staticlib"))
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
    let cargo_version = package["version"]
        .as_str()
        .ok_or_else(|| BuildError(format!("package `{name}` has no version")))?;
    let payload_targets = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|target| {
            let kind = if is_binary_target(target) {
                PayloadKind::Binary
            } else if is_staticlib_target(target) {
                PayloadKind::Staticlib
            } else {
                return None;
            };
            Some((target["name"].as_str()?, kind))
        })
        .collect::<Vec<_>>();
    let [(target_name, payload_kind)] = payload_targets.as_slice() else {
        return Err(BuildError(format!(
            "package `{name}` must have exactly one binary or staticlib target (found {})",
            payload_targets.len()
        )));
    };
    let display_name = package["metadata"]["vescpkg"]["name"]
        .as_str()
        .unwrap_or(name)
        .to_owned();
    let version = package["metadata"]["vescpkg"]["version"]
        .as_str()
        .unwrap_or(cargo_version)
        .to_owned();

    Ok(PackageMetadata {
        id: id.to_owned(),
        name: name.to_owned(),
        target_name: (*target_name).to_owned(),
        version,
        display_name,
        payload_kind: *payload_kind,
        force_c_float_math: package["metadata"]["vescpkg"]["force-c-float-math"]
            .as_bool()
            .unwrap_or(false),
    })
}

fn metadata_target_dir(metadata: &Value) -> Result<PathBuf, BuildError> {
    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| BuildError("Cargo metadata has no target directory".to_owned()))
}

fn metadata_workspace_root(metadata: &Value) -> Result<PathBuf, BuildError> {
    metadata["workspace_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| BuildError("Cargo metadata has no workspace root".to_owned()))
}

fn cargo_build(
    root: &Path,
    workspace_root: &Path,
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
    if package.payload_kind == PayloadKind::Staticlib {
        let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
        command
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env("RUSTFLAGS", format!("{rustflags} -C relocation-model=pic"));
    }
    let output = command_output(&mut command)?;
    let (payload, out_dir) = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .fold((None, None), |(elf, out_dir), message| {
            let (message_elf, message_out_dir) = cargo_message_artifacts(&message, package);
            (message_elf.or(elf), message_out_dir.or(out_dir))
        });

    let payload = payload.ok_or_else(|| {
        BuildError(format!(
            "Cargo produced no final payload for `{}`",
            package.name
        ))
    })?;
    let elf = match package.payload_kind {
        PayloadKind::Binary => payload,
        PayloadKind::Staticlib => {
            link_staticlib(workspace_root, &payload, package.force_c_float_math)?
        }
    };
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
    let payload = (message["reason"] == "compiler-artifact"
        && message["package_id"].as_str() == Some(package.id.as_str())
        && message["target"]["name"].as_str() == Some(package.target_name.as_str()))
    .then(|| match package.payload_kind {
        PayloadKind::Binary => message["executable"].as_str().map(PathBuf::from),
        PayloadKind::Staticlib => message["filenames"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .find(|path| path.ends_with(".a"))
            .map(PathBuf::from),
    })
    .flatten();
    (payload, out_dir)
}

fn link_staticlib(
    root: &Path,
    archive: &Path,
    force_c_float_math: bool,
) -> Result<PathBuf, BuildError> {
    let elf = archive.with_extension("elf");
    let mut command = Command::new("arm-none-eabi-gcc");
    command.args([
        "-nostartfiles",
        "-static",
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
    ]);
    if force_c_float_math {
        command.args([
            "-Wl,--undefined=asinf",
            "-Wl,--undefined=cosf",
            "-Wl,--undefined=sinf",
            "-Wl,--undefined=sqrtf",
            "-lm",
        ]);
    }
    command
        .arg(archive)
        .args(["-Wl,--gc-sections", "-Wl,--undefined=init", "-T"])
        .arg(staticlib_linker_script(root))
        .arg("-o")
        .arg(&elf);
    command_output(&mut command)?;
    Ok(elf)
}

fn staticlib_linker_script(root: &Path) -> PathBuf {
    root.join("examples/vescpkg-link.ld")
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
    use super::{
        PackageMetadata, PayloadKind, cargo_message_artifacts, command_failure_message,
        metadata_workspace_root, package_slug, select_package, stage_generated_assets,
        staticlib_linker_script, validate_target,
    };
    use serde_json::json;

    #[test]
    fn package_slug_matches_existing_artifact_names() {
        assert_eq!(package_slug("A minimal package"), "A-minimal-package");
    }

    #[test]
    fn rejects_non_vesc_build_targets() {
        let error = validate_target("aarch64-apple-darwin").expect_err("host target");

        assert_eq!(
            error.to_string(),
            "unsupported VESC package target `aarch64-apple-darwin`"
        );
    }

    #[test]
    fn accepts_the_vesc_build_target() {
        assert!(validate_target("thumbv7em-none-eabihf").is_ok());
    }

    #[test]
    fn preserves_rendered_cargo_diagnostics() {
        let stdout = br#"{"reason":"compiler-message","message":{"rendered":"error[E0308]: mismatched types\n"}}
{"reason":"build-finished","success":false}
"#;

        assert_eq!(
            command_failure_message(stdout, b"error: could not compile `broken`"),
            "error[E0308]: mismatched types\n"
        );
    }

    #[test]
    fn staticlibs_use_the_cargo_package_linker_script() {
        let metadata = json!({"workspace_root": "/repo"});

        assert_eq!(
            staticlib_linker_script(&metadata_workspace_root(&metadata).expect("workspace root")),
            std::path::PathBuf::from("/repo/examples/vescpkg-link.ld")
        );
    }

    #[test]
    fn stages_generated_qml_and_lisp_imports() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generated = temp.path().join("generated");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&generated).expect("generated directory");
        std::fs::create_dir_all(&output).expect("output directory");
        std::fs::write(generated.join("ui.qml"), "Item {}\n").expect("QML");
        std::fs::write(generated.join("bms.lisp"), "(define bms true)\n").expect("BMS Lisp");

        stage_generated_assets(&generated, &output).expect("stage assets");

        assert_eq!(
            std::fs::read_to_string(output.join("ui.qml")).expect("staged QML"),
            "Item {}\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("bms.lisp")).expect("staged BMS Lisp"),
            "(define bms true)\n"
        );
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
        assert_eq!(package.payload_kind, PayloadKind::Binary);
    }

    #[test]
    fn selects_a_staticlib_package_from_cargo_metadata() {
        let metadata = json!({
            "packages": [{
                "name": "static-package",
                "id": "path+file:///tmp/static-package#0.1.0",
                "version": "0.1.0",
                "targets": [{
                    "name": "static_package",
                    "kind": ["lib"],
                    "crate_types": ["staticlib"]
                }],
                "metadata": {"vescpkg": {
                    "name": "Static package",
                    "version": "1.2.1",
                    "force-c-float-math": true
                }}
            }]
        });

        let package = select_package(&metadata, "static-package").expect("package metadata");

        assert_eq!(package.payload_kind, PayloadKind::Staticlib);
        assert_eq!(package.version, "1.2.1");
        assert!(package.force_c_float_math);
    }

    #[test]
    fn selects_only_matching_build_artifacts() {
        let package = PackageMetadata {
            id: "path+file:///tmp/minimal-package#0.1.0".to_owned(),
            name: "minimal-package".to_owned(),
            target_name: "minimal-package".to_owned(),
            version: "0.1.0".to_owned(),
            display_name: "Minimal package".to_owned(),
            payload_kind: PayloadKind::Binary,
            force_c_float_math: false,
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

    #[test]
    fn selects_staticlib_archive_from_build_artifacts() {
        let package = PackageMetadata {
            id: "path+file:///tmp/static-package#0.1.0".to_owned(),
            name: "static-package".to_owned(),
            target_name: "static_package".to_owned(),
            version: "0.1.0".to_owned(),
            display_name: "Static package".to_owned(),
            payload_kind: PayloadKind::Staticlib,
            force_c_float_math: false,
        };
        let artifact = json!({
            "reason": "compiler-artifact",
            "package_id": "path+file:///tmp/static-package#0.1.0",
            "target": {
                "name": "static_package",
                "kind": ["lib"],
                "crate_types": ["staticlib"]
            },
            "filenames": ["/tmp/libstatic_package.a"]
        });

        assert_eq!(
            cargo_message_artifacts(&artifact, &package),
            (Some("/tmp/libstatic_package.a".into()), None)
        );
    }
}
