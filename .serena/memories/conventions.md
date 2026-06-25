# Conventions

- Favor aggressive TDD: add or pin tests before implementation when behavior is changing.
- Keep Rust idiomatic and compact; prefer iterators and combinators over ad hoc loops when they improve clarity.
- Use small, reviewable slices and commit often.
- Keep the package builder logic centralized in `crates/vesc-pkg-build`; shared path/layout logic should not be duplicated in ad hoc scripts.
- Preserve the BLE loopback package identity and VESC BTLE wiring in docs, tests, and package plans.
- Maintain package-size guards and symbol-audit tests as first-class regressions.
- Prefer stable, durable abstractions over one-off glue; sub-crates are acceptable when they isolate package tooling cleanly.