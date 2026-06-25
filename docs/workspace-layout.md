# Workspace Layout

This repo is organized around four responsibilities:

- `src/` contains the `no_std` device/package payload crate.
- `vesc-protocol/` contains the shared wire types used by both host and device code.
- `vesc-pkg-build/` owns package layout, staging, and build orchestration.
- `vesc-host-cli/` will own the host-side command surface for discovery, control, and transport testing.

The first workspace slices should keep host-only dependencies inside `vesc-host-cli`, keep packaging dependencies inside `vesc-pkg-build`, and keep the device payload crate free of both.
