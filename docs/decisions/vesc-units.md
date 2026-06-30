# vescpkg-units evaluation (deferred)

## Decision

Do **not** add a `vescpkg-units` crate in the current epic scope.

Physical units (speed, torque, temperature, battery state) belong outside the target SDK
and outside raw `vescpkg-sys`. They should live in a separate optional crate once real
vehicle semantics stabilize.

## Recommended direction

| Option | When |
|--------|------|
| `vescpkg-units` | Generic reusable SI/newtype helpers shared by multiple VESC packages |
| Project-specific crate (e.g. `cutout-units`) | Product semantics tied to one firmware/product line |

## Constraints for a future crate

- `no_std`, no `alloc` unless proven necessary
- No dependency on `vescpkg-build` or `vesc-cli`
- May depend on `vesc-protocol` only if wire encodings need typed units
- Keep out of `vescpkg` prelude until at least one real package consumes typed units

## Status

Spike complete. Revisit after GPIO helpers and install API are exercised on hardware.
