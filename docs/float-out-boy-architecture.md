# Float Out Boy architecture

Float Out Boy is a package-specific control application built on reusable
`vescpkg-rs` lifecycle, callback, firmware-capability, storage, unit, and
threading mechanics. This document follows the production path from a firmware
sample to a motor command and names the boundaries where state is reset,
reloaded, or persisted.

## Lifecycle and ownership

[`startup.rs`](../examples/float-out-boy/src/package/startup.rs) installs one
typed `FloatOutBoyPackageState` in loader-owned package state. Startup then:

1. allocates the optional firmware recorder buffer;
2. starts the main and auxiliary threads;
3. registers the IMU, app-data, and custom-config callbacks; and
4. registers the package extensions.

The main thread reads and validates EEPROM after it starts. That keeps the
larger persistence path off the firmware evaluator stack. A package stop
terminates the threads, disables callback gates, releases package-owned
resources, stops the recorder, and tears down the installed state through the
shared SDK lifecycle.

The package has three execution paths:

- The **IMU callback** owns the latency-sensitive balance-filter, state-machine,
  PID/booster, motor-command, and recorder-sample path.
- The **main thread** targets 500 Hz and refreshes controller inputs, footpads,
  configuration-derived state, motor telemetry, alerts, charging/BMS state,
  beeper state, and mode transitions. It performs no second motor-control step
  in the ARM artifact.
- The **auxiliary thread** runs at 30 Hz for LED rendering, frequency-settle
  checks, motor-configuration refresh, and non-running odometer backup.

All three recover the same typed package state through SDK callback/thread
contexts. Package-specific policy never owns or rebases a raw firmware state
pointer directly.

## Timing

The main thread's target is fixed at 500 Hz. The legacy stored `hertz` field is
layout compatibility, not a runtime rate selector. Each iteration measures the
actual elapsed time with the firmware timer. After the work is complete,
[`FloatOutBoyMainLoopTiming`](../examples/float-out-boy/src/package/threads.rs)
subtracts rounded work ticks from the nominal period and sleeps for at least one
system tick. Invalid elapsed samples fall back to the nominal sleep.

The control path uses measured elapsed seconds, not `1 / 500`, for filters,
integrators, setpoint motion, kinematics, Reverse Stop, and telemetry. The
[`FrequencyTracker`](../examples/float-out-boy/src/package/state/frequency_tracker.rs)
low-pass filters the observed frequency. After a strict one-second settle
period, a change greater than three percent updates the filter frequency used
by dependent runtime filters. Firmware which reports a zero IMU rate starts
from the Refloat-compatible 620 Hz estimate until measurement settles.

Units:

- system timestamps are wrapping 100 µs ticks;
- timer differences become `VescSeconds`;
- rates are `SampleRate`/Hz;
- main-thread sleep is an integer number of firmware ticks.

## Sensor-to-motor data flow

The production path is:

```text
firmware IMU sample
  -> package balance filter and measured elapsed time
  -> typed attitude, motor, footpad, input, and ride state
  -> smooth setpoint modifiers
  -> torque-domain PID and booster
  -> motor-torque saturation and current conversion
  -> darkride/traction/output policy
  -> firmware motor-current command
  -> realtime and recorder telemetry
```

[`FloatOutBoyImuRead`](../examples/float-out-boy/src/package/imu_callback.rs)
receives acceleration, angular velocity, and sample period in the firmware
callback. It refreshes the balance filter, advances the ride-state/control path,
and applies the motor command before returning. The main thread consumes the
resulting state for slower inputs and transitions; it does not race a parallel
balance step.

Invalid or unavailable inputs fail closed:

- startup remains outside `Running` until configuration, IMU, and state gates
  allow engagement;
- unknown firmware fault values remain faulted rather than becoming "no fault";
- unavailable optional capabilities do not fabricate telemetry;
- a stopped, disabled, faulted, or package-stop path releases or suppresses the
  motor command according to the typed run state.

## Setpoint composition

[`SmoothSetpoint`](../examples/float-out-boy/src/package/state/smooth_setpoint.rs)
is the common state machine for turn tilt, remote tilt, ATR, torque tilt, and
brake tilt. Each modifier owns its current value and direction. A configuration
supplies:

- a base time constant;
- separate on/off time constants;
- separate on/off angular-speed limits; and
- a multiplier used by the modifier's transition policy.

Each update derives an exponential step from measured elapsed time, limits the
maximum signed angle change, and moves toward the requested target without
overshoot. Turning a modifier off therefore follows its configured off-rate
instead of snapping to zero. Reset paths clear both the accumulated setpoint
and transition direction.

The ride-modifier layer composes the individual angles into the final balance
setpoint. Sign is always explicit:

- pitch/setpoint angles are degrees at the policy boundary and radians in the
  IMU filter;
- forward/reverse direction is derived from typed electrical speed and torque;
- darkride inversion happens after the balance request is formed;
- runtime telemetry exposes each modifier separately so clients need not
  reverse the sum.

## Torque and current domains

The control loop keeps proportional, rate, integral, and booster state in a
motor-torque domain. [`MotorTorqueConstant`](../examples/float-out-boy/src/motor_torque.rs)
derives newton-metres per amp from firmware FOC flux linkage and pole count.
When those settings are unavailable or invalid, the explicit Refloat-compatible
constant is used.

The sequence in
[`balance/step.rs`](../examples/float-out-boy/src/balance/step.rs) is:

1. update proportional/rate/integral torque from pitch and pitch-rate error;
2. filter booster torque using measured elapsed time;
3. apply soft-start and pitch-based current limits;
4. combine PID and booster torque;
5. convert configured motor-current limits into torque and clamp the request;
6. apply darkride sign and traction-control filtering; and
7. convert the final torque to `MotorCurrent` with the current firmware-derived
   constant.

This preserves physical torque when the motor constant changes. It also keeps
the conversion boundary visible: PID gains and booster state do not silently
become raw amps. Final saturation uses typed positive/negative motor and battery
limits. Telemetry distinguishes requested balance current, measured motor
current, directional current, filtered current, battery current, and booster
torque.

## Reverse Stop

[`ReverseStop`](../examples/float-out-boy/src/package/state/reverse_stop.rs)
uses absolute motor travel in metres, not a tick counter. Reverse travel beyond
the entry threshold starts a smooth setpoint transition toward 17 degrees over
0.25 m. Direction changes can reverse the target. A one-hertz elapsed-time
filter smooths progress, and the stop timer shortens as distance progress
increases.

The state resets its distance origin and setpoint progress when the ride-state
reset path runs. While disabled and inactive it continues tracking the current
distance origin without creating a stop request. Completion, return travel,
loss of the required footpad/state conditions, and reconfiguration all pass
through explicit reset or stop-condition transitions rather than leaving the
previous distance accumulator live.

## Configuration and persistence

[`config.rs`](../examples/float-out-boy/src/config.rs) owns the generated
custom-config layout:

- serialized config length: **282 bytes**;
- signature: `0x191a6c1b`;
- default image: generated from the pinned settings XML;
- access: typed field views and `FloatOutBoyConfigEditor`, not open-coded
  offsets in control code.

[`config_storage.rs`](../examples/float-out-boy/src/package/state/config_storage.rs)
wraps that config in a deterministic **320-byte EEPROM image**. The first word
is the signature; the 282-byte config follows and the tail is zero-filled.
Writes invalidate the signature first, write the payload, then write the valid
signature last, so an interrupted write cannot look valid.

Load behavior is explicit:

- a valid image becomes the active configuration;
- an invalid signature/image loads generated defaults;
- an EEPROM read failure also loads generated defaults but retains a distinct
  diagnostic outcome;
- startup and restore refresh every derived runtime slice after installing the
  chosen image.

There are three mutation boundaries:

- **Runtime tune commands** change the active image and reconfigure runtime
  state but never write EEPROM.
- **Command 4** persists the current active image; **command 5** discards
  unsaved changes and reloads EEPROM.
- **The firmware custom-config callback** validates an exact 282-byte image,
  rejects special-mode writes, preserves the enabled state while running,
  persists the image, and then reconfigures.

Legacy firmware IMU gains are migrated through typed firmware settings during
startup/reload when the old value indicates migration is needed. This is a
firmware-configuration write and is separate from the package's EEPROM image.

Reconfiguration refreshes the balance filter, LED effects, beeper and disabled
state, motor/config limits, setpoint filters, and other derived values. Mode
shutdown paths such as hand-test and flywheel restore the persisted image
before resuming normal control.

## Shared SDK versus package policy

Reusable `vescpkg-rs` mechanics include:

- loader-owned state and package start/stop;
- typed callback and thread contexts;
- firmware capability discovery and optional-slot handling;
- physical-unit newtypes and motor/config setting access;
- EEPROM word/image operations;
- recorder descriptor validation and bounded buffer access; and
- app-data/custom-config callback registration.

Float Out Boy owns:

- command IDs and wire layouts;
- ride states, fault policy, setpoint modifiers, and transition timing;
- PID, booster, torque normalization, saturation, and motor-command policy;
- its 282-byte config schema and 320-byte persistence policy;
- recorder field selection and pagination; and
- board/package-specific footpad, LED, LCM, remote, and safety behavior.

Keeping that boundary explicit prevents a single package's compatibility rules
from becoming an SDK promise.
