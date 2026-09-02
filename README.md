# vescpkg-rs

Unofficial Rust SDK and Cargo tooling for building native [VESC](https://vesc-project.com/) packages.

This workspace provides a `no_std` package-author API, raw firmware ABI bindings, shared protocol and unit types, build-script support, and a `cargo vescpkg` command that builds, assembles, installs, and probes `.vescpkg` artifacts.

The project is experimental, its APIs are not stable, and it is not affiliated with or endorsed by the VESC project.

## Nix development environment

Direnv loads the flake's Rust toolchain, ARM target, linker tools, and native
dependencies:

```console
$ direnv allow
```

All commands below assume that environment is active.

## Quick start

```console
$ make check
```

Build the full Float Out Boy example package:

```console
$ make package-only
```

Generated packages are written below `target/vescpkg/`.

## Building packages

`cargo-vescpkg` accepts ordinary Cargo package selections. A package owns its Rust binary, `[package.metadata.vescpkg]` metadata, build script, and `package/` asset tree.

```console
$ cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

The command performs the ARM build and final link, checks the resulting image, copies the package assets, and assembles the `.vescpkg` archive without modifying the source tree.

## Workspace

| Path | Purpose |
| --- | --- |
| `crates/vescpkg-rs` | Target-side `no_std` SDK with lifecycle, firmware, GPIO, LispBM extension, app-data, allocation, and typed helper APIs |
| `crates/vescpkg-rs-sys` | Raw `no_std` bindings to the VESC native-package firmware ABI |
| `crates/vescpkg-rs-units` | Reusable `no_std` physical-unit newtypes |
| `crates/vescpkg-build-support` | Shared Cargo build-script support for package assets and ARM linking |
| `crates/cargo-vescpkg` | Host-side `cargo vescpkg` build, install, deploy, erase, and loopback commands |
| `crates/vesc-protocol` | Shared `no_std` wire types used by host tools and package code |
| `examples` | Buildable packages that exercise the SDK and packaging flow |

## Examples

- `loopback` is the reference package and host probe for the BLE loopback path.
- `alloc-smoke` proves that Rust allocation can use the firmware allocator.
- [Float Out Boy](examples/float-out-boy/README.md) is a full self-balancing skateboard package and a Rust port of [Refloat](https://github.com/lukash/refloat). The port is intentionally renamed to prevent confusion with the original project.

## Device commands

The [`cargo vescpkg` command reference](docs/cargo-vescpkg-command.md) lists
every device command, BLE selector, controller-side effect, and mutation
warning. Device operations use the VESC BLE UART transport directly.

## Development

```console
$ make check
$ make check-full
```

More detail is available in the documentation for the
[workspace layout](docs/workspace-layout.md),
[package build flow](docs/package-flow.md),
[package UI and assets](docs/package-ui.md),
[`cargo vescpkg` command](docs/cargo-vescpkg-command.md), and
[SDK capabilities](docs/sdk-compatibility.md).
