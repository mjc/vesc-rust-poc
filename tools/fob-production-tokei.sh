#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
exec nix run nixpkgs#tokei -- \
  examples/float-out-boy/src \
  --types Rust \
  --exclude test \
  --exclude tests \
  --exclude test_support \
  --exclude test.rs \
  --exclude tests.rs \
  --exclude test_support.rs \
  --exclude '*_test.rs' \
  --exclude '*_tests.rs' \
  "$@"
