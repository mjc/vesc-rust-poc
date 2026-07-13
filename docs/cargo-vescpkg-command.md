# Cargo VescPkg Command

`cargo-vescpkg` is the only host-side Cargo external subcommand in this
experiment. Cargo owns compilation and the final embedded ELF link; the command
consumes Cargo's artifact JSON, flattens that ELF, assembles the package, and
optionally installs it.

Cargo packages are inputs selected with normal Cargo semantics, not plugins or
providers. Package-specific metadata lives in `[package.metadata.vescpkg]`.

## Build

```bash
nix develop -c cargo vescpkg build -p vesc-example-loopback
```

Build options are Cargo-shaped: `--manifest-path`, `--target`, `--profile`, and
`--features`. The build invokes `cargo metadata` and one
`cargo build --message-format=json-render-diagnostics`, selects the requested
package's final binary artifact, converts it with `rust-objcopy`, and emits it
under Cargo's target directory at `vescpkg/`.

## Device commands

- `cargo vescpkg deploy <package.vescpkg>`
- `cargo vescpkg package-install <package.vescpkg>`
- `cargo vescpkg erase-package`
- `cargo vescpkg loopback`
- `cargo vescpkg lisp-probe`
- `cargo vescpkg refloat-probe`

The checked workspace path remains `nix develop -c make check`.
