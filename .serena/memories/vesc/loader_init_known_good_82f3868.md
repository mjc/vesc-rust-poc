# VESC loader init known-good correction

User confirmed commit `82f3868` works on real VESC hardware. For the loopback package, the loader-facing `.init_fun init` should treat setup as best-effort and return success; do not fail `load-native-lib` just because optional setup or registration helpers return false.

Root cause found 2026-06-29: current HEAD had regressed `examples/loopback/src/init.rs` so `package_lib_init`/`.init_fun init` could return `0` on stop-hook, app-data, null `LibInfo`, or extension-registration false results. That could make VESC reject/abandon the native image before `lisp-probe`/loopback could prove the extension.

Fix shape: `package_lib_init` attempts stop-hook install and returns true; `.init_fun init` calls setup best-effort, re-registers the app-data handler after extension registration, and returns true. Keep lower-level lifecycle helpers strict in direct tests so firmware rejection still propagates below the loader entrypoint.

Artifact tests: keep the insta snapshot pinned to the `82f3868` known-good fixture; check current generated ELF semantically for best-effort init success, no `mov/movs r0, #0` failure return in `init`, and stop cleanup either direct or via tail-call helper.

Beads anchor: `br-pob.8`; do not close until hardware retest evidence exists for package-install plus `lisp-probe`/loopback on `VESC BLE UART`.