//! VESC firmware version values.

/// Firmware version reported by VESC as major, minor, and beta components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareVersion {
    major: i32,
    minor: i32,
    beta: i32,
}

impl FirmwareVersion {
    /// Create a firmware version from the components reported by VESC.
    #[must_use]
    pub const fn new(major: i32, minor: i32, beta: i32) -> Self {
        Self { major, minor, beta }
    }

    /// Return the major version component.
    #[must_use]
    pub const fn major(self) -> i32 {
        self.major
    }

    /// Return the minor version component.
    #[must_use]
    pub const fn minor(self) -> i32 {
        self.minor
    }

    /// Return the beta version component.
    #[must_use]
    pub const fn beta(self) -> i32 {
        self.beta
    }
}

#[cfg(test)]
mod tests {
    use super::FirmwareVersion;

    #[test]
    fn firmware_version_preserves_vesc_components() {
        let version = FirmwareVersion::new(6, 5, 0);

        assert_eq!(
            (version.major(), version.minor(), version.beta()),
            (6, 5, 0)
        );
    }
}
