# Workspace Layout

This repo is organized around layered responsibilities:

- `crates/vescpkg-sys/` — raw firmware ABI (`no_std`, unsafe table calls). See [vescpkg-sys testing](testing/vescpkg-sys.md).
- `crates/vesc-sdk/` — target-side SDK linked into native VESC packages.
- `crates/vesc-protocol/` — shared wire types for host and target.
- `crates/vescpkg-build/` — host-side `.vescpkg` format, build, and install.
- `crates/vesc-cli/` — host command-line tool (BLE, install, loopback).
- `examples/loopback/` — reference BLE loopback package staticlib.
- `scripts/` — small workspace helpers outside Rust crates.

Host-only dependencies stay in `vesc-cli` and `vescpkg-build`. Target code stays in
`vesc-sdk`, `vescpkg-sys`, and examples. Host tools must not depend on `vesc-sdk`
except when building examples.
