# Workspace Layout

This repo is organized around layered responsibilities:

- `vescpkg-*` crates are for code that runs inside a VESC package or builds
  `.vescpkg` artifacts. `vesc-*` names are reserved here for host/protocol
  communication surfaces, matching the broader Rust ecosystem naming split.
- `crates/vescpkg-sys/` — raw firmware ABI (`no_std`, unsafe table calls). See [vescpkg-sys testing](testing/vescpkg-sys.md).
- `crates/vescpkg/` — target-side SDK linked into native VESC packages.
- `crates/vesc-protocol/` — shared wire types for host and target.
- `crates/vescpkg-build/` — host-side `.vescpkg` format, build, and install.
- `crates/vesc-cli/` — host command-line tool (BLE, install, loopback).
- `examples/loopback/` — reference BLE loopback package staticlib.
- `scripts/` — small workspace helpers outside Rust crates.

Host-only dependencies stay in `vesc-cli` and `vescpkg-build`. Target code stays in
`vescpkg`, `vescpkg-sys`, and examples. Host tools must not depend on `vescpkg`
except when building examples.
