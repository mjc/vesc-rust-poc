# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | SHA-256 |
| --- | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | `b11d23463f6832da25377fde6d10616a1f9cf36013f8d1834e0ec13dca57c881` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | `5690a9897a74e6f66b0fb425e3476b5fca342870cddc5d21b11f337c6fd91720` |
| loopback `src/package_lib.bin` | `14c8f5f3b1e5dfa8414555b62c18ee46f24ef6487910385ca7e4bacc61cd24cc` |
| alloc-smoke `src/package_lib.bin` | `09a9067e7598021f6f194f72cb251460971ecf33f5577a1af4962f87c0e2fdef` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. The hardware gate was run with the
BLE deploy command for both artifacts: install, start, ping, echo, status, and
teardown all succeeded, and the echo response was `0102020908`. The alloc-smoke
package performs the same sequence through its allocator-backed app-data
callback, so the probe exercises the allocation instead of optimizing it away.
