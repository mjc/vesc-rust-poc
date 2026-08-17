#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

exec nix run nixpkgs#tokei -- \
  examples/float-out-boy/src \
  crates/vesc-float-out-boy-protocol/src \
  crates/vesc-float-out-boy-leds/src \
  crates/vescpkg-rs/src/stm32/float_out_boy_ws2812.rs \
  crates/vescpkg-rs/src/stm32/circular_dma_pwm.rs \
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
