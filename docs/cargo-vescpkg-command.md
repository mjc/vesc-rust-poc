# Cargo VescPkg Command

This note captures the intended long-term Cargo entrypoint for the Rust-backed
VESC package flow.

## Contract

- The command surface should stay thin.
- The shared implementation should live in `crates/vesc-pkg-build`.
- The command should build on the existing package plans rather than duplicating
  staging or artifact layout logic.
- The current checked workflow remains `nix develop -c make check`.

## Intended Shape

- `cargo vescpkg build`
- optional `cargo vescpkg build --package-only`
- optional `cargo vescpkg build --target thumbv7em-none-eabihf`

## Responsibilities

- run the Rust build for the device crate when needed
- stage package assets
- emit the final `.vescpkg`
- preserve the Predictable artifact path under `target/vescpkg`
- keep the package-size guard and symbol checks in the workspace gates

## Non-Goals

- do not reimplement VESC Tool packaging behavior in a second place
- do not hide the package layout or target assumptions inside ad hoc shell glue
- do not move the device payload out of the `no_std` crate

## Notes

- `xtask` remains a fallback shape if Cargo subcommand plumbing is too much for
  the first version.
- The eventual command should remain a wrapper around the same package plan and
  artifact contract that the Makefile already exercises.
