# Cargo VescPkg Command

`cargo-vescpkg` is the only host-side Cargo external subcommand in this
experiment. Cargo owns compilation and the final embedded ELF link; the command
consumes Cargo's artifact JSON, flattens that ELF, assembles the package, and
optionally installs it.

Cargo packages are inputs selected with normal Cargo semantics, not plugins or
providers. Package-specific metadata lives in `[package.metadata.vescpkg]`.

```toml
[package.metadata.vescpkg]
name = "Package display name"
qml-fullscreen = true
```

`qml-fullscreen` defaults to `false`. If `package/pkgdesc.qml` also declares
`pkgQmlIsFullscreen`, both values must agree. The complete `package/` asset tree
is staged recursively; `src/package_lib.bin` is reserved for the compiled
native payload.

## Build

```bash
cargo run -p cargo-vescpkg -- build -p vesc-example-loopback
```

Build options are Cargo-shaped: `--manifest-path`, `--target`, `--profile`, and
`--features`. The build invokes `cargo metadata` and one
`cargo build --message-format=json-render-diagnostics`, selects the requested
package's final binary artifact, converts it with `rust-objcopy`, and emits it
under Cargo's target directory at `vescpkg/`.

## Device commands

- `cargo run -p cargo-vescpkg -- deploy <package.vescpkg>`
- `cargo run -p cargo-vescpkg -- package-install <package.vescpkg>`
- `cargo run -p cargo-vescpkg -- erase-package`
- `cargo run -p cargo-vescpkg -- loopback`

The checked workspace path remains `make check`.
