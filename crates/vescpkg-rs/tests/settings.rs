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
    assert_eq!(settings.battery_cell_count().unwrap().as_u16(), 12);
    settings
        .set_battery_cell_count(vescpkg_rs::BatteryCellCount::try_new(14).unwrap())
        .unwrap();
    assert_eq!(settings.battery_cell_count().unwrap().as_u16(), 14);
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

#[test]
fn settings_reject_non_finite_float_values_before_abi_call() {
    let firmware = FirmwareTest::new();
    let settings = firmware.settings();

    assert_eq!(
        settings.set_float(FirmwareFloatSetting::MaxDuty, f32::NAN),
        Err(SettingsError::InvalidValue)
    );
}

#[test]
fn settings_reject_malformed_battery_cell_count_reads() {
    let firmware = FirmwareTest::new().with_raw_battery_cell_count(0);
    assert_eq!(
        firmware.settings().battery_cell_count(),
        Err(SettingsError::InvalidValue)
    );

    let firmware = firmware.with_raw_battery_cell_count(-1);
    assert_eq!(
        firmware.settings().battery_cell_count(),
        Err(SettingsError::InvalidValue)
    );
}
