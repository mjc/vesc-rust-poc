# Workspace Layout

This repo is organized around four responsibilities:

- `src/` contains the `no_std` device/package payload crate.
- `vesc-pkg-build/` owns package layout, staging, and build orchestration.
- `vesc-host-cli/` will own the host-side command surface for discovery, control, and transport testing.
- `vesc-protocol/` is the intended home for shared message types once the host/device contract is promoted out of sketches.

The first workspace slices should keep host-only dependencies inside `vesc-host-cli`, keep packaging dependencies inside `vesc-pkg-build`, and keep the device payload crate free of both.
