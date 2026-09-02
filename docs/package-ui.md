# Package UI and asset authoring

Package interfaces in this workspace are package-owned QML and LispBM assets.
There is no generated `ConfigUi`, `RealtimeUi`, or `vesc_ui!` API. The build
pipeline copies the package's asset tree and embeds the selected top-level
files alongside the native Rust payload.

## Minimal package setup

A package that owns assets uses this shape:

```text
my-package/
├── Cargo.toml
├── build.rs
├── src/
└── package/
    ├── README.md
    ├── code.lisp
    ├── ui.qml
    └── pkgdesc.qml
```

`Cargo.toml` and exactly one Rust binary or static-library target are required.
Use `build.rs` when the package owns assets or relies on the shared binary-link
configuration. Every file under `package/` is optional.

Declare the package-facing metadata in `Cargo.toml`:

```toml
[package.metadata.vescpkg]
name = "Example package"
version = "0.1.0"
qml-fullscreen = false
```

`name` defaults to the Cargo package name, `version` defaults to the Cargo
package version, and `qml-fullscreen` defaults to `false`.

Use the shared build-script helper:

```rust
fn main() {
    vescpkg_build_support::build_package(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
    );
}
```

Add `vescpkg-build-support` under `[build-dependencies]`. The helper watches
`package/`, clears its generated asset directory on every build, copies regular
files and nested directories, and configures the package linker for the ARM
target.

## Asset behavior

| Path | When present | When absent |
| --- | --- | --- |
| `package/README.md` | Becomes the package Markdown description; HTML is rendered from it for the archive | A description containing the display name and version is generated |
| `package/code.lisp` | Becomes the package loader and supplies top-level Lisp import declarations | A loader that imports `src/package_lib.bin` and calls `load-native-lib` is generated |
| `package/ui.qml` | Is embedded as the package AppUI source | No QML AppUI is embedded |
| `package/pkgdesc.qml` | Supplies a custom package descriptor after validation | A descriptor matching the staged files and Cargo metadata is generated |
| Other regular files | Are available while staging and can be included by a top-level `code.lisp` import | Nothing is added |

`src/package_lib.bin` is reserved for the linked Rust payload. A static or
generated asset at that path fails the build instead of replacing the native
image. Symlinks and other non-regular asset entries are also rejected.

Lisp imports are resolved relative to the staged package root. For example:

```lisp
(import "src/package_lib.bin" 'package-lib)
(import "bms.lisp" 'bms)
```

The builder packs imports declared directly in `code.lisp`, rejects absolute
paths and parent-directory traversal, aligns their payloads, and enforces the
firmware upload envelope. It does not recursively discover imports inside an
already imported Lisp file.

## Adding a QML interface

Create `package/ui.qml` with the imports and root item required by the intended
host. A minimal source is:

```qml
import QtQuick 2.15

Item {
    property string tabTitle: "Example"
}
```

Omit `pkgdesc.qml` unless the package needs descriptor-specific behavior such
as a compatibility function. The generated descriptor automatically names
`ui.qml` and uses `qml-fullscreen` from Cargo metadata.

When a custom descriptor is present, `cargo-vescpkg` requires string-literal
values matching the staged artifact:

- `pkgDescriptionMd` must be `README.md`;
- `pkgLisp` must be `code.lisp`;
- `pkgQml` must be `ui.qml`, or empty when there is no UI; and
- `pkgOutput` must be the generated artifact filename.

If the descriptor declares `pkgQmlIsFullscreen`, it must be a Boolean literal
matching `qml-fullscreen`. The descriptor can contain additional properties
and functions; Float Out Boy's
[`pkgdesc.qml`](../examples/float-out-boy/package/pkgdesc.qml) uses this for a
compatibility check.

To remove the UI, delete `package/ui.qml` and either delete the custom
descriptor or change its `pkgQml` property to the empty string. The clean asset
staging step prevents a deleted QML file from surviving a rebuild.

## QML-to-Rust boundary

The workspace does not generate widgets, configuration fields, realtime
fields, JavaScript, or manifests from Rust declarations. QML communicates
through interfaces supplied by its host and through the package's own
custom-config and app-data protocols.

When adding a field:

1. define its typed representation and wire behavior in the Rust package;
2. add encoder/decoder tests at that protocol boundary;
3. update the QML request or response handling; and
4. update the package's protocol reference when the field is public.

The [Float Out Boy protocol](float-out-boy-protocol.md) is the complete example
of keeping typed Rust commands, QML behavior, response semantics, and a checked
wire reference aligned.

QML syntax, imports, widgets, and host APIs are not validated by
`cargo-vescpkg`. They depend on the client that loads the package. A successful
Rust/package build proves asset assembly, not that the UI loads or behaves
correctly in every client.

## Build boundary

Build a package from the workspace root:

```console
cargo run -p cargo-vescpkg -- build -p my-package
```

The command stages the native payload and assets below
`target/vescpkg/<display-name-slug>-<version>/`, assembles the `.vescpkg`, and
decodes the result through the package reader. Installation and client/device
UI validation remain separate steps.
