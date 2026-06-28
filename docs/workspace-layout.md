# Workspace Layout

This repo is organized around layered responsibilities:

- `crates/vesc-ffi/` — raw firmware ABI (`no_std`, unsafe table calls). See [vesc-ffi testing](testing/vesc-ffi.md).
- `crates/vesc-sdk/` — target-side SDK linked into native VESC packages.
- `crates/vesc-protocol/` — shared wire types for host and target.
- `crates/vesc-pkg/` — host-side `.vescpkg` format, build, and install.
- `crates/vesc-cli/` — host command-line tool (BLE, install, loopback).
- `examples/loopback/` — reference BLE loopback package staticlib.
- `scripts/` — small workspace helpers outside Rust crates.

Host-only dependencies stay in `vesc-cli` and `vesc-pkg`. Target code stays in
`vesc-sdk`, `vesc-ffi`, and examples. Host tools must not depend on `vesc-sdk`
except when building examples.
