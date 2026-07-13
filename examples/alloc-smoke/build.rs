#![allow(missing_docs)]

use std::path::Path;
use std::{env, fs};

fn main() {
    let out_dir = env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR");
    let assets = Path::new(&out_dir).join("vescpkg");
    fs::create_dir_all(assets.join("src")).expect("create package asset directory");
    for name in ["README.md", "pkgdesc.qml", "code.lisp"] {
        fs::copy(Path::new("package").join(name), assets.join(name)).expect("copy package asset");
    }
    for name in ["README.md", "pkgdesc.qml", "code.lisp"] {
        println!("cargo::rerun-if-changed=package/{name}");
    }

    if env::var_os("TARGET").as_deref() == Some("thumbv7em-none-eabihf".as_ref()) {
        let linker_script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vescpkg-link.ld");
        println!("cargo::rerun-if-changed={}", linker_script.display());
        for arg in [
            "-nostartfiles",
            "-static",
            "-mcpu=cortex-m4",
            "-mthumb",
            "-mfloat-abi=hard",
            "-mfpu=fpv4-sp-d16",
            "-Wl,--gc-sections",
            "-Wl,--undefined=init",
        ] {
            println!("cargo::rustc-link-arg={arg}");
        }
        println!("cargo::rustc-link-arg=-T{}", linker_script.display());
    }
}
