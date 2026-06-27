//! Raw VESC firmware ABI surface.
//!
//! Mirrors the firmware C ABI: loader metadata, function-table slots, native
//! image rebasing, scalar/view wrappers, and direct firmware table calls.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

mod image;
mod loader;
mod types;
mod vesc_if;

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
