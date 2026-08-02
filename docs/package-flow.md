# Rust VESC package flow

`cargo-vescpkg` is the user-facing Cargo command. Packages are ordinary Cargo
packages selected with `-p`; no package-specific host adapter participates in
the build.

## Package inputs

Each package owns:

- one Rust binary or static-library payload;
- `[package.metadata.vescpkg]` metadata;
- a small build script using `vescpkg-build-support`; and
- a recursive `package/` asset tree.

The reserved `src/package_lib.bin` archive path is always supplied by the
compiled native payload. Metadata may set the display name and fullscreen-QML
policy; if `package/pkgdesc.qml` declares the same policy, the values must
agree.

The [package UI and asset authoring reference](package-ui.md) documents the
QML, LispBM, description, descriptor, and default-generation contracts.

## Build pipeline

```console
$ cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

The command:

1. resolves the selected package with `cargo metadata`;
2. runs one target Cargo build with JSON diagnostics;
3. selects exactly one final binary or static-library artifact;
4. performs the package's final ARM link when the input is a static library;
5. checks the ELF machine, layout, load range, relocation safety, and size;
6. converts the linked ELF to the flat native payload;
7. stages package assets under Cargo's target directory;
8. assembles the `.vescpkg`; and
9. decodes the result through the same reader used by installation.

The build never writes generated assets into the source tree and never invokes
Make, VESC Tool, another source checkout, or a package-specific host process.
Its outputs remain inside the ignored Cargo target tree.

## Device boundary

`deploy` is build plus install. `package-install` begins at an existing archive.
Both validate the complete package before opening BLE, then stop LispBM,
replace package data, restore QML/native assets, and restart the installed
program. `loopback` and `control-loop` are separate probes of already installed
packages.

The full command list, selectors, side-effect classifications, and mutation
warnings live in the
[`cargo vescpkg` command reference](cargo-vescpkg-command.md).

## Repository checks

```console
$ make check
$ make check-full
```

`make check` covers formatting, host and target checks, lints, tests, and
documentation. `make check-full` adds representative ARM ELF and package
assembly gates. Hardware-in-the-loop deployment is separate because a green
host/package build cannot establish BLE, firmware, or physical-controller
behavior.
