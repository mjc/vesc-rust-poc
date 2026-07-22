# Minimal VESC C ABI Inventory

This inventory records the narrow C surface needed for the first Rust-backed test package.

## Required Items

- `INIT_FUN`: package loader entry point.
- `INIT_START`: registration prologue macro.
- `INIT_END`: registration epilogue macro.
- `lib_info`: package metadata type passed into the entry point.
- `lbm_add_extension`: LispBM extension registration function.
- `lbm_value`: LispBM value type used by the extension boundary.
- `lbm_uint`: argument-count type used by the extension boundary.
- `lbm_dec_as_i32`: LispBM integer decode helper.
- `lbm_enc_i`: LispBM integer encode helper.
- `ENC_SYM_EERROR`: LispBM error symbol for bad arity.

## Optional Flat Values

`lbm_start_flatten`, `lbm_finish_flatten`, the `f_*` constructors, and
`lbm_unblock_ctx` are optional function-table entries. The Rust
`LispFlatValue` wrapper probes those slots and returns `None`/`Rejected` when a
table does not expose them. A successful `LispProcess::unblock_flat` transfers
the firmware buffer to LispBM, while a dropped or rejected value releases it
through the firmware allocator.

## Notes

- This list is intentionally narrow.
- It is enough for the first `(ext-rust-add 1 2)` proof.
- Later BLE work should add only the symbols that the protocol and transport code actually need.
