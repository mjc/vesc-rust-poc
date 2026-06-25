# Core

- Rust workspace for a VESC package/host split.
- Top-level workspace members: `crates/vesc-rust-poc` (device-side `no_std` crate), `crates/vesc-host-cli`, `crates/vesc-pkg-build`, `crates/vesc-protocol`.
- Package work centers on the BLE loopback test package; packaging logic lives in `crates/vesc-pkg-build` and the final artifact path is rooted under `target/vescpkg`.
- Repo-local `flake.nix` is the canonical environment entrypoint; use `nix develop`.
- Keep package construction and artifact inspection centralized in Rust; avoid reintroducing shell-script packaging glue.
- Hardware/BTLE smoke is a separate follow-up when a real adapter/device is available.