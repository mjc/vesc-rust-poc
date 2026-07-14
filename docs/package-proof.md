# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | SHA-256 |
| --- | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | `7a743257ea27e7420eb98e51c4cb732781577858331a67a74bafd32131ebe0fd` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | `373871fe3f7875d9dc32c813914370248d5103bff83157d477f6f405da92ff7e` |
| loopback `src/package_lib.bin` | `d09943d6531cb98159f1ecf613009be234019f4c0ea8448cbc72d010421d8353` |
| alloc-smoke `src/package_lib.bin` | `b0b7ce15cf6bd18f69d1d41616a88a397e067879da370535af49715b3d1fe533` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. The hardware gate was run with the
BLE deploy command for both artifacts: install, start, ping, echo, status, and
teardown all succeeded, and the echo response was `0102020908`. The alloc-smoke
package performs the same sequence through its allocator-backed app-data
callback, so the probe exercises the allocation instead of optimizing it away.
