# Tech Stack

- Rust workspace, edition 2021.
- Device crate is `no_std`; release/profile builds abort on panic.
- Workspace targets the VESC MCU toolchain (`thumbv7em-none-eabihf`) for the device payload.
- Nix is the supported development environment; `nix develop` is the preferred entrypoint.
- `make` drives the checked workflow and package targets.
- `arm-none-eabi-*` tooling is used for the native-lib/package conversion path.
- Keep packaging logic in `crates/vesc-pkg-build`; `cargo` subcommand plumbing is a later convenience layer.