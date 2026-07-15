use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
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
    qml_fullscreen: bool,
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
    validate_flat_image_relocations(&artifacts.elf, package.force_c_float_math)?;
    let package_slug = package_slug(&package.display_name);
    let artifact_name = format!("{package_slug}-{}", package.version);
    let artifact_root = metadata_target_dir(&metadata)?.join("vescpkg");
    let output_dir = artifact_root.join(&artifact_name);
    if output_dir.parent() != Some(artifact_root.as_path()) {
        return Err(BuildError(
            "package artifact escaped its output root".to_owned(),
        ));
    }

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(output_dir.join("src"))?;
    write_flattened_elf(&artifacts.elf, &output_dir.join("src/package_lib.bin"))?;
    let generated = artifacts.out_dir.as_ref().map(|path| path.join("vescpkg"));
    if let Some(generated) = generated.as_ref() {
        stage_generated_assets(generated, &output_dir)?;
    }
    let bytes = build_package_bytes(&package, &artifact_name, &output_dir)?;
    let output = output_dir.join(format!("{artifact_name}.vescpkg"));
    fs::write(&output, bytes)?;

    Ok(output)
}

fn build_package_bytes(
    package: &PackageMetadata,
    artifact_name: &str,
    output_dir: &Path,
) -> Result<Vec<u8>, BuildError> {
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
            "import QtQuick 2.15\n\nItem {{\n    property string pkgName: \"{}\"\n    property string pkgDescriptionMd: \"README.md\"\n    property string pkgLisp: \"code.lisp\"\n    property string pkgQml: \"{qml_path}\"\n    property bool pkgQmlIsFullscreen: {}\n    property string pkgOutput: \"{artifact_name}.vescpkg\"\n}}\n",
            package.display_name, package.qml_fullscreen,
        )
    });
    validate_descriptor_fullscreen(&descriptor, package.qml_fullscreen)?;
    build_vesc_package(&VescPackageInput {
        name: &package.display_name,
        description_md: &description_md,
        lisp_source: &loader,
        lisp_editor_path: output_dir,
        qml_file: &qml,
        pkg_desc_qml: &descriptor,
        qml_is_fullscreen: package.qml_fullscreen,
    })
    .map_err(BuildError::from)
}

fn validate_descriptor_fullscreen(descriptor: &str, expected: bool) -> Result<(), BuildError> {
    let declared = descriptor
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(declaration, _)| {
            declaration
                .split_whitespace()
                .eq(["property", "bool", "pkgQmlIsFullscreen"])
        })
        .map(|(_, value)| value.split_whitespace().next().unwrap_or_default())
        .map(|value| match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(BuildError(format!(
                "pkgQmlIsFullscreen must be the literal `true` or `false`, found `{value}`"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    match declared.as_slice() {
        [] => Ok(()),
        [actual] if *actual == expected => Ok(()),
        [actual] => Err(BuildError(format!(
            "pkgdesc.qml declares pkgQmlIsFullscreen={actual}, but Cargo metadata declares qml-fullscreen={expected}"
        ))),
        _ => Err(BuildError(
            "pkgdesc.qml declares pkgQmlIsFullscreen more than once".to_owned(),
        )),
    }
}

fn validate_target(target: &str) -> Result<(), BuildError> {
    (target == VESC_TARGET)
        .then_some(())
        .ok_or_else(|| BuildError(format!("unsupported VESC package target `{target}`")))
}

fn stage_generated_assets(generated: &Path, output: &Path) -> Result<(), BuildError> {
    if !generated.exists() {
        return Ok(());
    }
    stage_generated_asset_tree(generated, output, generated)
}

fn stage_generated_asset_tree(
    source_root: &Path,
    output: &Path,
    source_dir: &Path,
) -> Result<(), BuildError> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let source = entry.path();
        let relative = source
            .strip_prefix(source_root)
            .map_err(|error| BuildError(error.to_string()))?;
        if relative == Path::new("src/package_lib.bin") {
            return Err(BuildError(
                "generated asset `src/package_lib.bin` conflicts with the native payload"
                    .to_owned(),
            ));
        }
        let destination = output.join(relative);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&destination)?;
            stage_generated_asset_tree(source_root, output, &source)?;
        } else if file_type.is_file() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, destination)?;
        } else {
            return Err(BuildError(format!(
                "generated package asset `{}` must be a regular file or directory",
                source.display()
            )));
        }
    }
    Ok(())
}

fn validate_path_component(label: &str, value: &str) -> Result<(), BuildError> {
    let mut components = Path::new(value).components();
    let is_normal = matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && !value.contains(['/', '\\']);
    is_normal.then_some(()).ok_or_else(|| {
        BuildError(format!(
            "{label} `{value}` must be one normal filesystem path component"
        ))
    })
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
    validate_path_component("package version", &version)?;

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
        qml_fullscreen: package["metadata"]["vescpkg"]["qml-fullscreen"]
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
        .args([
            "-Wl,--emit-relocs",
            "-Wl,--gc-sections",
            "-Wl,--undefined=init",
            "-T",
        ])
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

fn validate_flat_image_relocations(
    elf: &Path,
    allow_c_float_math_runtime: bool,
) -> Result<(), BuildError> {
    let output = Command::new("arm-none-eabi-readelf")
        .args(["--relocs", "--wide"])
        .arg(elf)
        .output()?;
    if !output.status.success() {
        return Err(BuildError(format!(
            "arm-none-eabi-readelf failed for {} with {}",
            elf.display(),
            output.status
        )));
    }
    validate_relocation_report(
        &String::from_utf8_lossy(&output.stdout),
        allow_c_float_math_runtime,
    )
}

fn validate_relocation_report(
    report: &str,
    allow_c_float_math_runtime: bool,
) -> Result<(), BuildError> {
    const RUNTIME_REBASED_CODE: [&str; 2] = [".rel.init_fun", ".rel.text"];
    const NON_LOADABLE_PREFIXES: [&str; 2] = [".rel.debug", ".rel.comment"];
    const ABSOLUTE_RELOCATIONS: [&str; 4] = [
        "R_ARM_ABS32",
        "R_ARM_TARGET1",
        "R_ARM_MOVW_ABS_NC",
        "R_ARM_MOVT_ABS",
    ];

    let mut relocation_section = "";
    for line in report.lines() {
        if let Some(section) = line
            .strip_prefix("Relocation section '")
            .and_then(|line| line.split_once('\'').map(|(section, _)| section))
        {
            relocation_section = section;
            continue;
        }
        let relocation = line
            .split_whitespace()
            .find(|field| ABSOLUTE_RELOCATIONS.contains(field));
        let symbol = line.split_whitespace().last().unwrap_or_default();
        let non_loadable = NON_LOADABLE_PREFIXES
            .iter()
            .any(|prefix| relocation_section.starts_with(prefix));
        let c_float_math_runtime = allow_c_float_math_runtime
            && relocation_section == ".rel.data"
            && matches!(symbol, "_impure_data" | "__sf");
        if let Some(relocation) = relocation
            && !RUNTIME_REBASED_CODE.contains(&relocation_section)
            && !non_loadable
            && !c_float_math_runtime
        {
            return Err(BuildError(format!(
                "unsupported absolute relocation `{relocation}` in `{relocation_section}`; VESC flat images cannot contain pointer-bearing loadable statics"
            )));
        }
    }
    Ok(())
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
        PackageMetadata, PayloadKind, build_package_bytes, cargo_message_artifacts,
        command_failure_message, metadata_workspace_root, package_slug, select_package,
        stage_generated_assets, staticlib_linker_script, validate_descriptor_fullscreen,
        validate_relocation_report, validate_target,
    };
    use crate::package::Package;
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
    fn stages_nested_generated_assets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generated = temp.path().join("generated");
        let output = temp.path().join("output");
        std::fs::create_dir_all(generated.join("lib/config")).expect("generated directory");
        std::fs::create_dir_all(&output).expect("output directory");
        std::fs::write(generated.join("lib/config/defaults.lisp"), "(define x 1)\n")
            .expect("nested Lisp");

        stage_generated_assets(&generated, &output).expect("stage assets");

        assert_eq!(
            std::fs::read_to_string(output.join("lib/config/defaults.lisp"))
                .expect("staged nested Lisp"),
            "(define x 1)\n"
        );
    }

    #[test]
    fn generated_assets_cannot_replace_the_native_payload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generated = temp.path().join("generated");
        let output = temp.path().join("output");
        std::fs::create_dir_all(generated.join("src")).expect("generated directory");
        std::fs::create_dir_all(output.join("src")).expect("output directory");
        std::fs::write(generated.join("src/package_lib.bin"), b"asset").expect("asset payload");
        std::fs::write(output.join("src/package_lib.bin"), b"native").expect("native payload");

        let error = stage_generated_assets(&generated, &output).expect_err("reserved payload");

        assert!(error.to_string().contains("src/package_lib.bin"));
        assert_eq!(
            std::fs::read(output.join("src/package_lib.bin")).expect("native payload"),
            b"native"
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
                    "force-c-float-math": true,
                    "qml-fullscreen": true
                }}
            }]
        });

        let package = select_package(&metadata, "static-package").expect("package metadata");

        assert_eq!(package.payload_kind, PayloadKind::Staticlib);
        assert_eq!(package.version, "1.2.1");
        assert!(package.force_c_float_math);
        assert!(package.qml_fullscreen);
    }

    #[test]
    fn builds_and_decodes_a_fullscreen_qml_package() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("ui.qml"), "import QtQuick 2.15\nItem {}\n")
            .expect("QML fixture");
        std::fs::write(
            temp.path().join("pkgdesc.qml"),
            "Item {\n    property bool pkgQmlIsFullscreen: true\n}\n",
        )
        .expect("descriptor fixture");
        std::fs::create_dir(temp.path().join("src")).expect("payload directory");
        std::fs::write(temp.path().join("src/package_lib.bin"), b"native")
            .expect("payload fixture");
        let package = PackageMetadata {
            id: "fixture".to_owned(),
            name: "fixture".to_owned(),
            target_name: "fixture".to_owned(),
            version: "0.1.0".to_owned(),
            display_name: "Fullscreen fixture".to_owned(),
            payload_kind: PayloadKind::Binary,
            force_c_float_math: false,
            qml_fullscreen: true,
        };

        let bytes = build_package_bytes(&package, "Fullscreen-fixture-0.1.0", temp.path())
            .expect("build package");
        let decoded = Package::from_bytes(&bytes).expect("decode package");

        assert!(decoded.qml_app_ui.expect("QML app UI").mode.is_fullscreen());
    }

    #[test]
    fn rejects_conflicting_fullscreen_metadata_and_descriptor() {
        let error =
            validate_descriptor_fullscreen("property bool pkgQmlIsFullscreen: true\n", false)
                .expect_err("contradictory descriptor");

        assert!(error.to_string().contains("Cargo metadata"));
    }

    #[test]
    fn rejects_absolute_relocations_in_loadable_static_data() {
        let report = "\
Relocation section '.rel.rodata' at offset 0x100 contains 1 entry:\n\
 Offset     Info    Type                Sym. Value  Symbol's Name\n\
00000000  00000102 R_ARM_ABS32            00000004   STATIC_NAME\n";

        let error = validate_relocation_report(report, false).expect_err("pointer-bearing static");

        assert!(error.to_string().contains("R_ARM_ABS32"));
        assert!(
            error
                .to_string()
                .contains("pointer-bearing loadable statics")
        );
    }

    #[test]
    fn permits_runtime_rebased_code_literals() {
        let report = "\
Relocation section '.rel.text' at offset 0x100 contains 1 entry:\n\
 Offset     Info    Type                Sym. Value  Symbol's Name\n\
00000070  00000e02 R_ARM_ABS32            00000191   stop_package\n";

        assert!(validate_relocation_report(report, false).is_ok());
    }

    #[test]
    fn ignores_debug_relocations_omitted_from_the_flat_binary() {
        let report = "\
Relocation section '.rel.debug_frame' at offset 0x100 contains 1 entry:\n\
 Offset     Info    Type                Sym. Value  Symbol's Name\n\
00000014  00000902 R_ARM_ABS32            00000000   .debug_frame\n";

        assert!(validate_relocation_report(report, false).is_ok());
    }

    #[test]
    fn permits_newlib_state_pulled_in_by_requested_c_float_math() {
        let report = "\
Relocation section '.rel.data' at offset 0x100 contains 2 entries:\n\
 Offset     Info    Type                Sym. Value  Symbol's Name\n\
00000010  00013502 R_ARM_ABS32            00000018   _impure_data\n\
0000001c  00012602 R_ARM_ABS32            00000218   __sf\n";

        assert!(validate_relocation_report(report, true).is_ok());
        assert!(validate_relocation_report(report, false).is_err());
    }

    #[test]
    fn rejects_a_version_that_is_not_one_path_component() {
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
                "metadata": {"vescpkg": {"version": "../../outside"}}
            }]
        });

        let error = select_package(&metadata, "static-package").expect_err("unsafe version");

        assert!(error.to_string().contains("version"));
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
            qml_fullscreen: false,
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
            qml_fullscreen: false,
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
