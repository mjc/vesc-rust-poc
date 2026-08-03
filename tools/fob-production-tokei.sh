#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
filtered_dir=$(mktemp -d "${TMPDIR:-/tmp}/fob-production.XXXXXX")
trap 'rm -rf -- "$filtered_dir"' EXIT

cd "$repo_root"
"${CARGO:-cargo}" run --quiet \
  --manifest-path tools/fob-production-rust/Cargo.toml -- \
  examples/float-out-boy/src "$filtered_dir"
nix run nixpkgs#tokei -- "$filtered_dir" --types Rust "$@"
