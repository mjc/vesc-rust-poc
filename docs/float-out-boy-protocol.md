# Float Out Boy app-data protocol

This is the wire reference for clients talking to the Float Out Boy package.
The implementation source of truth is
[`FloatOutBoyAppDataCommand`](../examples/float-out-boy/src/domain/app_data.rs)
and the package dispatch in
[`state.rs`](../examples/float-out-boy/src/package/state.rs).

Every request begins with package ID `101`, followed by a command byte and its
payload:

```text
[101, command, payload...]
```

Multi-byte integers are big-endian. Realtime numeric fields use the package's
Float16 codec unless a command 33 request selects Float32. A packet with the
wrong package ID, an unknown command, or a truncated required block is ignored.
Trailing bytes are accepted where the command parser explicitly treats a
payload as extensible.

## Command IDs

This table is checked by the `FloatOutBoyAppDataCommand` unit test.

| Command | ID | Effect | App-data response |
| --- | ---: | --- | --- |
| `Info` | `0` | Query package metadata | Yes |
| `GetRealtimeData` | `1` | Query legacy realtime data | Yes |
| `RuntimeTune` | `2` | Change active tune | No |
| `TuneDefaults` | `3` | Reset active tune fields | No |
| `ConfigSave` | `4` | Persist the active configuration | No |
| `ConfigRestore` | `5` | Reload persisted configuration | No |
| `TuneOther` | `6` | Change active startup, tiltback, and input settings | No |
| `Booster` | `8` | Change active booster settings | No |
| `PrintInfo` | `9` | Compatibility no-op | No |
| `GetAllData` | `10` | Query legacy all-data modes | Yes |
| `Experiment` | `11` | Compatibility no-op | No |
| `Lock` | `12` | Reload, enable or disable, and persist while not running | No |
| `HandTest` | `13` | Enter or leave hand-test mode | No |
| `TuneTilt` | `14` | Change active duty and speed pushback settings | No |
| `Remote` | `15` | Update unified remote tilt/move input | No |
| `LightsControl` | `20` | Query or change runtime light overrides | Yes |
| `Flywheel` | `22` | Enter or leave flywheel mode | No |
| `LcmPoll` | `24` | Poll the external light-control module | Yes |
| `LcmLightInfo` | `25` | Query light configuration | Yes |
| `LcmLightControl` | `26` | Update light-control module state | No |
| `LcmDeviceInfo` | `27` | Query light-control module identity | Yes |
| `ChargingState` | `28` | Update charger state and readings | No |
| `LcmGetBattery` | `29` | Query battery data for the light-control module | Yes |
| `RealtimeData` | `31` | Query the AppUI realtime packet | Yes |
| `RealtimeDataIds` | `32` | Query realtime field identifiers | Yes |
| `RealtimeDataSelected` | `33` | Query mask-selected realtime fields | Yes |
| `AlertsList` | `35` | Query active and historical alerts | Yes |
| `AlertsControl` | `36` | Clear the fatal-alert latch | No |
| `DataRecordRequest` | `41` | Control or read the data recorder | Depends on subcommand |
| `DataRecordHeader` | `42` | Recorder header response only | Response only |
| `DataRecordData` | `43` | Recorder sample response only | Response only |
| `LcmDebug` | `99` | Reserved and intentionally undispatched | No |

Commands that return no app-data response still consume a valid request. A host
tool which always waits for a response can therefore report a timeout after a
successful mutation. Confirm mutations through a readback command or the
package UI; do not treat the timeout alone as proof of either success or
failure.

## Metadata and legacy queries

- `[101, 0]` requests the five-byte legacy info response.
- `[101, 0, 2, flags]` requests the 60-byte version 2 response. Unknown
  requested versions use the version 2 shape with request flags cleared.
- `[101, 1]` returns the fixed legacy realtime packet.
- `[101, 10, mode]` returns legacy all-data mode `1` through `4`. Higher mode
  numbers include every earlier extension block.

Minimal example: `[101, 0]`.

Boundary example: `[101, 0, 255, 255]` is accepted as an unknown info version
and receives the highest known response shape without echoing the unknown
flags.

## Runtime tuning

Commands `2`, `3`, `6`, `8`, and `14` modify the active 282-byte configuration
only. They do not write EEPROM. Send command `4` separately to persist the
result.

### Command 2: progressive tune blocks

The payload is decoded only when a complete block is present:

| Payload length | Newly available block |
| ---: | --- |
| `0..=11` | No tune block; the command is still handled |
| `12..=15` | Primary PID, booster, turn, ATR, and brake-tilt nibbles |
| `16` | Torque-tilt block |
| `17` | Brake-gain byte |
| `18` | No additional block |
| `19+` | Orientation and revised ATR-speed block |

The primary and torque blocks pack two four-bit settings into most bytes.
The precise scaling lives in
[`tuning.rs`](../examples/float-out-boy/src/package/state/tuning.rs); clients
should preserve unused nibbles rather than inventing values.

Minimal example: `[101, 2]` performs a runtime reconfigure without changing a
tune field.

Boundary example: a 19-byte payload after `[101, 2]` enables every progressive
block; byte 17 is the brake-gain byte and bytes 18-19 are the extended block.

### Command 6: startup and other tune settings

The first 12 payload bytes are required:

```text
flags, startup_speed, pitch_tolerance, roll_tolerance,
brake_current, click_current, constant_tilt, nose_speed,
constant_erpm, variable_rate, variable_max, variable_erpm
```

Optional bytes 13-14 carry input-tilt type/limit and its compatibility speed
byte. Optional byte 15 is the secondary flag byte:

- bit 0 disables moving faults;
- bit 1 enables the footpad beep;
- bits 2-3 select parking-brake mode.

`variable_max` uses a signed compatibility encoding: `0..=100` means
`value / 10` degrees; `101..=255` means `-(value - 100) / 10` degrees. Thus
`100` is `+10.0°`, `101` is `-0.1°`, and `255` is `-15.5°`.

Minimal valid example:
`[101, 6, 0, 0, 0, 0, 0, 0, 80, 0, 0, 0, 0, 0]`. The seventh payload byte
uses the lowest accepted constant-tilt encoding; the other required fields are
zero.

Boundary example: a payload with `variable_max = 255` applies `-15.5°`; a
15-byte payload also applies the secondary flags.

### Command 14: duty and speed pushback

The five required payload bytes are:

```text
flags, return_speed, duty_threshold, duty_angle, duty_speed
```

An optional sixth byte sets the speed-pushback threshold in km/h. Flag bit 0
controls the duty beep. A zero return-speed byte preserves the current return
speed.

Minimal example: `[101, 14, 0, 0, 0, 0, 0]`.

Boundary example: append `255` to set the optional speed threshold to the
maximum wire-byte value.

## Persistence commands

- `[101, 4]` stores the active configuration. The package intentionally sends
  no app-data acknowledgement.
- `[101, 5]` reloads the EEPROM image, applies defaults after a read failure or
  invalid signature, refreshes derived runtime state, and intentionally sends
  no app-data response.

The custom-config callback is a separate path: a valid 282-byte image is
validated, stored, installed as active state, and reconfigured. The package
stores it inside a zero-padded 320-byte EEPROM image. See
[the architecture reference](float-out-boy-architecture.md#configuration-and-persistence).

## Realtime protocols

### Commands 31 and 32

`[101, 31]` returns:

```text
101, 31, data_mask:u8, extra_flags:u8, timestamp:u32,
state_flags:u32, always_values:float16[18],
[running_values:float16[11]], [charging_current, charging_voltage],
active_alerts:u32, reserved_flags:u32, firmware_fault:u8
```

`data_mask` bit 0 says the running block is present, bit 1 says the charging
block is present, and bit 2 identifies the current packet shape. `state_flags`
packs mode, run state, footpad state, charging/fatal/darkride/wheelslip flags,
setpoint adjustment, stop condition, and beep reason. Consumers must mask known
bits and preserve unknown values.

`[101, 32]` returns two counted lists: 18 always-present field names followed by
11 running-only field names. Treat that response as the authoritative ordering
for command 31.

Minimal example: `[101, 31]`.

Boundary example: a running-and-charging response contains both conditional
blocks and the common alert/fault trailer.

### Command 33

The request payload is:

```text
control_flags:u8, mask1:u32, [mask2:u32]
```

The first five bytes are required. A partial second mask is ignored; it becomes
zero. Control bit 0 selects Float16 (`0`) or Float32 (`1`) for ordinary numeric
fields. Unknown control and mask bits are preserved and echoed.

The response is:

```text
101, 33, control_flags:u8, mask1:u32, mask2:u32, timestamp:u32,
selected fields in ascending mask-bit order
```

Mask 1 selects status, motor, IMU, footpad, setpoint, current, and loop fields.
Mask 2 selects odometer, distance, charging, energy, motor ID, and GNSS fields.
Odometer and timestamps remain `u32`; latitude and longitude remain `f64`;
other selected numeric fields use the requested precision. If GNSS fields are
selected but no valid snapshot is available, the request receives no response.

Minimal example: `[101, 33, 0, 0, 0, 0, 0]` returns only the echoed header and
timestamp.

Boundary example: `[101, 33, 255, 255, 255, 255, 255, 255, 255, 255, 255]`
preserves every unknown bit while emitting every currently assigned field that
has an available source.

## Data recorder

Recorder storage is optional. Before exposing it, the SDK validates the
firmware descriptor magic, major version `1`, minimum minor version `1`, word
alignment, nonzero size, overflow, and containment in the reserved recorder
RAM range. Without a valid descriptor, status remains available but header and
data requests intentionally receive no response.

All requests use command `41`:

| Request payload | Meaning | Response |
| --- | --- | --- |
| `[1, 0]` | Status | `[101, 41, available, flags, decimation, duration_cs:u16]` |
| `[1, 1, value]` | Stop (`0`) or start (`>0`) | Status |
| `[1, 2, value]` | Set autostart | Status |
| `[1, 3, value]` | Set autostop | Status |
| `[1, 4, value]` | Set decimation; zero normalizes to one | Status |
| `[2, 1]` | Stop recording and request header | Command `42` |
| `[2, 2, offset:u32]` | Request samples starting at offset | Command `43` |

The command 42 response is:

```text
101, 42, sample_count:u32, item_count:u8, counted UTF-8 item names...
```

There are 13 recorded fields. Each command 43 packet starts with
`[101, 43, offset:u32]` and appends as many complete samples as fit in 511
bytes. A sample is `timestamp:u32`, one packed state byte, then 13 Float16
values. Timestamps are system ticks and are forced strictly increasing in the
ring. The state byte packs setpoint adjustment in bits 4-7, footpad state in
bits 2-3, wheelslip in bit 1, and running in bit 0. Clients paginate by adding
the number of returned samples to the requested offset. An empty recording
produces no command 43 packet.

Minimal example: `[101, 41, 1, 0]` queries status.

Boundary example: `[101, 41, 2, 2, 255, 255, 255, 255]` is a valid out-of-range
page request and receives no data packet.

## Remote, lights, charging, and alerts

- Remote: `[101, 15, input]`, where `input` is signed `i8 / 127`; `-128` is
  reserved and ignored. The command input expires after 0.5 seconds.
- Lights: command 20 replies with the effective runtime enable/headlight bits.
  Command 26 requires brightness, idle brightness, and status brightness, then
  accepts a bounded forward payload for the next LCM poll.
- Charging: `[101, 28, 151, active, voltage:i16, current:i16]`; voltage and
  current use scale 10 and the charging state expires after five seconds
  without refresh.
- Alerts list: `[101, 35, since:u32]`; a missing timestamp means zero. The
  response contains active masks, current fault information, and bounded
  transition records newer than `since`.
- Alerts control: `[101, 36, 1]` clears the fatal latch without clearing an
  active firmware fault.

These mutation commands do not all reply. The command table above is the
response contract.

Minimal valid examples:

- `[101, 15, 0]` supplies a centered remote input;
- `[101, 20]` queries effective light state without changing an override;
- `[101, 24]` polls the LCM without adding a pending request payload;
- `[101, 28, 151, 0, 0, 0, 0, 0]` clears charging state;
- `[101, 35]` lists alerts since timestamp zero.

Boundary examples:

- `[101, 15, 128]` uses the reserved `i8::MIN` remote byte and is consumed
  without changing input;
- `[101, 20, 0, 0, 0, 3, 3]` updates both runtime light override bits and
  returns their effective state;
- command 26 forwards at most 64 extra payload bytes, discarding a longer tail;
- command 28 accepts the complete signed `i16` voltage/current wire range;
- `[101, 35, 255, 255, 255, 255]` requests only alert records newer than the
  largest wire timestamp and still returns current alert state.
