# Device-proven native-lib fixtures

`legacy-init.bin` captures init bytes from a package binary that ran on hardware before
Rust-owned loader registration replaced the legacy hand-asm path.

Tests compare the current `.init_fun` section against this fixture to ensure Rust init
no longer matches the old bytes.
