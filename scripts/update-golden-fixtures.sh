#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if command -v nix >/dev/null 2>&1; then
  RUNNER=(nix develop -c)
else
  RUNNER=()
fi

echo "==> verifying native build (symbol_audit)"
"${RUNNER[@]}" cargo test -p vesc-pkg symbol_audit -- --quiet

echo "==> writing golden fixtures"
"${RUNNER[@]}" cargo run -p vesc-pkg --bin write-golden-fixtures

echo "==> verifying golden tests"
"${RUNNER[@]}" cargo nextest run -p vesc-pkg --test golden_packaging --test native_lib_artifacts

echo "golden fixtures updated under fixtures/golden/ble-loopback-0.1.0/"
