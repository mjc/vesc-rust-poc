# `cargo vescpkg` command reference

`cargo-vescpkg` builds, installs, and probes Rust VESC packages. Run it from the
repository root in the development environment described by the root README:

```console
$ cargo run -p cargo-vescpkg -- --help
```

The command does not require or invoke VESC Tool. Device operations use the
VESC BLE UART transport directly.

## Device selection

Every device command accepts the same selectors:

- no selector: use the first discovered peripheral advertising the VESC BLE
  UART service or one of the loopback compatibility names;
- `--device <name>`: require a case-insensitive BLE local-name match;
- `--address <address>`: require a case-insensitive address match.

When both explicit selectors are present, `--address` wins. Prefer an explicit
selector whenever more than one controller is nearby.

## Command inventory

The classification names the most important controller-side boundary:

- **read-only** queries firmware or a running package without changing
  controller state;
- **package-mutating** installs, replaces, or erases controller package data;
- **firmware-config-mutating** changes live or stored firmware configuration;
- **package-specific** sends a protocol understood only by a particular
  installed package. Its payload determines whether it reads or mutates.

This table is checked against the live Clap subcommand list by a unit test.

| Command | Classification | What it does |
| --- | --- | --- |
| `build` | read-only on the controller; host output only | Build, link, validate, and assemble one Cargo package below `target/vescpkg/` |
| `loopback` | read-only, package-specific | Probe the installed loopback package and validate its response sequence |
| `custom-app-data` | package-specific | Send decimal payload bytes to the installed package and wait for app-data |
| `custom-config` | read-only | Fetch custom-config index 0 as raw bytes |
| `firmware-values` | read-only | Read odometer and uptime from selective setup values |
| `firmware-imu` | read-only or firmware-config-mutating | Read IMU gains, update them live, or update and store them |
| `fob-log` | read-only, package-specific | Capture bounded Float Out Boy command 31 responses to a host CSV |
| `lisp-stats` | read-only | Read LispBM CPU, heap, memory, stack, and result statistics |
| `control-loop` | read-only, package-specific | Probe the no-actuation control-loop package and report progress/timing |
| `control-loop-deploy` | package-mutating | Build and install a package, then run the control-loop probe |
| `package-install` | package-mutating | Decode, validate, and install an existing `.vescpkg` file |
| `erase-package` | package-mutating | Stop and erase installed package data |
| `deploy` | package-mutating | Build the selected Cargo package and install the resulting artifact |

## Host-only build

```console
$ cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

Build options:

- `-p, --package <package>` is required;
- `--manifest-path <path>` selects another Cargo manifest;
- `--target <triple>` defaults to `thumbv7em-none-eabihf`;
- `--profile <name>` defaults to `release`;
- `--features <list>` forwards Cargo's feature list unchanged.

The build consumes Cargo metadata and compiler-artifact JSON, links a single
binary or static-library payload, rejects unsupported absolute data
relocations, flattens the ARM ELF, stages the complete `package/` asset tree,
and validates the assembled archive. It writes only Cargo target output.

## Read-only device queries

These commands require a reachable controller but do not change its package or
firmware configuration:

```console
$ cargo run -p cargo-vescpkg -- custom-config --device "Floatwheel PintV"
$ cargo run -p cargo-vescpkg -- firmware-values --device "Floatwheel PintV"
$ cargo run -p cargo-vescpkg -- firmware-imu --device "Floatwheel PintV"
$ cargo run -p cargo-vescpkg -- lisp-stats --device "Floatwheel PintV"
```

Package-specific read-only probes:

```console
$ cargo run -p cargo-vescpkg -- loopback --device "VESC BLE UART"
$ cargo run -p cargo-vescpkg -- control-loop --device "VESC BLE UART"
$ cargo run -p cargo-vescpkg -- fob-log target/fob.csv --samples 100 --interval-ms 100 --device "Floatwheel PintV"
```

`fob-log` opens one session, requests Float Out Boy realtime command 31 for
each sample, validates the package/command prefix, and writes host elapsed
milliseconds plus the complete response bytes. It does not decode away unknown
future fields.

## Package-specific app-data

`custom-app-data` accepts one or more decimal bytes. Include the package ID and
command byte expected by the installed package:

```console
$ cargo run -p cargo-vescpkg -- custom-app-data 101,0,2,0 --device "Floatwheel PintV"
```

The example above is a read-only Float Out Boy info query. Other payloads can
change runtime tuning, persist configuration, control lights, or request motor
behavior.

> **Warning:** `custom-app-data` is package-specific and may mutate controller
> state. Inspect the installed package protocol before sending a payload.

Float Out Boy deliberately sends no app-data response for several successful
mutation commands, including runtime tune and config save/restore. This generic
command still waits for a response, so it can exit with a timeout after the
package consumed the mutation. A timeout alone proves neither success nor
failure. Use a documented readback, such as custom config, info, realtime, or
the package UI. See the
[Float Out Boy protocol](float-out-boy-protocol.md) for its response contract.

## Firmware IMU changes

Read current values by omitting mutation flags. Update the running firmware
configuration with:

```console
$ cargo run -p cargo-vescpkg -- firmware-imu --set-live 0.2 0.0 0.1 --device "Floatwheel PintV"
```

> **Warning:** `--set-live` changes the controller's live firmware IMU
> configuration. The values are not persisted without `--store`.

Persist the same change with:

```console
$ cargo run -p cargo-vescpkg -- firmware-imu --set-live 0.2 0.0 0.1 --store --device "Floatwheel PintV"
```

> **Warning:** `--store` writes the firmware application configuration. Back up
> the controller configuration and verify the intended gains before using it.

The command first reads the complete application configuration, replaces the
three IMU fields, writes the live or stored configuration through the matching
firmware command, then reads the values back.

## Install, deploy, and erase

Install an existing archive:

```console
$ cargo run -p cargo-vescpkg -- package-install target/vescpkg/Example.vescpkg --device "VESC BLE UART"
```

> **Warning:** `package-install` stops the running LispBM program and replaces
> package Lisp, QML, and native payload data.

Build and install in one command:

```console
$ cargo run -p cargo-vescpkg -- deploy -p vesc-example-loopback --device "VESC BLE UART"
```

> **Warning:** `deploy` performs the same controller mutation as
> `package-install` after building the selected package.

Build, install, then run the no-actuation control-loop probe:

```console
$ cargo run -p cargo-vescpkg -- control-loop-deploy -p vesc-example-control-loop-smoke --device "VESC BLE UART"
```

> **Warning:** `control-loop-deploy` replaces the installed package before
> probing it.

Erase the installed package:

```console
$ cargo run -p cargo-vescpkg -- erase-package --device "VESC BLE UART"
```

> **Warning:** `erase-package` removes installed package Lisp/QML/native data.
> Its normal preflight identifies the firmware and stops LispBM first.

`--no-preflight` skips firmware identification and uses the short-timeout
best-effort stop path before erasing. Use it only to recover a controller whose
normal preflight cannot complete.

## Hardware-in-the-loop test

The ignored HIL profile exercises a real control-loop package over BLE:

```console
$ VESC_DEVICE="Floatwheel PintV" VESC_BLE_ADDR="AA:BB:CC:DD:EE:FF" cargo nextest run -p cargo-vescpkg --features hil --profile hil -- --ignored
```

It is intentionally outside the default workspace gate. Building a package or
passing host tests does not establish controller behavior.
