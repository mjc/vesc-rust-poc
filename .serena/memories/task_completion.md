# Task Completion

- Run `nix develop -c make check` before calling a change complete.
- If packaging logic changed, also run `nix develop -c make package-only`.
- If a final package artifact matters, run `nix develop -c make package` with the local VESC Tool path already present on disk.
- For focused command-slice changes, `nix develop -c cargo test -p vesc-pkg-build cargo_vescpkg_command` is the fastest regression check.
- Keep the package-size guard green; it is part of the acceptance boundary for uploadable `.vescpkg` output.
- Hardware BTLE smoke remains separate when the machine lacks a Bluetooth adapter.