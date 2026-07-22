//! Package configuration persisted through the public EEPROM capability.
//!
//! This is intentionally a small usage-shaped example: the package owns its
//! custom-EEPROM address and performs persistence only when its caller asks.

use vescpkg_rs::{CustomEeprom, CustomEepromAddress, EepromWord, Firmware};

/// Read the loopback package's persisted probe value.
pub fn read_probe(firmware: &Firmware) -> Option<u32> {
    read_probe_from(firmware.eeprom())
}

/// Persist one loopback package probe value explicitly.
pub fn write_probe(firmware: &Firmware, value: u32) -> bool {
    write_probe_to(firmware.eeprom(), value)
}

fn read_probe_from(eeprom: &CustomEeprom) -> Option<u32> {
    let address = CustomEepromAddress::from_index(0)?;
    eeprom.read(address).map(EepromWord::to_u32)
}

fn write_probe_to(eeprom: &CustomEeprom, value: u32) -> bool {
    let Some(address) = CustomEepromAddress::from_index(0) else {
        return false;
    };
    eeprom.write(address, EepromWord::from_u32(value))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::{read_probe_from, write_probe_to};
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn package_config_round_trips_through_the_public_eeprom_handle() {
        let firmware = FirmwareTest::new();
        let eeprom = firmware.eeprom();

        assert_eq!(read_probe_from(eeprom), None);
        assert!(write_probe_to(eeprom, 0xfeed_beef));
        assert_eq!(read_probe_from(eeprom), Some(0xfeed_beef));
    }
}
