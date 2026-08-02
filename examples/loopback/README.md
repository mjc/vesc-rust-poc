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
explicit signature-checked custom-EEPROM probe image in `src/config.rs`, scoped
synchronization and clock reads, and a display-style GPIO bus plus bounded
SSD1306 framebuffer in `src/display.rs`. The framebuffer follows the page
layout and clipping behavior of upstream `c_libs/examples/ssd1306`; the source
mapping is recorded in the module documentation and the upstream source is not
vendored. The EEPROM helper only writes when its caller asks and never reaches
into `vescpkg-rs-sys`.

The package also exposes one fixed audio-smoke command through `src/audio.rs`.
After installing this example on a physically restrained controller, run:

```bash
cargo run -p cargo-vescpkg -- audio-beep --device "VESC BLE UART"
```

The command requests a short 440 Hz, 0.5 V FOC-audio beep, validates the typed
package response, and changes no stored configuration. It is physical motor
output, so keep the wheel clear and restore the prior package after the focused
check.

Build the package ELF:

```bash
cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

Run the full workspace checks (including symbol audit against this artifact):

```bash
make check
```
