# Refloat Port Inventory

This note records the Refloat package contract to preserve while porting the
package onto `vescpkg-rs` and `cargo-vescpkg`.

## Source

- Refloat source: local checkout at `/Users/mjc/projects/refloat`.
- Contract version: tag `v1.2.1`.
- Tag commit: `0ef6e99d8701886feeb7fe6c07cc4ec53fb3d97a`.
- Tag subject: `Change version to 1.2.1`.
- The Refloat checkout has no local `flake.nix`; use the user devshell fallback
  when building it, expected under `/Users/mjc/cfg/devshells`.

Inspect Refloat through Git at the tag instead of relying on the checkout's
current detached `HEAD`:

```sh
git -C /Users/mjc/projects/refloat show v1.2.1:src/main.c
```

## Baseline Build Status

Baseline capture starts from a clean Git worktree at
`target/refloat-v1.2.1-src`:

```sh
git -C /Users/mjc/projects/refloat worktree add --detach \
  /Users/mjc/projects/vesc-rust-poc/target/refloat-v1.2.1-src v1.2.1
```

`/Users/mjc/cfg/devshells#refloat` still fails on Darwin while building
`compiler-rt-libc-18.1.8`, so this branch captures the baseline through the
repo Nix shell plus an ignored desktop VESC Tool build:

```sh
git -C /Users/mjc/projects/vesc_tool worktree add --detach \
  /Users/mjc/projects/vesc-rust-poc/target/vesc_tool_cli HEAD
nix develop /Users/mjc/cfg/devshells#vesc_tool -c sh -c \
  'qmake -config release "CONFIG += release_macos build_original exclude_fw" \
     QMAKE_CC=clang QMAKE_CXX=clang++ QMAKE_LINK=clang++ &&
   make -f Makefile -j$(sysctl -n hw.ncpu 2>/dev/null || echo 4)'
```

The local wrapper `target/refloat-tools/vesc_tool` points at
`target/vesc_tool_cli/build/macos/VESC Tool.app/Contents/MacOS/VESC Tool`.
That desktop binary serves both Refloat config generation
(`--xmlConfToCode`) and package-baseline generation (`--buildPkgFromDesc`).

## Build And Package Contract

Refloat `v1.2.1` is Make-owned:

- Top-level `Makefile` builds `refloat.vescpkg`.
- `VESC_TOOL ?= vesc_tool` is the package builder.
- Default packaging uses `$(VESC_TOOL) --buildPkgFromDesc pkgdesc.qml`.
- `OLDVT=1` falls back to the older colon-separated `--buildPkg` invocation.
- `MINIFY_QML=1` pipes generated `ui.qml` through `rjsmin.py`; `MINIFY_QML=0`
  preserves QML text.
- `package_README-gen.md` is generated from `package_README.md`, `version`, build
  date, and Git commit.
- `ui.qml` is generated from `ui.qml.in`, `package_name`, and `version`.

`src/Makefile` owns the native package library:

- Target name is `package_lib`.
- Refloat sources are all `src/*.c` plus `src/lib/*.c`.
- `conf/settings.xml` is converted through `$(VESC_TOOL) --xmlConfToCode`.
- Generated config files are `conf/conf_default.h`, `conf/confparser.h`,
  `conf/confxml.h`, `conf/confparser.c`, and `conf/confxml.c`.
- `conf/conf_general.h` is generated from package name, version, version parts,
  suffix, and short Git hash.
- The native build includes `vesc_pkg_lib/rules.mk`.
- Refloat adds `-MMD -flto` and link-time `-flto`.

`vesc_pkg_lib/rules.mk` owns the C native image shape:

- Compiler/linker tools are `arm-none-eabi-gcc`, `arm-none-eabi-objdump`, and
  `arm-none-eabi-objcopy`.
- Native code uses Cortex-M4 hard-float flags, `-fpic`, `-Os`, `-mthumb`,
  `-fdata-sections`, `-ffunction-sections`, and `-DIS_VESC_LIB`.
- Link flags include `-nostartfiles`, `-static`, `--gc-sections`,
  `--undefined=init`, and `-T vesc_pkg_lib/link.ld`.
- Outputs are `package_lib.elf`, `package_lib.list`, `package_lib.bin`, and
  generated Lisp bytes through `conv.py`.

Package metadata comes from `pkgdesc.qml`:

- Package name: `Refloat`.
- Description markdown: `package_README-gen.md`.
- Lisp loader: `lisp/package.lisp`.
- QML UI: `ui.qml`.
- Fullscreen QML: `false`.
- Output: `refloat.vescpkg`.
- Compatibility check accepts only VESC hardware type.

## Cargo VescPkg Refloat Flow

The Rust copy-through package path is:

```sh
nix develop -c cargo run -p cargo-vescpkg -- build \
  --refloat-source target/refloat-v1.2.1-src \
  --build-date '2026-07-02 06:00:00-06:00' \
  --git-commit 0ef6e99 \
  --vesc-tool target/refloat-tools/vesc_tool
```

For Refloat source builds, `cargo-vescpkg` now:

1. Materializes `src/conf/conf_general.h` from Refloat package metadata.
2. Runs Refloat's native `make -C src` with the configured VESC Tool path, so
   upstream remains responsible for `settings.xml` conversion and
   `package_lib.bin`.
3. Materializes `package_README-gen.md` and minified `ui.qml`.
4. Packs `pkgdesc.qml`, README HTML/markdown, QML, and Lisp imports through the
   Rust package writer.

The VESC Tool package baseline is:

```sh
nix develop -c make -C target/refloat-v1.2.1-src \
  VESC_TOOL="$(pwd)/target/refloat-tools/vesc_tool"
```

Current copy-through proof compares:

- Rust package: `target/refloat-parity/refloat-rust-zlib.vescpkg`.
- VESC Tool package: `target/refloat-parity/refloat-vesctool.vescpkg`.
- `cmp` result: byte-identical.
- SHA-256:
  `e894e55ab12593743e1f1e20b82f4ffad534bb28b9f56a764b4119f9e5cbd487`.

## Baseline Artifact Inventory

The current repeatable Refloat `v1.2.1` baseline, generated from
`target/refloat-v1.2.1-src` with the commands above, has these file properties:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `refloat.vescpkg` | 97151 | `e894e55ab12593743e1f1e20b82f4ffad534bb28b9f56a764b4119f9e5cbd487` |
| `src/package_lib.elf` | 95808 | `00b731116bf6fecdd5ea75b3f9cf9f0e7683cf9c20bca1e3f549b03dff66f777` |
| `src/package_lib.bin` | 76928 | `a216efd83e9aa6ac308e56d41f46fb1e065eaca3a39ebd0f01da99b838b5903c` |
| `src/package_lib.lisp` | 384664 | `2aa82bb49a19a8fcef749990a716eb111fd9bed15d918605d7719d6b5d3078f8` |
| `package_README-gen.md` | 2359 | `4e68ac48523a42d4c1112f0b574a0fda5527f72fe4bba6306100e07d8d60ac0b` |
| `ui.qml` | 131388 | `f7ac80af2c1ce3e4800dab57593fc9c8e0941e00a33656e50d73f2be57e1db10` |
| `pkgdesc.qml` | 478 | `4d3c7e940d0b255631318991b3edf18b209f17b08b18c9db45d26d7546b672a4` |
| `src/conf/conf_general.h` | 1064 | `b2fd22b4e7e29dd6ac094e2d38723400ec799e8e24755b3e2f8651ac2e24abff` |
| `src/conf/conf_default.h` | 18296 | `5077cc91d21f4226a4fa00f12c1ec00191e49cda9ffdacfbfc36b8a686ea240d` |
| `src/conf/confparser.c` | 30856 | `5873cf3bc90c9080314159de3042ea2ffc932567bc16e387ad87e446d396d8ae` |
| `src/conf/confparser.h` | 527 | `df1e67f9c855523bd303f4a1972905b1ebdad97ce78f4592b95283d3a2ec973b` |
| `src/conf/confxml.c` | 157684 | `1b2fb65d52ad856049f592073eaae41ab9a6d5788e5e3949ccc2949bd86074e0` |
| `src/conf/confxml.h` | 275 | `f8cde76acd0774f4bf16b78c2abb0e387e6d9285139bc8fb6cfb2d41295d5a50` |

The baseline `.vescpkg` decompresses to `216086` bytes with SHA-256
`9c1927ed876fa829a37b275d3e356b87bd4873df1448aa220435e38bfac74d65` and
contains these fields in order:

| Field | Bytes | SHA-256 |
| --- | ---: | --- |
| `name` | 7 | `c2e06ee09397b2b034a6fd37001018bebbccb6b91a0e4a02aefe371f284bd886` |
| `description` | 2796 | `e6834af6fd592545d2682bf1fb82b4d22d16eaf3aa821f13fa18694032107519` |
| `description_md` | 2359 | `4e68ac48523a42d4c1112f0b574a0fda5527f72fe4bba6306100e07d8d60ac0b` |
| `lispData` | 78941 | `e556297f7a78b3a94801a366048d71a50465591c4542c6bfe11dd1483d929e72` |
| `qmlFile` | 131388 | `f7ac80af2c1ce3e4800dab57593fc9c8e0941e00a33656e50d73f2be57e1db10` |
| `pkgDescQml` | 478 | `4d3c7e940d0b255631318991b3edf18b209f17b08b18c9db45d26d7546b672a4` |
| `qmlIsFullscreen` | 1 | `6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d` |

`lispData` contains loader code of length `538`, SHA-256
`86ca5868154f0486e50846f67e58b3c63cd6391d9dcf2f8936aa8309ba0f6d40`, and
two imports:

| Import | Offset | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| `package-lib` | 576 | 76929 | `c307d369b7b1e1ea741f8b26cb3251bab5c4fdfd6315455975568ded1accb17c` |
| `bms` | 77508 | 1431 | `e8157ed96be407aea1418655dd8e661c3972ca06275278b819dcdf46119233ca` |

## Native Loader Contract

The linked native image keeps the VESC package loader layout:

- Memory origin is `0`, length `96k`.
- `.program_ptr` is first.
- `.init_fun` follows and contains `init`.
- `.data`, `.bss`, `.got`, and `.text` follow.
- `.text` includes `.text`, `.rodata`, and `.rodata.*`.
- The native payload is a flat binary produced from the linked ELF with gap fill
  `0x00`, not an ELF embedded into the package.

The Lisp loader imports the native payload with:

```lisp
(import "src/package_lib.bin" 'package-lib)
(load-native-lib package-lib)
```

## Binary-Comparison Target

The port should get as close as practical to Refloat `v1.2.1` binary output.
Use two comparison tiers:

1. Copy-through parity: package the exact Refloat-generated native payload and
   generated assets through `cargo-vescpkg`. The staged native `package_lib.bin`,
   generated Lisp import bytes, README/QML inputs, package metadata, and package
   field inventory should be byte-identical or have a documented package-writer
   reason for drift.
2. Rust-native parity: as Rust replaces C native code, compare the generated
   ELF/bin against Refloat for loader-visible invariants and small intentional
   differences. Expected non-identical areas include compiler codegen,
   translated function bodies, symbol layout, and any deliberately omitted
   Refloat-specific feature slice.

The first milestone is now complete for copy-through packaging: with Refloat's
own generated native payload and assets, the Rust package writer reproduces the
VESC Tool `v1.2.1` package bytes. Keep this artifact as the baseline while
Rust-owned pieces replace copied inputs.

## Runtime Lifecycle Contract

`src/main.c` defines the package lifecycle.

Startup:

- `INIT_FUN(lib_info *info)` calls `INIT_START`.
- Package state is allocated with `VESC_IF->malloc(sizeof(Data))`.
- Startup returns `false` on allocation failure.
- `data_init` zeroes state, reads config from EEPROM, sets default config when
  needed, initializes subsystem structs, and records odometer state.
- `info->stop_fun = stop`.
- `info->arg = d`, making package state available through firmware `ARG`.
- Refloat conditionally initializes the beeper IO pin.
- Refloat spawns two native threads:
  - `Refloat Main`, stack `1536`, entry `refloat_thd`.
  - `Refloat Aux`, stack `1024`, entry `aux_thd`.
- Startup returns `false` if either native thread cannot be spawned; the second
  failure requests termination of the main thread.
- Startup initializes footpad/LED state, then registers firmware callbacks and
  LispBM extensions.

Main thread:

- `refloat_thd` calls `configure(d)` once, then loops until
  `VESC_IF->should_terminate()`.
- It updates time, IMU, beeper, charging, motor data, remote input, tilt logic,
  footpad state, haptic feedback, state machine, motor control, LEDs, data
  recording, and related ride behavior.
- It sleeps using the configured loop period through `VESC_IF->sleep_us`.

Aux thread:

- `aux_thd` lowers priority when `thread_set_priority` is available.
- It loops until `VESC_IF->should_terminate()`.
- It updates LEDs, stores backup odometer data after idle distance changes,
  periodically refreshes motor config, and sleeps at LED refresh cadence.

Stop:

- Clears the IMU read callback.
- Clears the app-data handler.
- Clears custom config callbacks.
- Requests termination for aux and main native threads when present.
- Destroys LED state.
- Frees package state with `VESC_IF->free(d)`.

This means the Rust port needs first-class package state ownership, stop cleanup,
native-thread lifecycle, firmware allocator handles, callback registration, and
explicit failure paths for allocation/thread startup.

## Firmware API Surfaces Used

Refloat `v1.2.1` uses these VESC firmware surfaces directly:

- Allocation: `malloc`, `free`.
- Threading: `spawn`, `request_terminate`, `should_terminate`,
  `thread_set_priority`, `sleep_us`.
- Lifecycle/config: `conf_custom_add_config`, `conf_custom_clear_configs`,
  `read_eeprom_var`, `store_eeprom_var`, `store_backup_data`.
- IMU: `imu_set_read_callback`, `imu_startup_done`, `imu_get_pitch`,
  `imu_get_roll`, `imu_get_yaw`, `imu_get_gyro`, `imu_get_quaternions`.
- App data: `set_app_data_handler`, `send_app_data`.
- LispBM: `lbm_add_extension`, `lbm_dec_as_i32`, `lbm_dec_as_float`,
  `lbm_enc_sym_true`, `lbm_enc_sym_nil`.
- Motor/control telemetry and commands: current, duty, RPM/ERPM, speed,
  distance, odometer, battery level, amp-hours, watt-hours, FET/motor temps,
  motor faults, and timeout reset.
- IO: analog reads, pin mode/write, pad mode, PPM and remote input.
- Optional firmware slots are checked before use in at least some paths, for
  example `foc_play_tone`, `foc_get_id`, and `system_time_ticks`.

## App-Data Contract

Refloat app-data frames use package ID `101` and a command byte. Known command
IDs in `v1.2.1` include:

| Command | ID | Purpose |
| --- | ---: | --- |
| `COMMAND_INFO` | 0 | version/package info |
| `COMMAND_GET_RTDATA` | 1 | realtime data |
| `COMMAND_RT_TUNE` | 2 | runtime tuning without EEPROM write |
| `COMMAND_TUNE_DEFAULTS` | 3 | reset tune defaults without EEPROM write |
| `COMMAND_CFG_SAVE` | 4 | save config to EEPROM |
| `COMMAND_CFG_RESTORE` | 5 | restore config from EEPROM |
| `COMMAND_TUNE_OTHER` | 6 | runtime startup/config changes |
| `COMMAND_RC_MOVE` | 7 | idle motor movement |
| `COMMAND_BOOSTER` | 8 | booster settings |
| `COMMAND_PRINT_INFO` | 9 | verbose info |
| `COMMAND_GET_ALLDATA` | 10 | compact all-data response |
| `COMMAND_EXPERIMENT` | 11 | testing/tuning command |
| `COMMAND_LOCK` | 12 | lock/disable state |
| `COMMAND_HANDTEST` | 13 | hand-test mode |
| `COMMAND_TUNE_TILT` | 14 | tilt tuning |
| `COMMAND_LIGHTS_CONTROL` | 20 | lights control |
| `COMMAND_FLYWHEEL` | 22 | flywheel toggle |
| `COMMAND_REALTIME_DATA` | 31 | realtime data path |
| `COMMAND_REALTIME_DATA_IDS` | 32 | realtime data ID list |
| `COMMAND_ALERTS_LIST` | 35 | alert list |
| `COMMAND_ALERTS_CONTROL` | 36 | alert control |
| `COMMAND_DATA_RECORD_REQUEST` | 41 | data recorder request |

The handler also routes LCM and charging commands from their module headers.
Commands above `200` are explicitly unstable in the source comment.

## Lisp Contract

`lisp/package.lisp` performs the package startup sequence after native load:

- Reads firmware version through `(sysinfo 'fw-ver)`.
- Calls `ext-set-fw-version` with the firmware version list.
- Calls `ext-bms` to decide whether BMS support is enabled.
- On firmware `7.x` or `6.05+`, imports and evaluates `lisp/bms.lisp`.
- Spawns `"Refloat BMS"` with stack/priority argument `50` and entry `bms-loop`.

Native extensions:

- `ext-set-fw-version` decodes three integer arguments into package state and
  returns Lisp true.
- `ext-bms` decodes six values into BMS state when BMS is enabled and returns
  true/nil for enabled state.

`bms-loop` sleeps `0.2` seconds per iteration and calls `ext-bms` with cell
voltage, cell temperature, BMS temperature, and message age values.

## Configuration Contract

Refloat has both generated C config code and firmware custom-config callbacks:

- `conf/settings.xml` is the authoritative config schema.
- `VESC_TOOL --xmlConfToCode conf/settings.xml` generates parser/default/XML C
  sources and headers.
- `conf/conf_general.h` injects package metadata constants.
- `get_cfg` serializes either defaults or current config.
- `set_cfg` deserializes config, rejects writes in special modes, prevents
  disabling the package while running, writes EEPROM, reconfigures runtime
  state, and refreshes LED config.
- `get_cfg_xml` returns generated XML bytes rebased with `PROG_ADDR`.
- EEPROM config storage uses fixed serialized length `320` bytes rounded up to
  32-bit words.

## Rust-Native Hardware Parity Gaps

Current hardware evidence says the Rust-owned Refloat package can corrupt
firmware/persistent controller state in a way the official `v1.2.1` package does
not. Treat green host tests and matching QML/package metadata as insufficient
until these native-behavior gaps are closed or intentionally ruled out:

- Native init is loader-only containment, not Refloat parity. Official Refloat
  allocates `Data`, initializes EEPROM-backed config/state, stores `info->arg`,
  installs `stop_fun`, starts `Refloat Main` and `Refloat Aux`, and registers
  IMU/custom-config/app-data/LispBM callbacks in `src/main.c:2419-2461`. The
  Rust candidate currently only registers the two loader LispBM extensions from
  the tail of that sequence (`src/main.c:2458-2459`).
- App UI/config is not proven by package metadata or QML bytes. Official Refloat
  exposes VESC Tool config through `get_cfg`, `set_cfg`, and `get_cfg_xml` in
  `src/main.c:2334-2395`, then registers those callbacks with
  `conf_custom_add_config` at `src/main.c:2456`. The Rust raw ABI mirrors
  `conf_custom_add_config`/`conf_custom_clear_configs`, but there is no
  package-author wrapper or Refloat init call yet.
- Float Control/app-data connection is not Refloat parity unless the official
  dispatcher is actually installed with compatible state. Official Refloat
  registers `on_command_received` at `src/main.c:2457`; that dispatcher handles
  command routing in `src/main.c:2143-2295` through the same `Data *` that custom
  config, BMS, threads, and stop cleanup use.
- The current Rust app-data path models a narrow all-data snapshot, not the
  official shared `Data` lifecycle. Official `data_init` zeroes state, reads
  config from EEPROM, initializes subsystems, and records odometer state in
  `src/main.c:1190-1235`. `RefloatAllDataPayloads::source_startup()` is only a
  test/default model.
- Stop cleanup must be audited before reconnecting callbacks. Official `stop`
  clears IMU, app-data, and custom config callbacks, terminates both native
  threads, destroys LED state, and frees `Data` in `src/main.c:2398-2412`.
  Leaving any global firmware callback pointed at unloaded Rust code is a
  plausible device-wedging failure mode.
- Loader extensions currently acknowledge loader calls but do not update
  official state. Official `ext_set_fw_version` stores firmware version into
  `Data` (`src/main.c:2305-2312`), and `ext_bms` returns
  `d->float_conf.bms.enabled` while optionally storing BMS telemetry
  (`src/main.c:2319-2331`). Official `lisp/package.lisp:4-17` calls these
  immediately after native load and conditionally starts the BMS Lisp thread.
- Allocator slot codegen is not the current suspect, but it is also not
  exercised by the current Rust Refloat artifact. The official native import in
  `target/refloat-1.2.1-upstream.vescpkg` calls firmware malloc during init as
  `ldr.w r3, [r5, #184]`, `movw r0, #2120`, `blx r3`, matching
  `VESC_IF->malloc(sizeof(Data))` at `src/main.c:2419`. The Rust
  `vescpkg-rs-sys::raw::vesc_malloc` ARM assembly loads the same table base and
  slot (`0x1000f800`, `#184`) and tail-calls it with the caller's `r0`; it adds
  a null-slot guard. The current loader-only Rust Refloat package links no
  malloc/free/app-data allocation path at all.

## Rust Ownership Map

| Refloat behavior | Rust owner |
| --- | --- |
| `.vescpkg` staging, metadata, generated README/QML inputs, artifact inspection | `vescpkg-rs-build` and `cargo-vescpkg` |
| External command UX for Refloat-shaped package builds/install/proof | `cargo-vescpkg` |
| Native image layout, linker script parity, flat binary conversion, symbol/runtime audits | `vescpkg-rs-build` |
| Raw VESC firmware ABI slots and unsafe table calls | `vescpkg-rs-sys` |
| Firmware allocation handles, lifecycle registration, app-data handler registration, LispBM extension registration, safe package-author wrappers | `vescpkg-rs` |
| Refloat ride state, balancing logic, command protocol, BMS policy, config schema, and QML semantics | Refloat-specific package crate/code |
| Physical/domain units and semantic wrappers for package-author APIs | `vescpkg-rs-units` plus `vescpkg-rs::types`, where reusable |

## Immediate Port Gaps

- Rust-native artifact proof: compare the repo-built
  `cargo vescpkg build --example refloat` package against the captured
  copy-through `v1.2.1` package baseline, documenting intentional native payload
  differences as the runtime port replaces C behavior.
- Runtime API: expose enough typed lifecycle, allocation, thread, callback,
  custom config, app-data, and LispBM extension support to port behavior without
  leaking raw ABI into package code.
- Config generation: decide whether `settings.xml` conversion remains delegated
  to VESC Tool for the first port slice or becomes an explicit Rust-side
  generated-input step later.
- App-data tests: preserve package ID, command IDs, response encodings, and
  known length checks with focused fixtures.
- Hardware proof: split final validation into BLE/GATT preflight, package
  erase/write/reload/install, native-load/Lisp extension proof, and
  Refloat-specific app-data/config smoke.
