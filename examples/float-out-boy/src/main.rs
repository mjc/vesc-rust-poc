//! Float Out Boy VESC package payload.
//!
//! This crate owns Float Out Boy-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds stay `no_std`; startup state is allocated directly by firmware.
//!
//! Source map: package initialization mirrors Float Out Boy's `start`/`stop` wiring at
//! `third_party/float-out-boy/src/main.c:2401-2460`.

#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]
#![deny(warnings, clippy::pedantic)]
#![deny(unsafe_code)]
#![forbid(unused_extern_crates)]
// An embedded package cannot unwind or print a useful panic report. Keep
// explicit crash shortcuts out of the production entrypoint and its modules.
#![cfg_attr(
    not(test),
    deny(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::mem_forget,
        clippy::missing_safety_doc,
        clippy::multiple_unsafe_ops_per_block,
        clippy::panic,
        clippy::todo,
        clippy::undocumented_unsafe_blocks,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

#[cfg(any(test, not(target_arch = "arm")))]
extern crate std;

#[cfg(test)]
macro_rules! assert_f32_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual: f32 = $actual;
        let expected: f32 = $expected;
        let tolerance = f32::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 4.0;
        let exactly_equal = !actual.is_nan() && actual.to_bits() == expected.to_bits();
        assert!(
            exactly_equal
                || (actual.is_finite()
                    && expected.is_finite()
                    && (actual - expected).abs() <= tolerance),
            "expected {expected:?}, got {actual:?} (tolerance {tolerance:?})"
        );
    }};
}

#[cfg(test)]
macro_rules! assert_f32_ne {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual: f32 = $actual;
        let expected: f32 = $expected;
        let tolerance = f32::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 4.0;
        let exactly_equal = !actual.is_nan() && actual.to_bits() == expected.to_bits();
        assert!(
            !exactly_equal
                && (!actual.is_finite()
                    || !expected.is_finite()
                    || (actual - expected).abs() > tolerance),
            "expected values to differ by more than {tolerance:?}, both were near {actual:?}"
        );
    }};
}

macro_rules! const_field_getters {
    ($( $(#[$attribute:meta])* $visibility:vis fn $name:ident -> $output:ty = $field:ident; )+) => {
        $(
            $(#[$attribute])*
            #[must_use]
            $visibility const fn $name(self) -> $output {
                self.$field
            }
        )+
    };
}

macro_rules! typed_fields {
    (
        $(#[$type_attribute:meta])*
        $visibility:vis struct $name:ident {
            $( $field:ident: $field_type:ty => $getter:ident $(=> $with:ident)?, )+
        }
    ) => {
        $(#[$type_attribute])*
        $visibility struct $name {
            $( $field: $field_type, )+
        }

        impl $name {
            /// Build the typed field group.
            #[must_use]
            pub const fn new($( $field: $field_type, )+) -> Self {
                Self { $( $field, )+ }
            }

            const_field_getters! {
                $(
                    #[doc = concat!("Return the `", stringify!($field), "` field.")]
                    pub fn $getter -> $field_type = $field;
                )+
            }

            $(typed_fields!(@with $field: $field_type $(=> $with)?);)+
        }
    };

    (@with $field:ident: $field_type:ty => $with:ident) => {
        #[doc = concat!("Return this field group with a new `", stringify!($field), "` field.")]
        #[must_use]
        pub const fn $with(mut self, $field: $field_type) -> Self {
            self.$field = $field;
            self
        }
    };

    (@with $field:ident: $field_type:ty) => {};
}

macro_rules! typed_newtype {
    (
        $(#[$type_attribute:meta])*
        $visibility:vis struct $name:ident($inner:ty);
        $constructor:ident($value:ident);
        $getter:ident;
    ) => {
        $(#[$type_attribute])*
        $visibility struct $name($inner);

        impl $name {
            /// Build the typed value.
            #[must_use]
            pub const fn $constructor($value: $inner) -> Self {
                Self($value)
            }

            /// Return the wrapped value.
            #[must_use]
            pub const fn $getter(self) -> $inner {
                self.0
            }
        }
    };
}

macro_rules! wire_enum {
    (
        $(#[$enum_attribute:meta])*
        $visibility:vis enum $name:ident {
            $(
                $(#[$variant_attribute:meta])*
                $variant:ident = $id:literal,
            )+
        }
    ) => {
        $(#[$enum_attribute])*
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $visibility enum $name {
            $(
                $(#[$variant_attribute])*
                $variant = $id,
            )+
        }

        impl $name {
            /// Return the Float Out Boy `v1.2.1` wire ID.
            #[must_use]
            #[expect(
                clippy::as_conversions,
                reason = "the repr(u8) discriminant is the firmware wire value"
            )]
            pub const fn id(self) -> u8 {
                self as u8
            }

            const fn try_from_wire_id(value: u8) -> Result<Self, u8> {
                match value {
                    $($id => Ok(Self::$variant),)+
                    _ => Err(value),
                }
            }
        }

        impl TryFrom<u8> for $name {
            type Error = u8;

            fn try_from(value: u8) -> Result<Self, u8> {
                Self::try_from_wire_id(value)
            }
        }
    };
}

#[cfg(not(target_arch = "arm"))]
fn main() {}

#[cfg(all(not(test), not(target_arch = "arm")))]
#[global_allocator]
static HOST_ALLOCATOR: std::alloc::System = std::alloc::System;
#[cfg(all(not(test), target_arch = "arm"))]
#[global_allocator]
static FIRMWARE_ALLOCATOR: vescpkg_rs::VescAllocator = vescpkg_rs::VescAllocator;

mod balance;
mod beeper;
pub mod bms;
mod config;
pub mod domain;
pub mod extensions;
pub mod footpad;
pub use domain::{FloatOutBoyMode, FloatOutBoyRunState};
pub use footpad::FloatOutBoyFootpadState;
pub mod lcm;
pub mod leds;
mod motor_control;
pub mod package;
mod wire;

vescpkg_rs::package_start!(
    crate::package::start,
    crate::package::FloatOutBoyPackageState,
    crate::package::stop
);

#[cfg(test)]
mod tests;
