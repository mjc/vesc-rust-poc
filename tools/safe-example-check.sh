#!/usr/bin/env bash
set -euo pipefail

examples=(
  examples/alloc-smoke
  examples/control-loop-smoke
  examples/loopback
)

for example in "${examples[@]}"; do
  test -f "$example/Cargo.toml"
  test -f "$example/src/main.rs" || test -f "$example/src/lib.rs"
  rg -q '#!\[forbid\(unsafe_code\)\]' "$example/src"

  if rg -n 'vescpkg[-_]rs[-_]sys' "$example/Cargo.toml" "$example/src"; then
    printf 'safe example reaches vescpkg-rs-sys: %s\n' "$example" >&2
    exit 1
  fi
done
