macro_rules! scalar_unit {
    ($name:ident, $from:ident, $as:ident, $unit:literal) => {
        #[doc = concat!("Generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Zero value for this unit.
            pub const ZERO: Self = Self(0.0);

            #[doc = concat!("Create a value from ", $unit, ".")]
            pub const fn $from(value: f32) -> Self {
                Self(value)
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f32 {
                self.0
            }

            /// Return the absolute magnitude in the same unit.
            #[must_use]
            pub const fn abs(self) -> Self {
                Self(self.0.abs())
            }

            /// Return -1.0 for negative values and 1.0 otherwise, matching VESC `SIGN`.
            pub const fn signum(self) -> f32 {
                if self.0 < 0.0 { -1.0 } else { 1.0 }
            }

            /// Return true when this value is greater than zero.
            pub const fn is_positive(self) -> bool {
                self.0 > 0.0
            }

            /// Return true when this value is less than zero.
            pub const fn is_negative(self) -> bool {
                self.0 < 0.0
            }

            /// Return true when this value is exactly zero.
            pub const fn is_zero(self) -> bool {
                self.0 == 0.0
            }

            /// Return the smaller same-unit value.
            #[must_use]
            pub fn min(self, rhs: Self) -> Self {
                Self(self.0.min(rhs.0))
            }

            /// Return the larger same-unit value.
            #[must_use]
            pub fn max(self, rhs: Self) -> Self {
                Self(self.0.max(rhs.0))
            }
        }

        impl core::ops::Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl core::ops::Mul<f32> for $name {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl core::ops::Div<f32> for $name {
            type Output = Self;

            fn div(self, rhs: f32) -> Self::Output {
                Self(self.0 / rhs)
            }
        }

        impl core::ops::Div for $name {
            type Output = f32;

            fn div(self, rhs: Self) -> Self::Output {
                self.0 / rhs.0
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }
    };
}

macro_rules! scalar_unit_f64 {
    ($name:ident, $from:ident, $as:ident, $unit:literal) => {
        #[doc = concat!("Generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f64);

        impl $name {
            #[doc = concat!("Create a value from ", $unit, ".")]
            pub const fn $from(value: f64) -> Self {
                Self(value)
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f64 {
                self.0
            }
        }
    };
}

macro_rules! scalar_int_unit {
    ($name:ident, $from:ident, $as:ident, $storage:ty, $unit:literal) => {
        #[doc = concat!("Generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name($storage);

        impl $name {
            #[doc = concat!("Create a value from ", $unit, ".")]
            pub const fn $from(value: $storage) -> Self {
                Self(value)
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> $storage {
                self.0
            }
        }
    };
}

macro_rules! bounded_unit {
    ($name:ident, $from:ident, $from_const:ident, $as:ident, $min:expr, $max:expr, $unit:literal) => {
        #[doc = concat!("Bounded generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            /// Inclusive lower bound for this unit.
            pub const MIN: f32 = $min;

            /// Inclusive upper bound for this unit.
            pub const MAX: f32 = $max;

            #[doc = concat!("Create a checked value from ", $unit, ".")]
            ///
            /// # Errors
            ///
            /// Returns an error when `value` is outside the inclusive bounds.
            pub const fn $from(value: f32) -> Result<Self, crate::BoundedUnitError> {
                if value >= Self::MIN && value <= Self::MAX {
                    Ok(Self(value))
                } else {
                    Err(crate::BoundedUnitError::new(value, Self::MIN, Self::MAX))
                }
            }

            #[doc = concat!("Create a known-good package constant from ", $unit, ".")]
            ///
            /// This is for embedded configuration constants that should fail at compile time
            /// if the value is invalid. Use the checked constructor for runtime input.
            pub const fn $from_const(value: f32) -> Self {
                match Self::$from(value) {
                    Ok(value) => value,
                    Err(_) => panic!(concat!("invalid ", $unit, " constant")),
                }
            }

            #[doc = concat!("Clamp a primitive value into the valid ", $unit, " range.")]
            pub const fn clamped(value: f32) -> Self {
                if value.is_nan() || value < Self::MIN {
                    Self(Self::MIN)
                } else if value > Self::MAX {
                    Self(Self::MAX)
                } else {
                    Self(value)
                }
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f32 {
                self.0
            }
        }
    };
}

pub(crate) use bounded_unit;
pub(crate) use scalar_int_unit;
pub(crate) use scalar_unit;
pub(crate) use scalar_unit_f64;
