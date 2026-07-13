# Rust Package API Roadmap

This is the short migration ladder for the Rust-backed VESC package experiment
in `examples/loopback`. It stays deliberately narrow so later API work keeps
moving in the right direction instead of growing a too-clever wrapper too early.
This workspace is unofficial and is not an official VESC project or endorsed
Rust package API.

## Current workspace shape

- `crates/vescpkg-rs-sys` — raw firmware ABI (`no_std`, unsafe table calls)
- `crates/vescpkg-rs` — target-side SDK linked into native packages
- `examples/loopback` — BLE loopback reference package ELF
- `crates/cargo-vescpkg` — `cargo vescpkg` host command surface for `.vescpkg`
  format/build/install
- `crates/vesc-protocol` — shared wire protocol types

## Validation

- `make check`
- `make check-full` — strict host checks, target checks, package
  ELF build, and `.vescpkg` emission

## Deferred:

Hardware-in-the-loop validation is intentionally out of the default CI path.
Symbol resolution, and semantic instruction audits against device-proven fixtures;
`cargo vescpkg` exercises install/loopback against real hardware manually.

The ignored sketch lives in `crates/cargo-vescpkg/tests/hil_loopback.rs` and is
filtered by the `hil` nextest profile.

## Current Rust-Owned Boundary

- Rust exports `prog_ptr` and `init` for the native loader.
- Rust owns LispBM extension table registration.
- Rust owns BLE app-data and stop-hook lifecycle setup.
- Cargo uses the checked-in package linker script and emits the final ELF;
  `cargo-vescpkg` only packages the Cargo artifact after the build.

## Package-Author API Surface

Package code running inside the controller should import the common surface with
`use vescpkg_rs::prelude::*;`. That prelude exposes lifecycle controllers,
binding traits, extension descriptors, protocol names, domain-specific
`vescpkg-rs::types`, and non-conflicting physical units. It does not re-export
the raw `ffi` module.

Raw ABI bools can remain in `vescpkg-rs-sys` and low-level binding traits.
Package-author APIs should translate firmware success/failure into named
results such as `AppDataHandlerRegistrationError` so call sites do not have to
remember firmware polarity.

## Next Migration Ladder

1. Keep artifact, size, symbol, and ABI guards green under `make package`.
2. Hardware-validate install and `loopback` after each native boundary change.
3. Grow `vescpkg-rs` only where tests prove the ABI boundary is stable.
4. Keep `cargo vescpkg build` driven by Cargo metadata and compiler artifacts.
5. Replace generic VESC references only after tests prove byte/layout equivalence.

## Guardrail

Do not dump all of `vesc_c_if.h` into an ergonomic-looking API prematurely.
Do not publish the package-author crate as `vesc`, `vesc-api`, or `vesc-comm`;
those names are host/controller communication territory in the Rust ecosystem.

Keep the package code `no_std` and no-alloc, keep the unsafe surface small, and
move one capability at a time with tests first.
