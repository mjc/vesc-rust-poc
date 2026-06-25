# Repository Hygiene

This repo keeps reproducible lockfiles under version control and leaves generated package outputs out of source control.

## Tracked Files

- `Cargo.lock`
- `flake.lock`

## Ignored Outputs

- `target/`
- `*.bin`
- `*.vescpkg`

## Notes

- Generated package artifacts should stay under ignored staging paths.
- Source `.lisp` files remain tracked when they are part of fixtures or package inputs.
