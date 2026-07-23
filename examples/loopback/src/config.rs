//! Package configuration persisted through the public EEPROM capability.
//!
//! This is intentionally a small usage-shaped example: the package owns its
//! custom-EEPROM address and performs persistence only when its caller asks.

use vescpkg_rs::{CustomConfigImage, CustomEeprom, CustomEepromAddress, Firmware};

const LOOPBACK_CONFIG_LEN: usize = 8;
const LOOPBACK_CONFIG_SIGNATURE: [u8; 4] = *b"VSC!";
const LOOPBACK_CONFIG_EEPROM_WORD: usize = 0;

/// A fixed-size, signature-checked loopback configuration image.
///
/// The first four bytes identify the package-owned image. The remaining bytes
/// hold the probe value in native-endian form, matching the firmware EEPROM
/// word ABI without exposing raw EEPROM words to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopbackConfig(CustomConfigImage<LOOPBACK_CONFIG_LEN>);

impl LoopbackConfig {
    /// Build a configuration image for one probe value.
    #[must_use]
    pub fn new(probe: u32) -> Self {
        let mut bytes = [0_u8; LOOPBACK_CONFIG_LEN];
        bytes[..LOOPBACK_CONFIG_SIGNATURE.len()].copy_from_slice(&LOOPBACK_CONFIG_SIGNATURE);
        bytes[LOOPBACK_CONFIG_SIGNATURE.len()..].copy_from_slice(&probe.to_ne_bytes());
        Self(CustomConfigImage::new(bytes))
    }

    /// Parse a serialized image, rejecting missing or mismatched signatures.
    #[must_use]
    pub fn from_serialized(bytes: &[u8]) -> Option<Self> {
        CustomConfigImage::from_serialized(bytes, LOOPBACK_CONFIG_SIGNATURE).map(Self)
    }

    /// Return the configured probe value.
    #[must_use]
    pub fn probe(self) -> u32 {
        let bytes = self.0.as_bytes();
        let probe = <[u8; 4]>::try_from(&bytes[LOOPBACK_CONFIG_SIGNATURE.len()..])
            .expect("loopback config probe field has fixed size");
        u32::from_ne_bytes(probe)
    }

    fn read_from(eeprom: CustomEeprom) -> Option<Self> {
        let address = CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD)?;
        let mut bytes = [0_u8; LOOPBACK_CONFIG_LEN];
        eeprom.read_bytes_at(address, &mut bytes).ok()?;
        Self::from_serialized(&bytes)
    }

    fn write_to(self, eeprom: CustomEeprom) -> bool {
        let address = CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD);
        address.is_some_and(|address| eeprom.write_bytes_at(address, self.0.as_bytes()).is_ok())
    }
}

/// Read the loopback package's persisted probe value.
pub fn read_probe(firmware: &Firmware) -> Option<u32> {
    LoopbackConfig::read_from(*firmware.eeprom()).map(LoopbackConfig::probe)
}

/// Persist one loopback package probe value explicitly.
pub fn write_probe(firmware: &Firmware, value: u32) -> bool {
    LoopbackConfig::new(value).write_to(*firmware.eeprom())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{LOOPBACK_CONFIG_EEPROM_WORD, LoopbackConfig};
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn package_config_round_trips_through_the_public_eeprom_handle() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();

        assert_eq!(LoopbackConfig::read_from(eeprom), None);
        assert!(LoopbackConfig::new(0xfeed_beef).write_to(eeprom));
        assert_eq!(
            LoopbackConfig::read_from(eeprom).map(LoopbackConfig::probe),
            Some(0xfeed_beef)
        );
    }

    #[test]
    fn package_config_rejects_an_image_without_its_signature() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();
        assert!(eeprom.write_bytes(&[0; 8]).is_ok());
        assert_eq!(LoopbackConfig::read_from(eeprom), None);
    }

    #[test]
    fn package_config_reports_partial_eeprom_writes() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();
        let failed = vescpkg_rs::CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD + 1)
            .expect("probe word fits");
        firmware.fail_eeprom_write(failed);

        assert!(!LoopbackConfig::new(7).write_to(eeprom));
        assert_eq!(LoopbackConfig::read_from(eeprom), None);
    }
}
