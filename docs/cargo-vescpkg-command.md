# Cargo VescPkg Command

This note captures the intended Cargo entrypoint for the Rust-backed VESC
package flow.

This is an unofficial Cargo subcommand for Rust VESC package experiments; it is
not an official VESC project or endorsed command.

## Contract

- The command surface should stay thin.
- The shared implementation should live in `crates/vescpkg-rs-build`.
- The command should build on the existing package plans rather than duplicating
  staging or artifact layout logic.
- Operator workflows live in `cargo-vescpkg`; package users invoke them through
  `cargo vescpkg`.
- The package target is the device-side BTLE loopback package, not a generic
  archive builder.
- The current checked workflow remains `nix develop -c make check`.

## Intended Shape

- `cargo vescpkg build`
- optional `cargo vescpkg build --package-only`
- optional `cargo vescpkg build --target thumbv7em-none-eabihf`
- optional `cargo vescpkg build --manifest <pkgdesc.qml>` to build a package
  from a staged VESC package descriptor
- optional `cargo vescpkg build --refloat-source <checkout>` to package Refloat
  sources from an explicit checkout
- optional `cargo vescpkg build --build-date <date>` and
  `--git-commit <rev>` to stamp reproducible package provenance
- `cargo vescpkg deploy <package.vescpkg>`
- `cargo vescpkg package-install <package.vescpkg>`
- `cargo vescpkg erase-package`
- `cargo vescpkg loopback`
- `cargo vescpkg lisp-probe`
- `cargo vescpkg refloat-probe --vesc-tool <path>` to run the Refloat package
  probe through an explicit VESC Tool CLI
- the repo prototype lives in the thin `crates/cargo-vescpkg` subcommand crate

## Responsibilities

- run the Rust build for the device crate when needed
- stage package assets
- emit the final `.vescpkg`
- keep descriptor, source-checkout, provenance, and VESC Tool path overrides
  explicit so scripted operator runs remain reproducible
- keep the device package wired to VESC BTLE on the firmware side
- preserve the Predictable artifact path under `target/vescpkg`
- keep the package-size guard and symbol checks in the workspace gates
- own the host/operator command implementation directly, without a separate
  legacy CLI crate

## Non-Goals

- do not reimplement VESC Tool packaging behavior in a second place
- do not hide the package layout or target assumptions inside ad hoc shell glue
- do not move the device payload out of the `no_std` crate

## Notes

- `xtask` remains a fallback shape if Cargo subcommand plumbing is too much for
  the first version.
- The eventual command should remain a wrapper around the same package plan and
  artifact contract that the Makefile already exercises.
