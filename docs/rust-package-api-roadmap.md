# Rust Package API Roadmap

This is the short migration ladder for the Rust-backed VESC package experiment
in `crates/vesc-rust-poc`. It stays deliberately narrow so later API work keeps
moving in the right direction instead of growing a too-clever wrapper too early.

## Current workspace shape

- `crates/vesc-rust-poc`
- `crates/vesc-pkg-build`
- `crates/vesc-protocol`
- `crates/vesc-host-cli`

## Validation

- `nix develop -c make check`
- `nix develop -c make package`

## Current Rust-Owned Boundary

- Rust exports `prog_ptr` and `init` for the native loader.
- Rust owns LispBM extension table registration.
- Rust owns BLE app-data and stop-hook lifecycle setup.
- `vesc-pkg-build` still uses the generic VESC linker and conversion references:
  `src/vesc_c_if.h`, `src/link.ld`, `src/rules.mk`, and `scripts/conv.py`.

## Next Migration Ladder

1. Keep artifact, size, symbol, and ABI guards green under `nix develop -c make package`.
2. Hardware-validate install, `lisp-probe`, and `loopback` after each native boundary change.
3. Extract a safe wrapper crate around the small unsafe ABI surface.
4. Grow `cargo vescpkg build` from the tested `vesc-pkg-build` boundary.
5. Replace generic VESC references only after tests prove byte/layout equivalence.

## Guardrail

Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely.

Keep the package code `no_std` and no-alloc, keep the unsafe surface small, and
move one capability at a time with tests first.
