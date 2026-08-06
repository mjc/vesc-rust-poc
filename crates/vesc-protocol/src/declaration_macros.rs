/// Generate copy-value getters for named fields.
#[macro_export]
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

/// Generate copy-value getters forwarded through a named field.
#[macro_export]
macro_rules! const_forward_getters {
    ($( $(#[$attribute:meta])* $visibility:vis fn $name:ident -> $output:ty = $field:ident.$getter:ident(); )+) => {
        $(
            $(#[$attribute])*
            #[must_use]
            $visibility const fn $name(self) -> $output {
                self.$field.$getter()
            }
        )+
    };
}

/// Generate const copy-value builders for named fields.
#[macro_export]
macro_rules! const_field_builders {
    ($( $(#[$attribute:meta])* $visibility:vis fn $name:ident($value:ident: $value_type:ty) => $field:ident; )+) => {
        $(
            $(#[$attribute])*
            #[must_use]
            $visibility const fn $name(mut self, $value: $value_type) -> Self {
                self.$field = $value;
                self
            }
        )+
    };
}

/// Declare a typed field group with a const constructor, getters, and optional builders.
#[macro_export]
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
            #[allow(clippy::too_many_arguments)]
            #[must_use]
            pub const fn new($( $field: $field_type, )+) -> Self {
                Self { $( $field, )+ }
            }

            $crate::const_field_getters! {
                $(
                    #[doc = concat!("Return the `", stringify!($field), "` field.")]
                    pub fn $getter -> $field_type = $field;
                )+
            }

            $($crate::typed_fields!(@with $field: $field_type $(=> $with)?);)+
        }
    };

    (@with $field:ident: $field_type:ty => $with:ident) => {
        $crate::const_field_builders! {
            #[doc = concat!("Return this field group with a new `", stringify!($field), "` field.")]
            pub fn $with($field: $field_type) => $field;
        }
    };

    (@with $field:ident: $field_type:ty) => {};
}

/// Declare multiple typed field groups with shared attributes.
#[macro_export]
macro_rules! typed_field_groups {
    (
        attributes { $(#[$common_attribute:meta])* }
        $($groups:tt)+
    ) => {
        $crate::typed_field_groups!(@emit [$(#[$common_attribute])*] $($groups)+);
    };

    (@emit [$($common_attribute:tt)*]) => {};

    (@emit [$($common_attribute:tt)*]
        $(#[$type_attribute:meta])*
        $visibility:vis struct $name:ident {
            $( $field:ident: $field_type:ty => $getter:ident $(=> $with:ident)?, )+
        }
        $($remaining:tt)*
    ) => {
        $crate::typed_fields! {
            $($common_attribute)*
            $(#[$type_attribute])*
            $visibility struct $name {
                $( $field: $field_type => $getter $(=> $with)?, )+
            }
        }
        $crate::typed_field_groups!(@emit [$($common_attribute)*] $($remaining)*);
    };
}

/// Declare a typed newtype with a const constructor and getter.
#[macro_export]
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

/// Declare multiple typed newtypes with shared attributes.
#[macro_export]
macro_rules! typed_newtypes {
    (
        attributes { $(#[$common_attribute:meta])* }
        $($types:tt)+
    ) => {
        $crate::typed_newtypes!(@emit [$(#[$common_attribute])*] $($types)+);
    };

    (@emit [$($common_attribute:tt)*]) => {};

    (@emit [$($common_attribute:tt)*]
        $(#[$type_attribute:meta])*
        $visibility:vis struct $name:ident($inner:ty)
            => $constructor:ident($value:ident), $getter:ident;
        $($remaining:tt)*
    ) => {
        $crate::typed_newtype! {
            $($common_attribute)*
            $(#[$type_attribute])*
            $visibility struct $name($inner);
            $constructor($value);
            $getter;
        }
        $crate::typed_newtypes!(@emit [$($common_attribute)*] $($remaining)*);
    };
}

/// Declare a `u8` wire enum with exact ID conversion in both directions.
#[macro_export]
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
            /// Return the wire ID.
            #[must_use]
            #[expect(
                clippy::as_conversions,
                reason = "the repr(u8) discriminant is the wire value"
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
