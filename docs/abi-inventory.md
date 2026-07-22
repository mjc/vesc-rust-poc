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

## LispBM SDK Boundaries

The safe LispBM surface follows the pinned `vesc_c_if.h` table:

- symbol lookup and error reasons accept `&CStr`; the firmware pointer is only
  borrowed for the duration of the call;
- scalar decoders expose firmware `i32`, `u32`, and `f32` slots. The SDK offers
  exact immediate `i32`/`u32`/`i64`/`u64` decoding plus widened numeric
  `i64`/`u64`/`f64` conversions for values representable by those slots;
  exact float decoding rejects immediate integers, and `f64` encoding is
  accepted only when it round-trips exactly through the firmware `f32` encoder;
- 64-bit values are constructed through `LispFlatValue::push_i64` and
  `push_u64`, because the pinned table has no direct scalar 64-bit slots;
- the header has no callable array-header/data accessor, so the SDK does not
  expose an unsound borrowed array slice.
- evaluation pause/continue and pause-state observation are exposed as named
  `LispProcess` controls; they retain the firmware's minimum-free-word input
  instead of inventing an allocator policy in Rust.

## Firmware AHRS Boundaries

The pinned `ATTITUDE_INFO` struct and its initializer/update/getter slots are
copied into the owned `FirmwareAhrsSnapshot` surface. The SDK owns the state
storage, applies checked gains because the C initializer leaves those fields
for the caller, and keeps the retained IMU read callback behind an exclusive
unsafe lease. The wrapper does not alias firmware-owned estimator storage.

Callback leases fail closed when an optional unregister/disable slot rejects
cleanup: ownership is retained rather than permitting a replacement callback
to race with provider-owned state.

## Notes

- This list is intentionally narrow.
- It is enough for the first `(ext-rust-add 1 2)` proof.
- Later BLE work should add only the symbols that the protocol and transport code actually need.
