# vescpkg-rs-units decision

## Decision

Keep `vescpkg-rs-units` as the generic embedded units layer for the current
package-author API work.

The crate owns reusable physical-ish quantities such as voltage, current,
power, temperature, speed, distance, GNSS coordinates, and VESC system ticks.
It does not own VESC-domain meaning. Domain-specific names such as motor
current, battery current, FOC motor resistance, and GNSS speed belong in
`vescpkg-rs::types`.

Raw ABI values still belong in `vescpkg-rs-sys`, and raw protocol byte
conversion belongs in `vesc-protocol`.

## Recommended direction

| Option | When |
|--------|------|
| `vescpkg-rs-units` | Generic reusable no_std unit newtypes and obvious dimensional arithmetic |
| `vescpkg-rs::types` | VESC-specific semantic wrappers over units and raw typed tokens |
| Project-specific crate (e.g. `cutout-units`) | Product semantics tied to one firmware/product line |

## Constraints

- `no_std`, no `alloc` unless proven necessary
- Default features do not enable `std`, `alloc`, or `uom`
- Use `fugit` for VESC system tick duration/instant modeling
- Use explicit named accessors; do not add `From<Unit> for f32`
- No dependency on `vescpkg-rs-build` or `cargo-vescpkg`
- Keep VESC-specific meaning out of the units crate

## Status

Implemented for VESCR-46/VESCR-47. The current crate is intentionally small,
`no_std`, `fugit`-backed for system time, and consumed by `vescpkg-rs::types`.
