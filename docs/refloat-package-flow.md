# Refloat Package Flow

This note characterizes the working Refloat VESC package path that the Rust POC should mirror.

## Inputs

- `pkgdesc.qml` defines the package name, description markdown, Lisp loader, QML file, fullscreen flag, and final package output name.
- `lisp/package.lisp` loads `src/package_lib.bin` and registers Lisp-side behavior.
- `lisp/bms.lisp` is imported by the package loader when BMS integration is enabled.
- `package_README.md`, `ui.qml.in`, `package_name`, and `version` feed generated package assets.

## Native Payload Path

- `src/Makefile` builds `src/package_lib.bin` from the VESC C package library rules.
- `vesc_pkg_lib/rules.mk` compiles the C sources into an ELF, converts it to a raw binary, and turns that binary into a Lisp-loadable asset.
- The native build stays in the VESC native-library flow; the package layer does not build the final payload directly.

## Package Assembly

- The root `Makefile` gates packaging behind tests.
- `make check` runs the host C test suite and QML behavior checks before packaging.
- `make` and `make package` both end in `vesc_tool --buildPkgFromDesc pkgdesc.qml` unless `OLDVT=1` is selected.
- `package_README-gen.md` and `ui.qml` are generated artifacts, not hand-edited inputs.

## Build Metadata

- The generated README appends version, build date, and git commit details.
- `ui.qml` is templated from `ui.qml.in` with package name and version substitutions.
- The package output name is `refloat.vescpkg`.

## What This Means For Rust

- The Rust path should keep the same separation between package metadata and native payload generation.
- The first Rust proof should still let the VESC native-library flow own the final ELF/bin/conversion steps.
- Package staging, artifact inspection, and VESC Tool invocation belong in a dedicated packaging support crate rather than ad hoc shell fragments.
