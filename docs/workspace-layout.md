# Workspace Layout

This repo is organized around layered responsibilities:

- This is an unofficial Rust workspace for VESC package experiments; it is not
  an official VESC project or endorsed package stack.
- `vescpkg-rs-*` crates are for code that runs inside a VESC package or builds
  `.vescpkg` artifacts. `vesc-*` names are reserved here for host/protocol
  communication surfaces, matching the broader Rust ecosystem naming split.
- `crates/vescpkg-rs-sys/` — raw firmware ABI (`no_std`, unsafe table calls). See [vescpkg-rs-sys testing](testing/vescpkg-rs-sys.md).
- `crates/vescpkg-rs/` — target-side SDK linked into native VESC packages.
- `crates/vesc-protocol/` — shared wire types for host and target.
- `crates/cargo-vescpkg/` — the `cargo vescpkg` command and its host-side
  `.vescpkg` format, build, install, and loopback support.
- `examples/loopback/` — reference BLE loopback package library plus Cargo-owned
  final ELF target.
- `scripts/` — small workspace helpers outside Rust crates.

Host-only dependencies stay in `cargo-vescpkg`. Target code stays in
`vescpkg-rs`, `vescpkg-rs-sys`, and examples. Host tools must not depend
on `vescpkg-rs` except when building examples.
