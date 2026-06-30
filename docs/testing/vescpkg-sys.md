# vescpkg-sys testing

Strategy for the `vescpkg-sys` crate: a hand-maintained, `no_std` firmware ABI mirror. Tests focus on **layout contracts**, **dispatch behavior** through injectable mock tables, and **guardrails** — not firmware HIL.

## Test pyramid

| Tier | What | Where |
|------|------|--------|
| Compile-fail | `unsafe` required, no `std` leak, crate-internal test harness | `tests/ui/`, trybuild |
| Layout / ABI pins | `LibInfo`, `VescIf` size/offsets, newtypes | `src/tests.rs` |
| Raw dispatch | mock `VescIf` + stub call recording | `src/raw/dispatch_tests.rs` |
| Header parity | `ffi-compare` vs `vesc_c_if.h` | `vescpkg-build` tests + optional local header fixtures |
| Thumb/asm smoke | `ldr` immediates vs `VescIfAbi` | `src/tests.rs` |

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
use vescpkg_sys::test_support::{empty_table, with_table};

let mut table = empty_table();
table.lbm_enc_i = Some(my_stub);
with_table(&table, || unsafe {
    // raw::* calls use `table` via `vesc_if()` / BASE_ADDR routing
});
```

Implementation: `crates/vescpkg-sys/src/test_support.rs`, compiled only for `vescpkg-sys`'s own tests with `#[cfg(test)]`.
It is not a downstream feature surface; the compile-fail tests intentionally prove external crates cannot import it.

Production ARM builds keep inline `asm!` dispatch; host/test builds use `Option<fn>` slots on the mock table.

## Boundary: what not to test here

| Layer | Role |
|-------|------|
| `vescpkg-sys` | Layout + raw dispatch |
| `vesc-sdk` | Safe bindings, lifecycle, extension semantics |
| `vescpkg-build` | Symbol audit, elf semantics, package pipeline |
| HIL / real VESC | Manual `vesc-cli` profiles |

## CI commands

| Command | Scope |
|---------|--------|
| `nix develop -c make check` | fmt, clippy, default nextest (includes vescpkg-sys unit + dispatch) |
| `nix develop -c make vescpkg-sys-target-check` | no normal deps + `thumbv7em-none-eabihf` check |
| `nix develop -c make native-audit` | package-only + native-lib tests |
| `nix develop -c make check-full` | check + ARM gates + native audit |

## Adding a new `raw::*` wrapper

1. Add field to `raw::VescIf` in header order (run header parity when available).
2. Add `VescIfAbi` slot to `USED_SLOTS`.
3. Extend `vesc_if_offsets_for_tests()` and layout tests.
4. Add dispatch tests (Some + None paths) using `test_support`.
5. Update this inventory table.

## Miri (optional)

Host dispatch tests can be run under Miri to exercise the crate-internal mock-table harness:

```bash
nix develop -c cargo +nightly miri test -p vescpkg-sys
```

Miri does not cover the ARM `asm!` dispatch path (`cfg(all(target_arch = "arm", not(test)))`). Treat Miri as a harness sanity check, not firmware validation.

Epic tracking: **br-uc4**.
