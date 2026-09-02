#[cfg(not(test))]
use core::ffi::c_char;

/// Borrowed UTF-8 text backed by NUL-terminated storage for firmware calls.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FirmwareStr<'a>(&'a str);

impl<'a> FirmwareStr<'a> {
    /// Validate text whose final byte is its only NUL byte.
    #[must_use]
    pub const fn from_str_with_nul(value: &'a str) -> Option<Self> {
        let bytes = value.as_bytes();
        let Some((&0, text)) = bytes.split_last() else {
            return None;
        };
        let mut index = 0;
        while index < text.len() {
            if text[index] == 0 {
                return None;
            }
            index += 1;
        }
        Some(Self(value))
    }

    /// Supply a type-correct value for the unreachable invalid macro branch.
    #[doc(hidden)]
    #[must_use]
    pub const fn __invalid() -> FirmwareStr<'static> {
        FirmwareStr("\0")
    }

    /// Return the Rust text without its firmware terminator.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.0.strip_suffix('\0').unwrap_or(self.0)
    }

    #[cfg(not(test))]
    pub(crate) const fn as_ptr(self) -> *const c_char {
        self.0.as_ptr().cast()
    }
}

impl core::fmt::Debug for FirmwareStr<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(formatter)
    }
}

/// Create borrowed firmware text from a Rust string literal.
#[macro_export]
macro_rules! firmware_str {
    ($value:literal) => {
        const {
            const VALUE: Option<$crate::FirmwareStr<'static>> =
                $crate::FirmwareStr::from_str_with_nul(concat!($value, "\0"));
            const _: [(); 1] = [(); VALUE.is_some() as usize];
            match VALUE {
                Some(value) => value,
                None => $crate::FirmwareStr::__invalid(),
            }
        }
    };
}
