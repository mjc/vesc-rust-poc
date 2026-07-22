# Loopback example package

Reference VESC package built as a Cargo-owned `thumbv7em-none-eabihf` ELF.
This example is unofficial and is not an official VESC package.

This example links [`vescpkg-rs`](../../crates/vescpkg-rs). Cargo links the
package library and package entrypoint into the final ELF.
`cargo-vescpkg` discovers that ELF from Cargo's JSON artifact stream and embeds
its binary payload into the BLE loopback `.vescpkg` artifact.

The package also includes usage-shaped public-API examples: a port of VESC's
official `examples/extension` `ext-test` callback plus a typed diagnostic
extension in `src/extensions.rs`, app-data transport in `src/app_data.rs`, an
official-shape custom application-data codec in `src/custom_data.rs`, an
explicit custom-EEPROM probe in `src/config.rs`, scoped synchronization and
clock reads, and a display-style GPIO bus plus bounded SSD1306 framebuffer in
`src/display.rs`. The framebuffer follows the page layout and clipping behavior
of the vendored official `examples/ssd1306` port. The EEPROM helper only writes
when its caller asks and never reaches into `vescpkg-rs-sys`.

Build the package ELF:

```bash
cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

Run the full workspace checks (including symbol audit against this artifact):

```bash
make check
```
