# Cargo package proof

The package cutover intentionally changes the bytes produced by the final
Cargo-owned embedded link. The accepted proof is therefore the decoded package
contract, stable native payload hashes, and real-device behavior rather than a
copy of the deleted legacy builder's output.

The current release artifacts produced by `cargo vescpkg build` are:

| artifact | bytes | SHA-256 |
| --- | ---: | --- |
| `Rust-BLE-loopback-test-package-0.1.0.vescpkg` | 2,820 | `deb0d07c23889ee77a2ead27a3da634dca77f0a60c954f9225730395b8543ef8` |
| `Rust-alloc-smoke-package-0.1.0.vescpkg` | 4,187 | `e937929f1d9f745a5d941718cbe636fb144fe0fa91f0debed3344b2469a677d1` |
| `Rust-control-loop-smoke-package-0.1.0.vescpkg` | 2,871 | `b535bdec53aca2e6c77899ffeb07ea18f9d77aecdd72fd805f565f45be0d0401` |
| `Refloat-1.2.1.vescpkg` | 87,023 | `d5e7afc37698668638d38c97554a84218aee00f110f106c7b837374680faa773` |
| loopback `src/package_lib.bin` | 2,972 | `f6d4d981508b56aa2738aba62e79b763c2946fe53c5875e9d976eb1f9e0e47c3` |
| alloc-smoke `src/package_lib.bin` | 4,888 | `0e6452d42128df37bf55f433d84abbf4708572b424418281dc35fff5df38c9e4` |
| control-loop `src/package_lib.bin` | 3,097 | `5152a5f6ebea10443c141a00a04f3e2dc2c80e1ec69b31b11715a7f3d6fb66fe` |
| Refloat `src/package_lib.bin` | 57,958 | `392c8236f96c758d04d337705e62b89f2eba4721212151dd7af3fbd22ba921f6` |

`cargo test -p cargo-vescpkg` decodes the compressed wire fixture through the
same package reader used by installation. A historical hardware gate ran the
BLE deploy command for the loopback and alloc-smoke artifacts: install, start,
ping, echo, status, and teardown all succeeded, and the echo response was
`0102020908`. The alloc-smoke package performs the same sequence through its
allocator-backed app-data callback, so the probe exercises the allocation
instead of optimizing it away.

The hashes above are the current artifact-only proof after the direct Cargo
binary cutover. A user-confirmed known-good Refloat device baseline is
`46323b07725b807f90ab9e3387d87a977e06a03a`; use that exact revision when
comparing later runtime changes. The hashes above do not claim a new device
run. The ignored HIL workflow in
`crates/cargo-vescpkg/tests/hil_loopback.rs` remains the required current
device gate and needs `VESC_DEVICE` plus `VESC_BLE_ADDR` before it can be run.

Regenerate the complete representative set with:

```text
nix develop --command make package-examples
```
