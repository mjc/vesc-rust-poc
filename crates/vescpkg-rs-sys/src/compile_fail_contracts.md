
# Compile-time contracts

These examples protect raw ABI boundaries that ordinary runtime tests cannot
exercise.

GPIO mode changes require an unsafe block:

```compile_fail
let _ = vescpkg_rs_sys::raw::io_set_mode(
    vescpkg_rs_sys::VescPin(0),
    vescpkg_rs_sys::VescPinMode(0),
);
```

`LispBM` encoding requires an unsafe block:

```compile_fail
let _ = vescpkg_rs_sys::raw::lbm_enc_i(0);
```

Raw extension registration requires an unsafe block:

```compile_fail
extern "C" fn handler(_: *mut u32, _: u32) -> u32 {
    0
}

let _ = vescpkg_rs_sys::raw::lbm_add_extension(core::ptr::null(), handler);
```

`no_std` consumers cannot use `std` accidentally:

```compile_fail
#![no_std]

use std::vec::Vec;

let _empty: Vec<u8> = Vec::new();
```

The mock firmware table remains test-only:

```compile_fail
let _table = vescpkg_rs_sys::test_support::empty_table();
```
