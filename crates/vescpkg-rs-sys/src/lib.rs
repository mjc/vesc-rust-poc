//! Raw/minimal VESC firmware ABI bindings.
//!
//! This crate mirrors the VESC native package ABI. It does not provide
//! high-level vehicle semantics, package building, or host transport code.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.
//!
//! Testing strategy: see `docs/testing/vescpkg-rs-sys.md`.

#![doc = include_str!("compile_fail_contracts.md")]
#![no_std]
#![deny(warnings, clippy::all, clippy::pedantic)]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]
// Embedded callers cannot recover from an unwind. Raw wrappers therefore use
// inert typed values when a C function-table slot is unexpectedly absent.
#![cfg_attr(
    not(test),
    deny(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]
#[cfg(test)]
extern crate std;

// These tests verify values crossing the C ABI exactly. Comparing the IEEE-754
// bit patterns makes that intent explicit and avoids accidentally replacing an
// ABI check with an approximate numerical comparison.
#[cfg(test)]
macro_rules! assert_f32_eq {
    ($left:expr, $right:expr $(,)?) => {{
        let left: f32 = $left;
        let right: f32 = $right;
        assert_eq!(left.to_bits(), right.to_bits());
    }};
}

#[cfg(test)]
macro_rules! assert_f64_eq {
    ($left:expr, $right:expr $(,)?) => {{
        let left: f64 = $left;
        let right: f64 = $right;
        assert_eq!(left.to_bits(), right.to_bits());
    }};
}

mod image;
mod c_vesc_if {
    include!(concat!(env!("OUT_DIR"), "/c_vesc_if.rs"));
}

// bindgen copies type names from the C header because those names are part of
// the interface we must match. C uses names such as `systime_t` and `HW_TYPE`;
// Rust normally spells types in `UpperCamelCase`. This expectation applies
// only to generated declarations. Handwritten Rust remains fully linted.
#[expect(
    dead_code,
    clippy::type_complexity,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    reason = "generated declarations must preserve the VESC C ABI"
)]
mod bindgen {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod loader;
mod types;
mod vesc_if;

#[cfg(test)]
pub mod test_support;

/// Raw firmware layout mirrors used when host code needs to inspect payloads directly.
pub mod raw;
/// Typed borrowed views over raw firmware packet bytes.
pub mod views;

pub use image::{ImageOffset, NativeAddress, NativeImage};
pub use loader::{AppDataHandler, ExtensionHandler, LibInfo, LibInfoAbi, StopHandler};
pub use types::*;
pub use vesc_if::{
    AbiError, Stm32AbiRevision, VescIfAbi, VescIfManifestEntry, VescIfPresence, VescIfSlot,
    VescIfSlotKind,
};
pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[cfg(test)]
mod tests;
