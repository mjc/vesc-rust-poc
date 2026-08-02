//! Package configuration persisted through the public EEPROM capability.
//!
//! This is intentionally a small usage-shaped example: the package owns its
//! custom-EEPROM address and performs persistence only when its caller asks.
//! It adapts the signature-checked storage shape from VESC's official
//! [`c_libs/examples/config`](https://github.com/vedderb/vesc_pkg/blob/ddf1e162d5b7d01d848263af317cc7f8f14c0d14/c_libs/examples/config/code.c)
//! example without carrying its generated application-specific schema into the SDK.

use vescpkg_rs::{CustomConfigImage, CustomEeprom, CustomEepromAddress, Firmware, FirmwareEffects};

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
        let [signature_0, signature_1, signature_2, signature_3] = LOOPBACK_CONFIG_SIGNATURE;
        let [probe_0, probe_1, probe_2, probe_3] = probe.to_ne_bytes();
        Self(CustomConfigImage::new([
            signature_0,
            signature_1,
            signature_2,
            signature_3,
            probe_0,
            probe_1,
            probe_2,
            probe_3,
        ]))
    }

    /// Parse a serialized image, rejecting missing or mismatched signatures.
    #[must_use]
    pub fn from_serialized(bytes: &[u8]) -> Option<Self> {
        CustomConfigImage::from_serialized(bytes, LOOPBACK_CONFIG_SIGNATURE).map(Self)
    }

    /// Return the configured probe value.
    #[must_use]
    pub fn probe(self) -> u32 {
        let [_, _, _, _, probe_0, probe_1, probe_2, probe_3] = *self.0.as_bytes();
        u32::from_ne_bytes([probe_0, probe_1, probe_2, probe_3])
    }

    fn read_from(eeprom: CustomEeprom, effects: &FirmwareEffects) -> Option<Self> {
        let address = CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD)?;
        let mut bytes = [0_u8; LOOPBACK_CONFIG_LEN];
        eeprom.read_bytes_at(effects, address, &mut bytes).ok()?;
        Self::from_serialized(&bytes)
    }

    fn write_to(self, eeprom: CustomEeprom, effects: &FirmwareEffects) -> bool {
        let address = CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD);
        address.is_some_and(|address| {
            eeprom
                .write_bytes_at(effects, address, self.0.as_bytes())
                .is_ok()
        })
    }
}

/// Read the loopback package's persisted probe value.
pub fn read_probe(firmware: &Firmware, effects: &FirmwareEffects) -> Option<u32> {
    LoopbackConfig::read_from(*firmware.eeprom(), effects).map(LoopbackConfig::probe)
}

/// Persist one loopback package probe value explicitly.
#[must_use]
pub fn write_probe(firmware: &Firmware, effects: &FirmwareEffects, value: u32) -> bool {
    LoopbackConfig::new(value).write_to(*firmware.eeprom(), effects)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{LOOPBACK_CONFIG_EEPROM_WORD, LoopbackConfig};
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn package_config_round_trips_through_the_public_eeprom_handle() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();
        let effects = firmware.effects();

        assert_eq!(LoopbackConfig::read_from(eeprom, effects), None);
        assert!(LoopbackConfig::new(0xfeed_beef).write_to(eeprom, effects));
        assert_eq!(
            LoopbackConfig::read_from(eeprom, effects).map(LoopbackConfig::probe),
            Some(0xfeed_beef)
        );
    }

    #[test]
    fn package_config_rejects_an_image_without_its_signature() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();
        let effects = firmware.effects();
        assert!(eeprom.write_bytes(effects, &[0; 8]).is_ok());
        assert_eq!(LoopbackConfig::read_from(eeprom, effects), None);
    }

    #[test]
    fn package_config_reports_partial_eeprom_writes() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();
        let effects = firmware.effects();
        let failed = vescpkg_rs::CustomEepromAddress::from_index(LOOPBACK_CONFIG_EEPROM_WORD + 1)
            .expect("probe word fits");
        firmware.fail_eeprom_write(failed);

        assert!(!LoopbackConfig::new(7).write_to(eeprom, effects));
        assert_eq!(LoopbackConfig::read_from(eeprom, effects), None);
    }
}
