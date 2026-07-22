# SDK compatibility matrix

This workspace is an unofficial Rust SDK for controller-resident VESC
packages. The matrix below records the supported proof paths and the limits of
the current STM32 package ABI; it is not a firmware-version promise.

| Surface | Evidence | Boundary |
| --- | --- | --- |
| STM32 function table | `vescpkg-rs-sys` derives 253 slots from the pinned `vesc_c_if.h`; the libclang audit checks field names, offsets, sizes, and callable/scalar shape | The pinned header is the source of truth |
| Older firmware / null tail | `VescIfPresence` preserves holes and scalar words; optional shims return absence or use a documented fallback | Required shims fail closed when a slot is missing |
| Host ABI and mocks | `cargo test -p vescpkg-rs-sys --lib` exercises independent present/absent mock slots and layout contracts | Host pointers are not treated as STM32 pointers |
| ARM/no-alloc sys crate | `cargo check -p vescpkg-rs-sys --target thumbv7em-none-eabihf --no-default-features` | Fixed firmware addresses and inline ARM dispatch remain unsafe internally |
| No-alloc package | `examples/loopback` and `examples/refloat` build without the `alloc` feature | APIs accept caller-owned buffers and slices |
| Allocator-enabled package | `examples/alloc-smoke` installs `VescAllocator` and is covered by the package build gate | Allocation is package-local and firmware-backed |
| GPIO inventory and leases | `gpio_lease` covers all 13 pinned digital enum values plus exclusive acquire/configure/read/release behavior; `stm32_pad` covers explicit raw pad resolution and mode/set/clear forwarding | Low-level STM32 pad access remains explicitly unsafe and needs hardware electrical proof |
| Clock/synchronization | Host tests cover tick rollover, RAII mutex/semaphore cleanup, and optional thread-priority absence | Firmware clock domains remain distinct; provider-specific stack proof is separate |
| IMU and package-owned AHRS | `imu` covers readiness, atomic roll/pitch/yaw and nine-field calibration snapshots; `ahrs` covers package-owned initial orientation plus normalized Mahony and Madgwick state | Firmware estimator snapshots remain separate from package-owned algorithms; calibration hardware proof is still open |
| Live firmware settings | `settings` covers typed float/int reads, checked writes, and explicit `store_cfg` persistence for the settings used by SDK telemetry | The typed list is intentionally bounded to source-backed settings; firmware rejection and hardware persistence remain observable boundaries |
| Persistent storage | `eeprom` covers word codecs, offset byte images, partial-word padding, interruption, and write failure; `nvm` covers bounded byte ranges, wipe, capacity, and firmware failures | EEPROM is the package custom range; NVM capability and capacity remain firmware-provided |
| Safe package API | Examples import `vescpkg_rs` only; raw sys calls stay behind crate-private bindings | Open-loop FOC, STM32 pads, variadic `printf`, and raw pointers stay unsafe/internal |
| LispBM arrays | The pinned LispBM contract exposes byte arrays; `LispValue::is_array` is intentionally the same capability as `is_byte_array` | No generic array-data borrow is exposed without a length-bearing ABI slot |
| VESC Express | Not implemented in this STM32 family | Express needs a separate table, types, loader, and target proof; it must not share STM32 slot order |

## Reproduce the matrix

From the repository root:

```text
nix develop --command cargo check --workspace
nix develop --command cargo test -p vescpkg-rs-sys --lib
nix develop --command cargo check -p vescpkg-rs-sys --target thumbv7em-none-eabihf --no-default-features
nix develop --command cargo nextest run -p vescpkg-rs --features "test-support math"
```

The broader package and artifact gates are exposed by `make check-full` and
`make package`. Hardware-in-the-loop deployment remains a separate, explicitly
requested gate; passing this matrix does not claim device validation.
