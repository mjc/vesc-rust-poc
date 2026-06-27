# Task Completion Checklist

## Firmware changes

1. Build the relevant board target:
   ```bash
   make <board_name>
   ```
2. If multiple boards share changed `hwconf` core headers, spot-check at least one sibling target.
3. For motor/control logic changes, prefer building a board you can hardware-test; otherwise pick a common target (e.g. `100_250`).

## Host-side / algorithm changes

Run applicable tests under `tests/`:

```bash
cd tests/utils_math && make test
cd tests/angles && make test
cd tests/float_serialization && make test
cd tests/packet_recovery && make test
```

## Before considering done

- No unintended edits in vendored subtrees (`ChibiOS_3.0.5/`, `lispBM/lispBM/`)
- Board-specific changes scoped to correct `hwconf/` files
- Flash instructions unchanged unless flash workflow was modified

## No project-wide linter

There is no repo-wide `make lint` or clang-format hook; match existing style manually.