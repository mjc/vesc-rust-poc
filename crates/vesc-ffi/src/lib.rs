//! Minimal VESC ABI crate.
//!
//! This crate mirrors the firmware C ABI and keeps semantic Rust domain types
//! out of the raw boundary. It exposes raw scalar wrappers, view wrappers, and
//! firmware-facing helper APIs, but it does not define the later ergonomic
//! `vesc-types` / `vesc-units` surface.
#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

mod bindings;
mod extension;
mod image;
mod lifecycle;
mod loader;
mod vesc_if;

mod types;
pub use types::*;

pub mod views;

pub use views::{
    AppDataPacket, CanPayload, CommandPacket, ConfigPayload, ConfigXmlBytes, MutablePacket,
    NvmBytes, PlotAxisName, PlotGraphName, ReplyPacket, ThreadName,
};

#[cfg(not(test))]
pub use bindings::RealBindings;
pub use bindings::{AppDataBindings, LbmBindings};
pub use extension::{ExtensionDescriptor, ExtensionNameError, RegisterError};
pub use image::{ImageOffset, NativeAddress, NativeImage};
pub use lifecycle::{LbmApi, LoopbackLifecycle, PackageLifecycle};
pub use loader::{AppDataHandler, ExtensionHandler, LibInfo, LibInfoAbi, StopHandler};
pub use vesc_if::{VescIfAbi, VescIfSlot};

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod raw;

#[cfg(test)]
mod tests;
