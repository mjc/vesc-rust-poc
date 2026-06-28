# Rust Package API Roadmap

This is the short migration ladder for the Rust-backed VESC package experiment
in `crates/vesc-ble-loopback`. It stays deliberately narrow so later API work keeps
moving in the right direction instead of growing a too-clever wrapper too early.

## Current workspace shape

- `crates/vesc-ffi` — raw firmware ABI (`no_std`, unsafe table calls)
- `crates/vesc-package` — safe wrapper on top of `vesc-ffi`
- `crates/vesc-ble-loopback` — BLE loopback package staticlib payload
- `crates/vesc-pkg-build`
- `crates/vesc-protocol`
- `crates/vesc-host-cli`

## Validation

- `nix develop -c make check` — fast host tier (`nextest` default profile)
- `nix develop -c make check-full` — host tier plus embedded native-lib audits
- `nix develop -c make symbol-check` — embedded native-lib audit tier only
- `nix develop -c make package`

## Deferred: QEMU / hardware-in-the-loop

On-target validation (QEMU system emulation or a physical VESC on the bench) is intentionally
out of scope for the current test pyramid. Host-side checks cover artifact bytes, ELF layout,
symbol resolution, and disassembly patterns against device-proven fixtures; `vesc-host-cli`
covers install/loopback against real or fake BLE transports.

When a HIL lane is added later, it should run only the smoke probes that require firmware
(`lisp-probe`, loopback echo) and leave the heavy native-lib audits in the embedded CI tier.
No QEMU runner is wired in this repo yet.

## Current Rust-Owned Boundary

- Rust exports `prog_ptr` and `init` for the native loader.
- Rust owns LispBM extension table registration.
- Rust owns BLE app-data and stop-hook lifecycle setup.
- `vesc-pkg-build` still uses the generic VESC linker and conversion references:
  `src/vesc_c_if.h`, `src/link.ld`, `src/rules.mk`, and `scripts/conv.py`.

## Next Migration Ladder

1. Keep artifact, size, symbol, and ABI guards green under `nix develop -c make package`.
2. Hardware-validate install, `lisp-probe`, and `loopback` after each native boundary change.
3. Grow the safe wrapper crate (`vesc-package`) only where tests prove the ABI boundary is stable.
4. Grow `cargo vescpkg build` from the tested `vesc-pkg-build` boundary.
5. Replace generic VESC references only after tests prove byte/layout equivalence.

## Guardrail

Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely.

Keep the package code `no_std` and no-alloc, keep the unsafe surface small, and
move one capability at a time with tests first.
