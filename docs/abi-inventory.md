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

## Notes

- This list is intentionally narrow.
- It is enough for the first `(ext-rust-add 1 2)` proof.
- Later BLE work should add only the symbols that the protocol and transport code actually need.
