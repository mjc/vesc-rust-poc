# Loopback example package

Reference VESC native package built as a `thumbv7em-none-eabihf` staticlib.

This example links [`vescpkg`](../../crates/vescpkg) and produces
`libvesc_example_loopback.a`, which `vescpkg-build` embeds into the BLE loopback
`.vescpkg` artifact.

Build the staticlib:

```bash
nix develop -c cargo build -p vesc-example-loopback --release --target thumbv7em-none-eabihf
```

Run the full workspace checks (including symbol audit against this artifact):

```bash
nix develop -c make check
```
