#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
filtered_dir=$(mktemp -d "${TMPDIR:-/tmp}/fob-production.XXXXXX")
trap 'rm -rf -- "$filtered_dir"' EXIT

cd "$repo_root"
for source in \
  examples/float-out-boy/src \
  crates/vesc-float-out-boy-protocol/src \
  crates/vesc-float-out-boy-leds/src \
  crates/vescpkg-rs/src/stm32
do
  "${CARGO:-cargo}" run --quiet \
    --manifest-path tools/fob-production-rust/Cargo.toml -- \
    "$source" "$filtered_dir/$source"
done
nix run nixpkgs#tokei -- "$filtered_dir" --types Rust "$@"
