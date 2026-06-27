# BLE loopback golden fixtures (0.1.0)

Pinned byte-identical payloads for the Rust BLE loopback test package. These fixtures gate the device-visible native binary and `lispData` blob produced by `vesc-pkg-build`.

## Files

- `package_lib.bin` — canonical native payload copied from `target/native-lib-baseline/package_lib.bin`
- `lisp_data.bin` — canonical `lispData` for empty provenance (no git commit / build date in README)
- `fingerprints.toml` — FNV-1a 64-bit hex fingerprints for quick review

## Refresh

Only update after intentional FFI, linker, loader, or lisp packing changes. From the repo root inside `nix develop`:

```bash
./scripts/update-golden-fixtures.sh
```

Review the diff, update any pinned layout assertions in `package_format` tests if import offsets/sizes changed, then commit the fixture directory.

## Toolchain

Generate fixtures only from the Nix dev shell so gcc/rust versions stay pinned. Golden bytes will drift if the cross toolchain changes without refreshing these files.
