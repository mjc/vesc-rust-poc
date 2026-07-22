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

The firmware table addresses are target-specific: ESP32-C3 `0x3FCDBE00`,
ESP32-S3 `0x3FCE8800`, ESP32-C6 `0x4087B800`, and ESP32-P4 `0x4FF3A000`.
They are documentation only here; a host pointer must never be made from one
of these values.

## Current implementation boundary

`ExpressTable::load` checks the version before exposing any slot, accepts a
shorter table for older firmware, and returns raw words or non-null target
addresses without calling through them. Callable Express wrappers and
target-specific package builds still need to be added and proven on Express
hardware. This foundation deliberately does not use bindgen or reinterpret the
STM32 ABI.
