# Loopback example package

Reference VESC package built as a Cargo-owned `thumbv7em-none-eabihf` ELF.
This example is unofficial and is not an official VESC package.

This example links [`vescpkg-rs`](../../crates/vescpkg-rs) and produces
Cargo links the package library and the package entrypoint into the final ELF.
`cargo-vescpkg` discovers that ELF from Cargo's JSON artifact stream and embeds
its binary payload into the BLE loopback `.vescpkg` artifact.

Build the package ELF:

```bash
nix develop -c cargo vescpkg build -p vesc-example-loopback
```

Run the full workspace checks (including symbol audit against this artifact):

```bash
nix develop -c make check
```
