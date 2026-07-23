# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

These values were regenerated from SDK commit `0e082f93` on the ARM32 package
path; they are an artifact baseline, not a claim of byte identity with
`origin/main`.

| artifact | bytes | SHA-256 |
| --- | ---: | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | 2,822 | `825db1ee9e7d2378dc7abb61f22af2b704b64590d298e8d03221b79953503efc` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | 4,185 | `7c7e731fbb66ba75f03461f9276eae7a08aaa67c6152d54b5713b371369446ea` |
| `Rust-control-loop-smoke-package-0.1.0.vescpkg` | 2,873 | `f95f02385aad8d6105eaf11d6f92290d5917d8988c5a42eda77ba9145687e72e` |
| `Float-Out-Boy-0.1.0.vescpkg` | 101,013 | `f4ac1daee7ba31fdc85a889e74046515590b6c623d2eb0377e916eb322321ecf` |
| loopback `src/package_lib.bin` | 2,972 | `e488bd6413b8d5f8429ce431e52d497d127d5b2e8f945b7b6c22f96fbd3b1a4f` |
| alloc-smoke `src/package_lib.bin` | 4,888 | `ca2ba53cb68fcd968a6e97738dba37bfac780909dd42c1a5fabfca3ef0d682b6` |
| control-loop `src/package_lib.bin` | 3,097 | `98986583d770b2d70dac189c9c6bed724a93126fe368fd6864a427944c4da4a8` |
| Float Out Boy `src/package_lib.bin` | 78,312 | `4c6fc31a3d3a399eb9f9e98afb806ddf454ca6652d628d788b91a4a1278928c3` |

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
