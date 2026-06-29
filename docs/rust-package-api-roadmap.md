# Rust Package API Roadmap

This is the short migration ladder for the Rust-backed VESC package experiment
in `examples/loopback`. It stays deliberately narrow so later API work keeps
moving in the right direction instead of growing a too-clever wrapper too early.

## Current workspace shape

- `crates/vesc-ffi` — raw firmware ABI (`no_std`, unsafe table calls)
- `crates/vesc-sdk` — target-side SDK linked into native packages
- `examples/loopback` — BLE loopback reference package staticlib
- `crates/vesc-pkg` — host-side `.vescpkg` format/build/install
- `crates/vesc-protocol` — shared wire protocol types
- `crates/vesc-cli` — host CLI binary

## Validation

- `nix develop -c make check`
- `nix develop -c make check-full`
- `nix develop -c make hack-check` — `cargo-hack --each-feature` matrix for host crates plus thumb release lib build for `vesc-example-loopback`

## Deferred:

Hardware-in-the-loop validation is intentionally out of the default CI path.
Symbol resolution, and semantic instruction audits against device-proven fixtures; `vesc-cli`
exercises install/loopback against real hardware manually.

The ignored sketch lives in `crates/vesc-cli/tests/hil_loopback.rs` and is filtered by the
`hil` nextest profile.

## Current Rust-Owned Boundary

- Rust exports `prog_ptr` and `init` for the native loader.
- Rust owns LispBM extension table registration.
- Rust owns BLE app-data and stop-hook lifecycle setup.
- `vesc-pkg` still uses the generic VESC linker and conversion references:
  `src/vesc_c_if.h`, `src/link.ld`, `src/rules.mk`, and `scripts/conv.py`.

## Next Migration Ladder

1. Keep artifact, size, symbol, and ABI guards green under `nix develop -c make package`.
2. Hardware-validate install, `lisp-probe`, and `loopback` after each native boundary change.
3. Grow `vesc-sdk` only where tests prove the ABI boundary is stable.
4. Grow `cargo vescpkg build` from the tested `vesc-pkg` boundary.
5. Replace generic VESC references only after tests prove byte/layout equivalence.

## Guardrail

Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely.

Keep the package code `no_std` and no-alloc, keep the unsafe surface small, and
move one capability at a time with tests first.
