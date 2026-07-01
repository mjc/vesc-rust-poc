# Snake example package

Reference VESC native package for the Snake example.
This example is unofficial and is not an official VESC package.

The host-side terminal surface is `cargo vescpkg snake`. This package crate is
the device/package side of the same larger example and keeps game-state types
strongly typed so board dimensions, cells, ticks, and scores are not passed as
loose primitives.

Build the staticlib:

```bash
nix develop -c cargo build -p vesc-example-snake --release --target thumbv7em-none-eabihf
```

Run the full workspace checks:

```bash
nix develop -c make check
```
