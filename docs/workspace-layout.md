# Workspace Layout

This repo is organized around layered responsibilities:

- `vescpkg-rs-*` crates are for code that runs inside a VESC package or builds
  `.vescpkg` artifacts. `vesc-*` names are reserved here for host/protocol
  communication surfaces, matching the broader Rust ecosystem naming split.
- `crates/vescpkg-rs-sys/` — raw firmware ABI (`no_std`, unsafe table calls). See [vescpkg-rs-sys testing](testing/vescpkg-rs-sys.md).
- `crates/vescpkg-rs/` — target-side SDK linked into native VESC packages.
- `crates/vesc-protocol/` — shared wire types for host and target.
- `crates/vescpkg-rs-build/` — host-side `.vescpkg` format, build, and install.
- `crates/vesc-cli/` — host command-line tool (BLE, install, loopback).
- `examples/loopback/` — reference BLE loopback package staticlib.
- `scripts/` — small workspace helpers outside Rust crates.

Host-only dependencies stay in `vesc-cli` and `vescpkg-rs-build`. Target code stays in
`vescpkg-rs`, `vescpkg-rs-sys`, and examples. Host tools must not depend on `vescpkg-rs`
except when building examples.
