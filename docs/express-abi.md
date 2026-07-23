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
`ExpressTarget::target_name`, `from_target_name`, and `sdkconfig_define` provide
the corresponding ESP-IDF target selection (`esp32c3`, `esp32s3`, `esp32c6`, or
`esp32p4`) for a future native-library build integration. They are
documentation/metadata only here; a host pointer must never be made from one
of these values.

`ExpressTarget::native_load_kind` records the loader split from the pinned
Express source: ESP32-S3 uses the relocatable container, while ESP32-C3, C6,
and P4 use the XIP image path.

## Build boundary

The current `cargo vescpkg build` path emits STM32 packages for
`thumbv7em-none-eabihf`; it must not be used as an Express builder. An Express
native library must select exactly one `ExpressTarget` through its ESP-IDF
`sdkconfig.h` define, use the matching fixed table address, and retain the
Express container/loader contract. Rust-side target metadata is now available
for that integration, while the Express-specific compiler/linker/package path
and device installation proof remain intentionally open.

`ExpressNativeXipImage` validates the pinned XIP shape (big-endian magic,
loader-required minimum length, and the eight-byte magic/program-address
header). `ExpressNativeContainer` validates the pinned ESP32-S3 relocatable
container header, little-endian region metadata, bounded code/data regions, and
region-relative relocation offsets without allocating or applying patches.
`ExpressNativeImage::parse` selects exactly one of those views from the target's
load-kind metadata, so a C3/C6/P4 XIP image cannot be passed as an S3 container
or vice versa. These are checked input boundaries for the future ESP-IDF/package
builder, not host loaders or substitutes for target execution.

## Current implementation boundary

`ExpressInterface::from_words` checks the version before exposing any slot,
accepts a shorter table for older firmware, and exposes named capability
queries. Each `ExpressSlot` also retains its pinned C name and exposes const
`kind`/`is_callable` metadata, so scalar symbol constants cannot be mistaken
for nullable function slots. Its explicitly unsafe `function::<F>` boundary can resolve a raw
function pointer only when the caller supplies the exact C ABI signature from
the pinned header. It does not make those calls safe, and it never reinterprets
the STM32 table. Target-specific package builds and hardware proof still need
to be added.

The module provides named pointer aliases for the shared clock, sleep,
allocation, thread, mutex, and semaphore signatures. `ExpressRuntime` turns the
clock/sleep/timer/termination/priority/get-arg subset into checked methods
after its unsafe live-table constructor establishes the target invariant.
`get_arg` remains an unsafe raw-pointer return because the argument vector is
owned by the live native program. Variadic `printf` and STM32-only
motor/CAN/peripheral slots remain outside this shared surface.

Thread spawning is available only through an unsafe method because callback,
name, and argument lifetimes are firmware-owned contracts; termination requests
remain checked once the caller holds an opaque firmware thread handle.

The same provider exposes RAII `ExpressMutex` and `ExpressSemaphore` handles;
firmware-owned handles are released with the header-prescribed `free` slot,
and mutex guards unlock on drop. Creation rejects absent slots and null handles
without manufacturing a dummy synchronization object.

`ExpressAllocation` provides the corresponding explicit firmware-owned byte
allocation. It is not installed as a global allocator: callers initialize or
borrow the handle deliberately, and drop always returns the pointer through the
Express `free` slot.

`ExpressLisp` covers the core scalar encode/decode, cons/list, type-predicate,
symbol-constant, evaluator-control, context/message, and extension-registration
slots with typed `ExpressLispValue`/`ExpressLispSymbol` wrappers. The symbol
lookup names follow the official `const char *` ABI, while registration,
error-reason, and string-decoding pointers retain their source mutability.
The
`ExpressFlatValue` builder covers the pinned flat-value constructors and
transfers or releases its firmware-owned buffer according to the context
handoff result. Registration, error-reason, and string-decoding entry points
retain explicit unsafe raw-pointer boundaries; the facade does not invent
ownership for firmware-managed strings or values.

The fixed-address `ExpressInterface::from_target` constructor is also unsafe:
it is only valid on the matching 32-bit Express target and is intentionally not
used by host tests. This foundation deliberately does not use bindgen.

The loader entry contract is available independently of any target toolchain:
`ExpressLibInfo` mirrors the pinned `lib_info` record, and
`express_native_start!` emits the `.program_ptr` and `.init_fun` entry symbols
after checking the loader pointer. An initializer must install its static stop
callback before returning success. The macro does not choose an ESP-IDF target,
linker script, or package container; those remain an explicit integration and
device-proof boundary.
