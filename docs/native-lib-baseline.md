# Native Library Baseline

This fixture captures the smallest C package skeleton we want to keep around while the Rust path grows.

## Input Layout

- `src/package_lib.c`
- `src/vesc_c_if.h`
- `src/rules.mk`
- `src/link.ld`
- `scripts/conv.py`
- `package/code.lisp`
- `package/pkgdesc.qml`
- `package/README.md`

## Planned Outputs

- `target/native-lib-baseline/native_lib.elf`
- `target/native-lib-baseline/native_lib.bin`
- `target/native-lib-baseline/package_lib.bin`
- `target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg`

## Notes

- The fixture is intentionally small.
- The baseline test checks that the package skeleton files stay present.
- The package payload smoke test keeps the fixture under a 16 KiB budget, leaving
  ample headroom below the 128 KiB VESC Tool flash block limit.
- The next link step should combine `target/native-lib-baseline/package_lib.o` with
  `target/thumbv7em-none-eabihf/release/libvesc_rust_poc.a` to produce
  `target/native-lib-baseline/native_lib.elf`.
- A symbol audit now inspects the Rust archive and flags unexpected unresolved
  externals before the final native-library link step grows out.
- The same audit also checks the relocatable final ELF built from the C shim plus
  the Rust staticlib.
- The fixture pins `src/vesc_c_if.h` with fingerprint `a8980de23614d274`; if
  that header changes, refresh the expected fingerprint in
  `crates/vesc-pkg-build/src/native_lib_baseline.rs` after reviewing the ABI diff.
- The package loader fixture loads `src/package_lib.bin` for the BLE loopback test package.
- Build integration can land later without changing the fixture contract.
