# Rust VESC Package Flow

This note characterizes the package path that the Rust VESC package experiment should support.

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
- `make check` runs workspace tests, formatting, linting, the Rust archive symbol audit,
  and the package-size smoke test before packaging work moves forward.
- `make test`, `make fmt`, `make clippy`, `make symbol-check`, and `make package-smoke`
  stay available as smaller commands when a slice only needs one gate.
- `make package` runs the checked package build wrapper and emits the final `.vescpkg` path.
- `make package-only` skips the top-level `check` dependency for debugging the packaging wrapper itself.
- `make` currently defaults to `check`; the package-build command path lives in the repo now instead of an ad hoc shell fragment.
- `package_README-gen.md` and `ui.qml` are generated artifacts, not hand-edited inputs.

## Build Metadata

- The generated README appends version, build date, and git commit details.
- `ui.qml` is templated from `ui.qml.in` with package name and version substitutions.
- The package output name is the package name with a `.vescpkg` suffix.

## What This Means For Rust

- The Rust path should keep the same separation between package metadata and native payload generation.
- The first Rust proof should still let the VESC native-library flow own the final ELF/bin/conversion steps.
- Package staging, package asset rendering, artifact inspection, and VESC Tool invocation
  belong in the dedicated `vesc-pkg-build` crate rather than ad hoc shell fragments.
