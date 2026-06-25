# Rust VESC Package Flow

This note characterizes the package path that the Rust VESC package experiment should support.

## Inputs

- `pkgdesc.qml` defines the package name, description markdown, Lisp loader, QML file, fullscreen flag, and final package output name.
- `fixtures/native-lib-baseline/package/code.lisp` loads `src/package_lib.bin` and registers Lisp-side behavior for the baseline package.
- `fixtures/native-lib-baseline/src/package_lib.c` and `fixtures/native-lib-baseline/src/rules.mk` model the native-library build flow that produces the Rust-backed binary payload.
- `lisp/bms.lisp` is imported by the package loader when BMS integration is enabled.
- `package_README.md`, `ui.qml.in`, `package_name`, and `version` feed generated package assets.

## Native Payload Path

- `crates/vesc-pkg-build` owns package staging, conversion, and inspection.
- `fixtures/native-lib-baseline/src/rules.mk` compiles the C sources into an ELF, converts it to a raw binary, and turns that binary into a Lisp-loadable asset.
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

## Build And Upload Workflow

1. Enter the Nix shell with `nix develop`.
2. Run `make check` before packaging any change.
3. Use `make package-only` to exercise staging, conversion, artifact inspection, and package path rendering without VESC Tool.
4. Use `make package` when `VESC_TOOL` or `vesc_tool` is available and you want the final `.vescpkg` emitted.
5. Upload the emitted package in VESC Tool, then run the host `loopback` command against the device-side package.

The current package name is `Rust BLE loopback test package`, and the predictable output path is
`target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg`.

## Device Assumptions

- The target firmware must expose the BLE loopback package entrypoint.
- The device must be able to stay connected long enough for the loopback exchange.
- The host and device should agree on the `vesc_protocol` wire version before any real hardware smoke test.

## Troubleshooting

- If `make package` fails because `vesc_tool` is missing, install VESC Tool or set `VESC_TOOL` to its binary path.
- If `make check` fails early, start with `make test` and `make symbol-check` to narrow the failure.
- If generated files drift, run `make clean` before rebuilding.
- If the host loopback fails with a scan timeout, the adapter or device was not discovered in time.
- If the host loopback fails with connect failure, retry the connection path before blaming the package.
- If the host loopback fails with a missing service error, the device-side package likely did not advertise the loopback service.

## Build Metadata

- The generated README appends version, build date, and git commit details.
- `VESC_PKG_GIT_COMMIT` and `VESC_PKG_BUILD_DATE` feed the package provenance fields when they are set.
- `ui.qml` is templated from `ui.qml.in` with package name and version substitutions.
- The package output name is the package name with a `.vescpkg` suffix.

## What This Means For Rust

- The Rust path should keep the same separation between package metadata and native payload generation.
- The first Rust proof should still let the VESC native-library flow own the final ELF/bin/conversion steps.
- Package staging, package asset rendering, artifact inspection, and VESC Tool invocation
  belong in the dedicated `vesc-pkg-build` crate rather than ad hoc shell fragments.
