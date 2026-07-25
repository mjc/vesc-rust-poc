# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

These values were regenerated from SDK commit `468cb66d` on the ARM32 package
path; they are an artifact baseline, not a claim of byte identity with
`origin/main`.

| artifact | bytes | SHA-256 |
| --- | ---: | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | 2,942 | `640640541ac2210e9b3ca86939d7ca1d9ecd6274a3f0621baa37343a19ce309c` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | 4,227 | `6ff9fa6db8af00284aaa2b474ead62ba171cc7a92c6c17cec2b7667b3b43f4d4` |
| `Rust-control-loop-smoke-package-0.1.0.vescpkg` | 3,479 | `abac826d2b31852dde2468bb522843916ee996d0e5e77c372681171ffd58a67b` |
| `Float-Out-Boy-0.1.0.vescpkg` | 101,076 | `f9f9b57fe36793e36579d91e799a472f4d8c9b38deaa1f31a6dd2bfc23f9bc70` |
| loopback `src/package_lib.bin` | 3,148 | `1e4ff87ade57dede9cb63b543241ab8204c75f0e59f55f2f56bb216636656be2` |
| alloc-smoke `src/package_lib.bin` | 4,968 | `09432f96bc013f270661f85fa888ffc3b493bac038dcac0e262dad928d3a2fd6` |
| control-loop `src/package_lib.bin` | 3,817 | `3cb48cd42c72102cd9acef99913d99a37fedd093f0f534d51816062720d56998` |
| Float Out Boy `src/package_lib.bin` | 78,424 | `e9d121d75e1b118e9e6e5148ca2bf888367ad9b67a865993956a9b982d6cbb5d` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. A historical hardware gate ran the
BLE deploy command for the loopback and alloc-smoke artifacts: install, start,
ping, echo, status, and teardown all succeeded, and the echo response was
`0102020908`. The alloc-smoke package performs the same sequence through its
allocator-backed app-data callback, so the probe exercises the allocation
instead of optimizing it away.

The hashes above are the current artifact-only proof after the direct Cargo
binary cutover. The hashes above do not claim a new device run. The ignored HIL workflow in
`crates/cargo-vescpkg/tests/hil_loopback.rs` remains the required current
device gate and needs `VESC_DEVICE` plus `VESC_BLE_ADDR` before it can be run.

Regenerate the complete representative set with:

```text
nix develop --command make package-examples
```
