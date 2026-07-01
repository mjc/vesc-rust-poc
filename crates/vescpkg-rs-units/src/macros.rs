macro_rules! scalar_unit {
    ($name:ident, $from:ident, $as:ident, $unit:literal) => {
        #[doc = concat!("Generic measurement value stored in ", $unit, ".")]
        #[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(f32);

        impl $name {
            #[doc = concat!("Create a value from ", $unit, ".")]
            pub const fn $from(value: f32) -> Self {
                Self(value)
            }

            #[doc = concat!("Return this value in ", $unit, ".")]
            pub const fn $as(self) -> f32 {
                self.0
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
    ($name:ident, $from:ident, $as:ident, $min:expr, $max:expr, $unit:literal) => {
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
            pub const fn $from(value: f32) -> Result<Self, crate::BoundedUnitError> {
                if value >= Self::MIN && value <= Self::MAX {
                    Ok(Self(value))
                } else {
                    Err(crate::BoundedUnitError::new(value, Self::MIN, Self::MAX))
                }
            }

            #[doc = concat!("Clamp a primitive value into the valid ", $unit, " range.")]
            pub const fn clamped(value: f32) -> Self {
                if value != value || value < Self::MIN {
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
