# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | SHA-256 |
| --- | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | `ae157bbdd8ba4cbd421432909c6eb17f630d09beb3d9127017b350c3bb92fe38` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | `e53d96842c473db0257162b8098ec31d45d0ca3c8792ff3400c4598a6e162982` |
| loopback `src/package_lib.bin` | `c91e68d73e1c2b1ca3ef22d47fa40c2e788e636d71fa9af6fc6d7960e93572a8` |
| alloc-smoke `src/package_lib.bin` | `158c3ffd23bec72d93ff09f43d79269e56ba59497221eeef2c36166600e092f2` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. The hardware gate was run with the
BLE deploy command for both artifacts: install, start, ping, echo, status, and
teardown all succeeded, and the echo response was `0102020908`. The alloc-smoke
package performs the same sequence through its allocator-backed app-data
callback, so the probe exercises the allocation instead of optimizing it away.
