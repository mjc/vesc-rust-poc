# Workspace Layout

This repo is organized around layered responsibilities:

- `crates/vesc-ffi/` contains the raw firmware ABI surface (`no_std`, unsafe table calls).
- `crates/vesc-package/` contains the safe wrapper (bindings, lifecycle, init, loopback runtime).
- `crates/vesc-ble-loopback/` contains the `no_std` BLE loopback package staticlib payload.
- `crates/vesc-protocol/` contains the shared wire types used by both host and device code.
- `crates/vesc-pkg-build/` owns package layout, staging, and build orchestration.
- `crates/vesc-host-cli/` owns the host-side command surface for discovery, control, and transport testing.
- `scripts/` holds small workspace-level helpers that are not part of a Rust crate.

Host-only dependencies stay inside `vesc-host-cli`, packaging dependencies stay inside `vesc-pkg-build`, and the device payload crates stay free of both.
