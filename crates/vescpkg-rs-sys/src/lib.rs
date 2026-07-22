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
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_safety_doc)]

#[cfg(test)]
extern crate std;

mod image;
#[allow(dead_code)]
mod c_vesc_if {
    include!(concat!(env!("OUT_DIR"), "/c_vesc_if.rs"));
}

#[allow(
    clippy::all,
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unsafe_op_in_unsafe_fn
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
