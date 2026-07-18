# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | SHA-256 |
| --- | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | `36a3106194ff948dca7127c523e754925cbb0505b1b783c18d9a0427bd093e57` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | `f32f6621f505fa4e3660e16f4ad520afa86bb7c2155fd261272010aaad7f3439` |
| loopback `src/package_lib.bin` | `5f8d468fa9c0d79738e1b96a8258d3ed9bf79df881ca0b134b901a4312fe5a48` |
| alloc-smoke `src/package_lib.bin` | `879713045ecbb3257ce3af077911a51895a5d0a1bb9ed968a735331c98327e20` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. The hardware gate was run with the
BLE deploy command for both artifacts: install, start, ping, echo, status, and
teardown all succeeded, and the echo response was `0102020908`. The alloc-smoke
package performs the same sequence through its allocator-backed app-data
callback, so the probe exercises the allocation instead of optimizing it away.
