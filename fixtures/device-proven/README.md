# Device-proven native-lib fixtures

`legacy-init.hex` captures init bytes from a package binary that ran on hardware before
Rust-owned loader registration replaced the legacy hand-asm path. Tests decode this fixture
at runtime so fresh clones do not depend on a gitignored `.bin` file.

Tests compare the current `.init_fun` section against this fixture to ensure Rust init
no longer matches the old bytes.

## Regenerating the fixture

1. Capture the legacy package binary init region from hardware-validated artifacts.
2. Write the full 183-byte reference payload as lowercase hex to `legacy-init.hex` (no spaces required).
3. Run `nix develop -c make check-full` to confirm layout audits still pass.

The previous on-disk name was `legacy-init.bin`; only the hex form is versioned in git.
