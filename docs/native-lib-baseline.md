# Native Library Baseline

This fixture captures the smallest C package skeleton we want to keep around while the Rust path grows.

## Input Layout

- `src/package_lib.c`
- `src/vesc_c_if.h`
- `src/rules.mk`
- `src/link.ld`
- `src/conv.py`
- `package/code.lisp`
- `package/pkgdesc.qml`
- `package/README.md`

## Planned Outputs

- `target/native-lib-baseline/native_lib.elf`
- `target/native-lib-baseline/native_lib.bin`
- `target/native-lib-baseline/package_lib.bin`
- `target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg`

## Notes

- The fixture is intentionally small.
- The baseline test checks that the package skeleton files stay present.
- Build integration can land later without changing the fixture contract.
