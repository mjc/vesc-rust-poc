# Rust Package API Roadmap

This is the short migration ladder for the Rust-backed VESC package experiment
in `crates/vesc-rust-poc`. It stays deliberately narrow so later API work keeps moving
in the right direction instead of growing a too-clever wrapper too early.

## Current workspace shape

- `crates/vesc-rust-poc`
- `crates/vesc-pkg-build`
- `crates/vesc-protocol`
- `crates/vesc-host-cli`

## Validation

- `nix develop -c make check`

## Migration ladder

1. Rust pure computation behind C shim.
2. Rust handles primitive logic while C decodes LispBM values.
3. Rust calls one VESC_IF function through the shim.
4. Rust receives a VESC_IF pointer/raw binding.
5. safe wrapper crate.
6. xtask.
7. eventual `cargo vescpkg build`.

## Guardrail

Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely.

The first releases should keep the C shim as the ABI adapter, keep the unsafe
surface small, and move one capability at a time.
