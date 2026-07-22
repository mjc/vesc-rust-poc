# Control-loop smoke package

This unofficial package is a small, no-actuation example of the SDK runtime
pattern: a periodic firmware thread updates owned state under the SDK gate and
an app-data callback reads or changes the same state. The loop sleeps only
after releasing the state borrow.

The deliberately tiny command surface is:

- `[1, lo, hi]` sets a signed little-endian setpoint and returns `[1, 0]`;
- `[2]` returns `[2, setpoint, sampled_input, output, tick_count]` in the
  corresponding little-endian fields.

Host tests exercise the control step, command validation, shared-state
round-trips, and package startup. Build the ARM package with:

```text
cargo run -p cargo-vescpkg -- build -p vesc-example-control-loop-smoke
```

The package intentionally does not move a motor. A real-device probe still
needs to record advancing ticks, setpoint/output round-trips, request progress
under repeated traffic, and clean teardown.
