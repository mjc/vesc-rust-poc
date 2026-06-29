//! Raw/minimal VESC firmware ABI bindings.
//!
//! This crate mirrors the VESC native package ABI. It does not provide
//! high-level vehicle semantics, package building, or host transport code.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.
//!
//! Testing strategy: see `docs/testing/vesc-ffi.md`.

#![no_std]
#![forbid(unused_extern_crates)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(test)]
extern crate std;

mod image;
mod loader;
mod types;
mod vesc_if;

#[cfg(test)]
pub mod test_support;

pub mod raw;
pub mod views;

pub use image::{ImageOffset, NativeAddress, NativeImage};
pub use loader::{AppDataHandler, ExtensionHandler, LibInfo, LibInfoAbi, StopHandler};
pub use types::*;
pub use vesc_if::{VescIfAbi, VescIfSlot};
pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[cfg(test)]
mod tests;
