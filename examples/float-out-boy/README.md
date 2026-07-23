# Float Out Boy

Float Out Boy is an unofficial Rust port of [Refloat](https://github.com/lukash/refloat), built as a real-world example of the `vescpkg-rs` SDK and `cargo-vescpkg` packaging flow.

The port has a different name on purpose. It follows Refloat's behavior and package design, but it is a separate implementation; retaining the Refloat name would create confusion around releases, bugs, and support.

## Build and test

From the workspace root:

```console
$ nix develop
$ cargo nextest run -p vesc-example-float-out-boy --features test-support
$ cargo run -p cargo-vescpkg -- build -p vesc-example-float-out-boy
```

The finished artifact is written below `target/vescpkg/Float-Out-Boy-0.1.0/`.

## Layout

- `src/` contains the Rust package implementation and host-side tests.
- `package/` contains the QML, LispBM, and rider-facing package assets.
- [`package/README.md`](package/README.md) contains installation, safety, and configuration guidance shown with the packaged application.

## Upstream lineage

Refloat is authored by Lukáš Hrázký and builds on the original Float package by Mitch Lustig, Dado Mista, and Nico Aleman.

- [Refloat source](https://github.com/lukash/refloat)
- [Refloat releases](https://github.com/lukash/refloat/releases)
- [Refloat 1.2 release notes](https://pev.dev/t/refloat-version-1-2/2795)
