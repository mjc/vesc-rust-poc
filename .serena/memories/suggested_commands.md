# Suggested Commands

- `nix develop -c make check` for the canonical workspace verification.
- `nix develop -c cargo test --workspace` for quick Rust test runs.
- `nix develop -c cargo test -p vesc-pkg-build cargo_vescpkg_command` for the command-design slice.
- `nix develop -c make package-only` to validate staging, conversion, inspection, and package layout.
- `nix develop -c make package` to emit the final `.vescpkg` when a full package artifact is needed.
- `git status --short --branch` to inspect the current branch and dirty state.
- Use repo-local paths and on-disk references before reaching for the network.