# Rust VESC Package Flow

This note characterizes the package path that the Rust VESC package experiment should support.

## Inputs

- `pkgdesc.qml` defines the package name, description markdown, Lisp loader, QML file, fullscreen flag, and final package output name.
- `fixtures/native-lib-baseline/package/code.lisp` loads `src/package_lib.bin` and registers Lisp-side behavior for the baseline package.
- `target/thumbv7em-none-eabihf/release/libvesc_ble_loopback.a` and the generic VESC references in `fixtures/native-lib-baseline/src/` model the native-library build flow that produces the Rust-backed binary payload.
- `lisp/bms.lisp` is imported by the package loader when BMS integration is enabled.
- `package_README.md`, `ui.qml.in`, `package_name`, and `version` feed generated package assets.

## Native Payload Path

- `crates/vesc-pkg-build` owns package staging, conversion, and inspection.
- `fixtures/native-lib-baseline/src/rules.mk` and `scripts/conv.py` are placeholder references retained for VESC layout parity; the Rust build path compiles via `native_lib_materialize` and copies `native_lib.bin` to `package_lib.bin` without invoking them.
- The native build stays in the VESC native-library flow; the package layer does not build the final payload directly.

## Package Assembly

- The root `Makefile` gates packaging behind tests.
- `make check` runs formatting, linting, and the fast host test tier (`nextest` default profile).
- `make check-full` adds the embedded native-lib audit tier (`make symbol-check`) on top of `check`.
- `make symbol-check` runs the embedded native-lib integration audit (`tests/native_lib_artifacts.rs`).
- `make test`, `make fmt`, `make clippy`, `make test-embedded`, `make test-package`, and `make package-smoke`
  stay available as smaller commands when a slice only needs one gate.
- `make test-package` runs the package tier integration tests (`tests/fixtures.rs` and `tests/package_pipeline.rs`) via the nextest `package` profile.
- `make package-smoke` is an alias for `make test-package`.
- `make package` runs the checked package build wrapper and emits the final `.vescpkg` path.
- `make package-only` skips the top-level `check` dependency for debugging the packaging wrapper itself.
- `make` currently defaults to `check`; the package-build command path lives in the repo now instead of an ad hoc shell fragment.
- `package_README-gen.md` and `ui.qml` are generated artifacts, not hand-edited inputs.

## Build And Upload Workflow

1. Enter the Nix shell with `nix develop`.
2. Run `make check-full` before packaging native-boundary changes; `make check` is enough for host-only edits.
3. Use `make package-only` to exercise staging, conversion, package emission, and artifact inspection.
4. Use `make package` for the checked path that still emits the final `.vescpkg` from the local Rust packer.
5. Upload the emitted package in VESC Tool, then run the host `loopback` command against the device-side package.

The current package name is `Rust BLE loopback test package`, and the predictable output path is
`target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg`.

## Device Assumptions

- The target firmware must expose the BLE loopback package entrypoint.
- The device must be able to stay connected long enough for the loopback exchange.
- The host and device should agree on the `vesc_protocol` wire version before any real hardware smoke test.

## Troubleshooting

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
- The first Rust proof should keep the native-library flow for ELF/bin generation and let the Rust packer own the final `.vescpkg` emission.
- Package staging, package asset rendering, artifact inspection, and final package emission
  belong in the dedicated `vesc-pkg-build` crate rather than ad hoc shell fragments.
