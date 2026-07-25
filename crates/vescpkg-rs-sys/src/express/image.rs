//! Checked views over the VESC Express native-library image formats.
//!
//! The pinned Express loader accepts an in-place XIP image on ESP32-C3, C6,
//! and P4, and a versioned relocatable container on ESP32-S3. This module
//! selects that format from [`ExpressTarget`] so a package builder cannot
//! silently hand the wrong image kind to a firmware target.

use super::container::{ExpressNativeContainer, ExpressNativeContainerError};
use super::types::{EXPRESS_NATIVE_LIB_MAGIC, ExpressNativeLoadKind, ExpressTarget};

const XIP_HEADER_LEN: usize = 8;
const XIP_MIN_IMAGE_LEN: usize = 13;

/// Error returned when the XIP native-library image is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressNativeXipError {
    /// The loader would reject an image that is 12 bytes or shorter.
    Truncated,
    /// The leading magic does not identify an XIP Express image.
    InvalidMagic {
        /// The decoded big-endian magic.
        found: u32,
    },
}

/// Error returned when a target-selected Express native image is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressNativeImageError {
    /// The selected XIP image failed validation.
    Xip(ExpressNativeXipError),
    /// The selected relocatable container failed validation.
    Relocatable(ExpressNativeContainerError),
}

/// A borrowed, structurally validated Express XIP image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressNativeXipImage<'a> {
    bytes: &'a [u8],
}

impl<'a> ExpressNativeXipImage<'a> {
    /// Parse the XIP image shape accepted by the pinned Express loader.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ExpressNativeXipError> {
        if bytes.len() < XIP_MIN_IMAGE_LEN {
            return Err(ExpressNativeXipError::Truncated);
        }
        let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != EXPRESS_NATIVE_LIB_MAGIC {
            return Err(ExpressNativeXipError::InvalidMagic { found: magic });
        }
        Ok(Self { bytes })
    }

    /// Return the validated encoded image length.
    pub const fn encoded_len(self) -> usize {
        self.bytes.len()
    }

    /// Return the entry offset used by the Express loader.
    pub const fn entry_offset(self) -> usize {
        XIP_HEADER_LEN
    }

    /// Borrow the bytes after the magic and program-address header.
    pub fn code(self) -> &'a [u8] {
        &self.bytes[XIP_HEADER_LEN..]
    }
}

/// A target-selected, validated Express native-library image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressNativeImage<'a> {
    /// An in-place image for an XIP target.
    Xip(ExpressNativeXipImage<'a>),
    /// A relocatable image for ESP32-S3.
    Relocatable(ExpressNativeContainer<'a>),
}

impl<'a> ExpressNativeImage<'a> {
    /// Parse the image format required by `target`.
    pub fn parse(target: ExpressTarget, bytes: &'a [u8]) -> Result<Self, ExpressNativeImageError> {
        match target.native_load_kind() {
            ExpressNativeLoadKind::Xip => ExpressNativeXipImage::parse(bytes)
                .map(Self::Xip)
                .map_err(ExpressNativeImageError::Xip),
            ExpressNativeLoadKind::Relocatable => ExpressNativeContainer::parse(bytes)
                .map(Self::Relocatable)
                .map_err(ExpressNativeImageError::Relocatable),
        }
    }

    /// Return the loader image kind selected by the target.
    pub const fn load_kind(self) -> ExpressNativeLoadKind {
        match self {
            Self::Xip(_) => ExpressNativeLoadKind::Xip,
            Self::Relocatable(_) => ExpressNativeLoadKind::Relocatable,
        }
    }

    /// Return the validated encoded image length.
    pub const fn encoded_len(self) -> usize {
        match self {
            Self::Xip(image) => image.encoded_len(),
            Self::Relocatable(container) => container.encoded_len(),
        }
    }

    /// Return the loader entry offset within the encoded image.
    pub const fn entry_offset(self) -> usize {
        match self {
            Self::Xip(image) => image.entry_offset(),
            Self::Relocatable(container) => container.entry_offset(),
        }
    }
}
