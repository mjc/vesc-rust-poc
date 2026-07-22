# vescpkg-rs-sys testing

Strategy for the `vescpkg-rs-sys` crate: a `no_std` firmware ABI layer generated from the pinned VESC package header with bindgen. Tests focus on **layout contracts**, **dispatch behavior** through injectable mock tables, and **guardrails** — not firmware HIL.

## Test pyramid

| Tier | What | Where |
|------|------|--------|
| Compile-fail | `unsafe` required, no `std` leak, crate-internal test harness | rustdoc contracts in `src/compile_fail_contracts.md` |
| Layout / ABI pins | `LibInfo`, `VescIf` size/offsets, newtypes | `src/tests.rs` |
| Raw dispatch | mock `VescIf` + stub call recording | `src/raw/dispatch_tests.rs` |
| Header parity | independent libclang audit of generated slots | `src/raw/abi_audit.rs` |
| Thumb/asm smoke | `ldr` offsets, optional branch, and indirect call vs generated manifest | `tools/thumb-dispatch-smoke.sh` |
| Manifest gate | callable/scalar presence semantics and raw-shim reachability | `src/tests.rs`, `raw.rs` |

## Public export inventory

| Export | Kind | `VescIfAbi` slot | Dispatch tests |
|--------|------|------------------|----------------|
| `raw::lbm_add_extension` | fn | `LBM_ADD_EXTENSION` | yes |
| `raw::lbm_add_extension_with_table_base` | fn | `LBM_ADD_EXTENSION` | yes |
| `raw::lbm_dec_as_i32` | fn | `LBM_DEC_AS_I32` | yes |
| `raw::lbm_enc_i` | fn | `LBM_ENC_I` | yes |
| `raw::lbm_is_number` | fn | `LBM_IS_NUMBER` | yes |
| `raw::lbm_enc_sym_eerror` | fn | `LBM_ENC_SYM_EERROR` (usize field) | yes |
| `raw::vesc_set_app_data_handler` | fn | `SET_APP_DATA_HANDLER` | yes |
| `raw::vesc_send_app_data` | fn | `SEND_APP_DATA` | yes |
| `raw::vesc_system_time_ticks` | fn | `SYSTEM_TIME_TICKS` | yes |
| `raw::io_set_mode` | fn | `IO_SET_MODE` | yes |
| `raw::io_write` | fn | `IO_WRITE` | yes |
| `raw::io_read` | fn | `IO_READ` | yes |
| `VescIfAbi` / `VescIfSlot` | types | all `USED_SLOTS` | offset tests |
| `LibInfo` / `LibInfoAbi` | types | loader header | yes |
| `NativeImage` | type | rebasing | yes |
| `views::*` | newtypes | — | size tests |
| `types::*` | newtypes | — | size tests |

## Mock-table harness

Host tests must not dereference `VescIfAbi::BASE_ADDR`. The harness installs a stack/static mock table:

```rust
use vescpkg_rs_sys::test_support::{empty_table, with_table};

let mut table = empty_table();
table.lbm_enc_i = Some(my_stub);
with_table(&table, || unsafe {
    // raw::* calls use `table` via `vesc_if()` / BASE_ADDR routing
});
```

Implementation: `crates/vescpkg-rs-sys/src/test_support.rs`, compiled only for `vescpkg-rs-sys`'s own tests with `#[cfg(test)]`.
It is not a downstream feature surface; the compile-fail tests intentionally prove external crates cannot import it.

Production ARM and host/test dispatch both preserve the header's nullable
function pointers. Required raw wrappers fail through one checked boundary;
optional capabilities propagate absence or use their documented fallback.

## Generated header inventory

The build script invokes bindgen against the vendored copy of the pinned
`vesc_c_if.h` using the firmware's `arm-none-eabi` target configuration. A
workspace build verifies that copy against the pinned `vesc_pkg_lib` submodule;
the vendored copy keeps `cargo package` and relocated builds self-contained.
The semantic slot manifest is derived from bindgen's generated `vesc_c_if`
definition, so field names, order, signatures, and ABI structs come directly
from the header.

Libclang is therefore a build-time dependency. The Nix development shell
supplies it and sets `LIBCLANG_PATH` for the normal build and test commands.

The complete host gate can be run directly with:

```bash
nix develop --command cargo test -p vescpkg-rs-sys --lib -- --test-threads=1
```

The serial flag keeps the process-global libclang fixture deterministic when
using Cargo's in-process test runner. `cargo nextest` remains the workspace
default because each test binary runs in its own process.

## Boundary: what not to test here

| Layer | Role |
|-------|------|
| `vescpkg-rs-sys` | Layout + raw dispatch |
| `vescpkg-rs` | Safe bindings, lifecycle, extension semantics |
| `cargo-vescpkg` | Cargo artifacts, native link, package pipeline |
| HIL / real VESC | Manual `cargo-vescpkg` profiles |

## CI commands

| Command | Scope |
|---------|--------|
| `make check` | fmt, clippy, default nextest (includes vescpkg-rs-sys unit + dispatch) |
| `make vescpkg-rs-sys-target-check` | no normal deps + `thumbv7em-none-eabihf` check |
| `make thumb-dispatch-smoke` | compile, lower, and inspect representative Thumb dispatch from the generated manifest |
| `make check-full` | check + ARM/package build gates |

## Adding a new `raw::*` wrapper

1. Update the pinned header, then regenerate through the normal bindgen build.
2. Add the raw wrapper and choose explicit required or optional behavior.
3. Add present/absent dispatch tests using `test_support`.
4. Update this inventory table.

The generated manifest also emits a compile-time callable-shim gate in
`raw.rs`; adding a callable manifest entry without a matching raw shim fails
the `vescpkg-rs-sys` build before dispatch tests run.

## Miri (optional)

Host dispatch tests can be run under Miri to exercise the crate-internal mock-table harness:

```bash
cargo +nightly miri test -p vescpkg-rs-sys
```

Miri does not cover the ARM `asm!` dispatch path (`cfg(all(target_arch = "arm", not(test)))`). Treat Miri as a harness sanity check, not firmware validation.

Epic tracking: **br-uc4**.
