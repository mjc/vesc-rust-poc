#![cfg(feature = "test-support")]
#![allow(missing_docs)]

use vescpkg_rs::test_support::FirmwareTest;
use vescpkg_rs::{FirmwareFloatSetting, FirmwareIntSetting, FirmwareSettings, SettingsError};

#[test]
fn typed_settings_read_write_and_persist() {
    let firmware = FirmwareTest::new();
    let settings: &FirmwareSettings = firmware.settings();

    assert_eq!(
        settings.get_float(FirmwareFloatSetting::MotorCurrentMax),
        100.0
    );
    settings
        .set_float(FirmwareFloatSetting::MotorCurrentMax, 42.0)
        .unwrap();
    assert_eq!(
        settings.get_float(FirmwareFloatSetting::MotorCurrentMax),
        42.0
    );

    settings
        .set_int(FirmwareIntSetting::BatteryCellCount, 12)
        .unwrap();
    assert_eq!(settings.get_int(FirmwareIntSetting::BatteryCellCount), 12);
    settings.store().unwrap();
}

#[test]
fn settings_report_firmware_rejections() {
    let firmware = FirmwareTest::new();
    let settings = firmware.settings();

    firmware.fail_settings_writes();
    assert!(matches!(
        settings.set_int(FirmwareIntSetting::BatteryCellCount, 8),
        Err(SettingsError::Rejected {
            operation: "integer setting"
        })
    ));

    firmware.fail_settings_store();
    assert!(matches!(
        settings.store(),
        Err(SettingsError::Rejected {
            operation: "settings persistence"
        })
    ));
}
