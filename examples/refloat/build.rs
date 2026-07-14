#![allow(missing_docs)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let refloat = manifest.join("../../third_party/refloat");
    let output =
        Path::new(&std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("vescpkg");
    std::fs::create_dir_all(&output).expect("create Refloat package assets");

    for (source, target) in [
        ("package_README.md", "README.md"),
        ("pkgdesc.qml", "pkgdesc.qml"),
        ("lisp/package.lisp", "code.lisp"),
        ("lisp/bms.lisp", "bms.lisp"),
    ] {
        let source = refloat.join(source);
        std::fs::copy(&source, output.join(target)).expect("copy Refloat package asset");
        println!("cargo::rerun-if-changed={}", source.display());
    }

    let package_name = read_trimmed(&refloat.join("package_name"));
    let package_name = package_name.chars().take(20).collect::<String>();
    let version = read_trimmed(&refloat.join("version"));
    let template =
        std::fs::read_to_string(refloat.join("ui.qml.in")).expect("read Refloat QML template");
    let qml = template
        .replace("{{PACKAGE_NAME}}", &package_name)
        .replace("{{VERSION}}", &version);
    let mut minifier = Command::new(refloat.join("rjsmin.py"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start Refloat QML minifier");
    minifier
        .stdin
        .as_mut()
        .expect("open QML minifier stdin")
        .write_all(qml.as_bytes())
        .expect("write Refloat QML");
    let qml = minifier
        .wait_with_output()
        .expect("wait for Refloat QML minifier");
    assert!(qml.status.success(), "Refloat QML minifier failed");
    std::fs::write(output.join("ui.qml"), qml.stdout).expect("write Refloat QML");

    for source in ["package_name", "version", "ui.qml.in", "rjsmin.py"] {
        println!("cargo::rerun-if-changed={}", refloat.join(source).display());
    }
}

fn read_trimmed(path: &Path) -> String {
    std::fs::read_to_string(path)
        .expect("read Refloat package metadata")
        .trim()
        .to_owned()
}
