# Rust VESC package flow

`cargo-vescpkg` is the user-facing Cargo subcommand. Cargo packages are
ordinary inputs selected with `-p`; they are not host-side plugins.

## Build

```bash
nix develop -c cargo vescpkg build -p vesc-example-loopback
nix develop -c cargo vescpkg build -p vesc-example-alloc-smoke
```

Each package owns its target library, package metadata, package assets, and
small build script. Cargo links the library and package entrypoint into one
`thumbv7em-none-eabihf` ELF using `examples/vescpkg-link.ld`. The build script
copies package assets into Cargo's `OUT_DIR` and declares the package directory
as its rerun input.

`cargo-vescpkg` runs one Cargo build, consumes `compiler-artifact` and
`build-script-executed` JSON, converts the final ELF with `rust-objcopy`, and
assembles the `.vescpkg` archive under Cargo's target directory. It never writes
generated assets into the source tree and never invokes Make, a nested build,
the VESC Tool source tree, or a package-specific host adapter.

The target directory can be overridden normally:

```bash
nix develop -c sh -lc \
  'export CARGO_TARGET_DIR="$PWD/target/custom"; cargo vescpkg build -p vesc-example-loopback'
```

## Checks

- `nix develop -c make check` runs formatting, strict host checks, target checks,
  and workspace tests.
- `nix develop -c make check-full` also builds the package ELF and `.vescpkg`.
- `cargo nextest run -p cargo-vescpkg --profile hil -- --ignored` is the
  hardware lane and requires an attached VESC plus its device selection.

The generated package is decoded by the same package reader used by the install
path before BLE transport is opened. A successful hardware sign-off still
requires installing and probing both loopback and alloc-smoke packages.

## Deferred hardware tooling

`cargo-flash`/probe-rs are not part of this build path. VESC package deployment
uses a bespoke VESC transport and will be designed separately before adding
deployment tooling to the devshell.
