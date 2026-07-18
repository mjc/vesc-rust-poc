# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | SHA-256 |
| --- | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | `7e365076b3b1ec052e0c0e678babebc192e123be5f486fcc5cd643b76a3a4635` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | `13e2b2865dfa76e4f16b5c63d192b750ecdc8a48fdb7ff80819c721353913820` |
| loopback `src/package_lib.bin` | `24f6da8eae16e4b703a3d20243df6410906449d28fa1589971854a6b03d45d43` |
| alloc-smoke `src/package_lib.bin` | `965853ac02abacea27180f7056e256a2f48e627f950795f7be09564f38aab13d` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. The hardware gate was run with the
BLE deploy command for both artifacts: install, start, ping, echo, status, and
teardown all succeeded, and the echo response was `0102020908`. The alloc-smoke
package performs the same sequence through its allocator-backed app-data
callback, so the probe exercises the allocation instead of optimizing it away.
