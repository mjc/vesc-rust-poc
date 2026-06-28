# BLE loopback golden fixtures (0.1.0)

Pinned byte-identical payloads for the Rust BLE loopback test package.

## Files

- `package_lib.bin` — staged native payload copied into `.vescpkg`
- `native_lib.bin` — flattened native image bytes
- `native_lib.elf` — linked ELF used for semantic snapshot tests
- `lisp_data.bin` — canonical `lispData` for empty provenance
- `fingerprints.toml` — FNV-1a 64-bit hex fingerprints for quick review

Integration tests embed these bytes with `include_bytes!` and compare derived
metadata with `insta` snapshots. Tests never rebuild native code.

Native refresh is explicit:

```bash
./scripts/update-golden-fixtures.sh
```

That runs `cargo run -p vesc-pkg --bin write-golden-fixtures` after a native build.

## Toolchain

Generate fixtures only from the Nix dev shell so gcc/rust versions stay pinned.
