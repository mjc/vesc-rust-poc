# VESC Express ABI boundary

The Express native-library ABI is a separate interface from the STM32 `VescIf`
table. The SDK keeps it in `vescpkg-rs-sys::express` so the two slot orders and
pointer models cannot be mixed accidentally.

## Pinned source

The current mapping is from the official
[`vesc_c_if.h`](https://raw.githubusercontent.com/vedderb/vesc_express/2ae16033156d1a077fce3719ddf438c40a646b54/main/c_libs/vesc_c_if.h)
at commit `2ae16033156d1a077fce3719ddf438c40a646b54`.

That header defines:

- ABI version `1` in the first slot;
- native-library magics `0xCAFEBABE` and `0xCAFEBABF`;
- an 80-word, 320-byte target table (the target ABI uses 32-bit words);
- five inline LispBM symbol constants at slots 38 through 42, with function
  slots elsewhere;
- appended-only function slots, which are null on firmware that predates them;
- a 1,000 Hz system tick rate.

The firmware table addresses are represented by `ExpressTarget` and are
target-specific: ESP32-C3 `0x3FCDBE00`,
ESP32-S3 `0x3FCE8800`, ESP32-C6 `0x4087B800`, and ESP32-P4 `0x4FF3A000`.
They are documentation only here; a host pointer must never be made from one
of these values.

## Current implementation boundary

`ExpressInterface::from_words` checks the version before exposing any slot,
accepts a shorter table for older firmware, and exposes named capability
queries. Its explicitly unsafe `function::<F>` boundary can resolve a raw
function pointer only when the caller supplies the exact C ABI signature from
the pinned header. It does not make those calls safe, and it never reinterprets
the STM32 table. Target-specific package builds and hardware proof still need
to be added.

The module provides named pointer aliases for the shared clock, sleep,
allocation, thread, mutex, and semaphore signatures. `ExpressRuntime` turns the
clock/sleep/timer/termination/priority subset into checked methods after its
unsafe live-table constructor establishes the target invariant. Variadic
`printf` and STM32-only motor/CAN/peripheral slots remain outside this shared
surface.

The same provider exposes RAII `ExpressMutex` and `ExpressSemaphore` handles;
firmware-owned handles are released with the header-prescribed `free` slot,
and mutex guards unlock on drop. Creation rejects absent slots and null handles
without manufacturing a dummy synchronization object.

`ExpressAllocation` provides the corresponding explicit firmware-owned byte
allocation. It is not installed as a global allocator: callers initialize or
borrow the handle deliberately, and drop always returns the pointer through the
Express `free` slot.

The fixed-address `ExpressInterface::from_target` constructor is also unsafe:
it is only valid on the matching 32-bit Express target and is intentionally not
used by host tests. This foundation deliberately does not use bindgen.
