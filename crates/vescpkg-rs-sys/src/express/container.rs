//! Checked views over the VESC Express relocatable native-library container.
//!
//! The container is the ESP32-S3 path described by the pinned Express loader
//! (`main/lispif_c_lib.c`): a big-endian magic followed by little-endian
//! metadata, relocation entries, code, and data. This parser does not allocate
//! or apply relocations; it gives a future target builder/loader a bounded,
//! source-shaped view of the bytes.

use super::types::EXPRESS_NATIVE_LIB_RELOC_MAGIC;

const HEADER_LEN: usize = 24;
const MAX_REGION_BYTES: u32 = 0x40000;

/// Error returned when an Express relocatable native container is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressNativeContainerError {
    /// The container ended before the requested number of bytes.
    Truncated,
    /// The leading magic does not identify an Express relocatable container.
    InvalidMagic {
        /// The decoded big-endian magic.
        found: u32,
    },
    /// The container metadata version is not supported by the pinned loader.
    UnsupportedVersion {
        /// The decoded little-endian container version.
        found: u32,
    },
    /// The code region is outside the loader's bounded, word-aligned range.
    InvalidCodeSize {
        /// The rejected code-region size in bytes.
        found: u32,
    },
    /// The data region is outside the loader's bounded, word-aligned range.
    InvalidDataSize {
        /// The rejected data-region size in bytes.
        found: u32,
    },
    /// The entry point is not a word-aligned offset inside the code region.
    InvalidEntryOffset {
        /// The rejected code-relative entry offset.
        found: u32,
    },
    /// The relocation count exceeds the maximum number of words in both regions.
    InvalidRelocationCount {
        /// The rejected relocation count.
        found: u32,
    },
    /// A relocation points outside the region it patches.
    InvalidRelocation {
        /// The relocation index in the table.
        index: u32,
        /// The rejected region-relative patch offset.
        offset: u32,
    },
}

/// A borrowed, structurally validated Express relocatable native container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressNativeContainer<'a> {
    bytes: &'a [u8],
    version: u32,
    code_size: usize,
    data_size: usize,
    entry_offset: usize,
    reloc_count: usize,
}

impl<'a> ExpressNativeContainer<'a> {
    /// Parse the pinned version-2 relocatable container format.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ExpressNativeContainerError> {
        if bytes.len() < HEADER_LEN {
            return Err(ExpressNativeContainerError::Truncated);
        }

        let magic = read_be_u32(bytes, 0);
        if magic != EXPRESS_NATIVE_LIB_RELOC_MAGIC {
            return Err(ExpressNativeContainerError::InvalidMagic { found: magic });
        }

        let version = read_le_u32(bytes, 4);
        if version != 2 {
            return Err(ExpressNativeContainerError::UnsupportedVersion { found: version });
        }

        let code_size = read_le_u32(bytes, 8);
        if !(4..=MAX_REGION_BYTES).contains(&code_size) || code_size & 3 != 0 {
            return Err(ExpressNativeContainerError::InvalidCodeSize { found: code_size });
        }

        let data_size = read_le_u32(bytes, 12);
        if !(8..=MAX_REGION_BYTES).contains(&data_size) || data_size & 3 != 0 {
            return Err(ExpressNativeContainerError::InvalidDataSize { found: data_size });
        }

        let entry_offset = read_le_u32(bytes, 16);
        if entry_offset >= code_size || entry_offset & 3 != 0 {
            return Err(ExpressNativeContainerError::InvalidEntryOffset {
                found: entry_offset,
            });
        }

        let reloc_count = read_le_u32(bytes, 20);
        if reloc_count > (code_size + data_size) / 4 {
            return Err(ExpressNativeContainerError::InvalidRelocationCount { found: reloc_count });
        }

        let code_size = code_size as usize;
        let data_size = data_size as usize;
        let entry_offset = entry_offset as usize;
        let reloc_count = reloc_count as usize;
        let reloc_bytes = reloc_count.checked_mul(4).ok_or(
            ExpressNativeContainerError::InvalidRelocationCount {
                found: reloc_count as u32,
            },
        )?;
        let code_start = HEADER_LEN
            .checked_add(reloc_bytes)
            .ok_or(ExpressNativeContainerError::Truncated)?;
        let data_start = code_start
            .checked_add(code_size)
            .ok_or(ExpressNativeContainerError::Truncated)?;
        let end = data_start
            .checked_add(data_size)
            .ok_or(ExpressNativeContainerError::Truncated)?;
        if bytes.len() < end {
            return Err(ExpressNativeContainerError::Truncated);
        }

        let container = Self {
            bytes,
            version,
            code_size,
            data_size,
            entry_offset,
            reloc_count,
        };
        for index in 0..reloc_count {
            let relocation = container
                .relocation(index)
                .expect("validated relocation index");
            let region_size = if relocation.patches_data() {
                data_size
            } else {
                code_size
            };
            if relocation.offset() as usize + 4 > region_size {
                return Err(ExpressNativeContainerError::InvalidRelocation {
                    index: index as u32,
                    offset: relocation.offset(),
                });
            }
        }
        Ok(container)
    }

    /// Return the version of the container format.
    pub const fn version(self) -> u32 {
        self.version
    }

    /// Return the code region size in bytes.
    pub const fn code_size(self) -> usize {
        self.code_size
    }

    /// Return the data region size in bytes.
    pub const fn data_size(self) -> usize {
        self.data_size
    }

    /// Return the word-aligned entry offset within the code region.
    pub const fn entry_offset(self) -> usize {
        self.entry_offset
    }

    /// Return the number of relocation entries.
    pub const fn relocation_count(self) -> usize {
        self.reloc_count
    }

    /// Return the validated byte length of the encoded container.
    pub const fn encoded_len(self) -> usize {
        HEADER_LEN + self.reloc_count * 4 + self.code_size + self.data_size
    }

    /// Borrow the code bytes following the relocation table.
    pub fn code(self) -> &'a [u8] {
        let start = HEADER_LEN + self.reloc_count * 4;
        &self.bytes[start..start + self.code_size]
    }

    /// Borrow the data bytes following the code region.
    pub fn data(self) -> &'a [u8] {
        let start = HEADER_LEN + self.reloc_count * 4 + self.code_size;
        &self.bytes[start..start + self.data_size]
    }

    /// Return one validated relocation entry.
    pub fn relocation(self, index: usize) -> Option<ExpressRelocation> {
        if index >= self.reloc_count {
            return None;
        }
        let offset = HEADER_LEN + index * 4;
        Some(ExpressRelocation::from_raw(read_le_u32(self.bytes, offset)))
    }

    /// Iterate over the validated relocation entries in source order.
    pub fn relocations(self) -> ExpressRelocationIter<'a> {
        ExpressRelocationIter {
            container: self,
            next: 0,
        }
    }
}

/// Iterator over a validated Express relocation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressRelocationIter<'a> {
    container: ExpressNativeContainer<'a>,
    next: usize,
}

impl Iterator for ExpressRelocationIter<'_> {
    type Item = ExpressRelocation;

    fn next(&mut self) -> Option<Self::Item> {
        let relocation = self.container.relocation(self.next)?;
        self.next += 1;
        Some(relocation)
    }
}

/// A single Express relocation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressRelocation(u32);

impl ExpressRelocation {
    const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Return whether the patched word lives in the data region.
    pub const fn patches_data(self) -> bool {
        self.0 & 0x4000_0000 != 0
    }

    /// Return whether the word stores a code-region target.
    pub const fn targets_code(self) -> bool {
        self.0 & 0x8000_0000 != 0
    }

    /// Return the region-relative word offset.
    pub const fn offset(self) -> u32 {
        self.0 & 0x3fff_ffff
    }
}

fn read_be_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
