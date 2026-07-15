use std::path::Path;

fn main() {
    let linker_script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../examples/vescpkg-link.ld")
        .canonicalize()
        .expect("workspace linker script");
    println!("cargo::rerun-if-changed={}", linker_script.display());
    for argument in [
        "-nostartfiles",
        "-static",
        "-mcpu=cortex-m4",
        "-mthumb",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        "-Wl,--emit-relocs",
        "-Wl,--gc-sections",
        "-Wl,--undefined=init",
    ] {
        println!("cargo::rustc-link-arg={argument}");
    }
    println!("cargo::rustc-link-arg=-T{}", linker_script.display());
}
