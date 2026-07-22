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
    assert_eq!(
        settings.get_float(FirmwareFloatSetting::InputCurrentMax),
        60.0
    );
    assert_eq!(settings.motor_current_max().current().as_amps(), 100.0);
    assert_eq!(settings.motor_current_min().current().as_amps(), 100.0);
    assert_eq!(settings.input_current_max().current().as_amps(), 60.0);
    assert_eq!(settings.input_current_min().current().as_amps(), -60.0);
    assert_eq!(settings.absolute_current_max().current().as_amps(), 150.0);
    assert_eq!(
        settings
            .minimum_electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        0.0
    );
    assert_eq!(
        settings
            .maximum_electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        12_000.0
    );
    assert_eq!(settings.input_voltage_min().voltage().as_volts(), 20.0);
    assert_eq!(settings.input_voltage_max().voltage().as_volts(), 60.0);
    assert_eq!(
        settings.battery_cut_start_voltage().voltage().as_volts(),
        30.0
    );
    assert_eq!(
        settings.battery_cut_end_voltage().voltage().as_volts(),
        28.0
    );
    assert_eq!(
        settings
            .mosfet_temperature_start()
            .temperature()
            .as_degrees_celsius(),
        85.0
    );
    assert_eq!(
        settings
            .mosfet_temperature_end()
            .temperature()
            .as_degrees_celsius(),
        90.0
    );
    assert_eq!(
        settings
            .motor_temperature_start()
            .temperature()
            .as_degrees_celsius(),
        85.0
    );
    assert_eq!(
        settings
            .motor_temperature_end()
            .temperature()
            .as_degrees_celsius(),
        95.0
    );
    assert_eq!(settings.duty_cycle_limit().ratio().as_ratio(), 0.95);
    settings
        .set_motor_current_max(vescpkg_rs::MotorCurrentLimit::new(
            vescpkg_rs::Current::from_amps(80.0),
        ))
        .unwrap();
    settings
        .set_motor_current_min(vescpkg_rs::MotorCurrentLimit::new(
            vescpkg_rs::Current::from_amps(40.0),
        ))
        .unwrap();
    settings
        .set_input_current_max(vescpkg_rs::InputCurrent::new(
            vescpkg_rs::Current::from_amps(30.0),
        ))
        .unwrap();
    settings
        .set_input_current_min(vescpkg_rs::InputCurrent::new(
            vescpkg_rs::Current::from_amps(-20.0),
        ))
        .unwrap();
    settings
        .set_absolute_current_max(vescpkg_rs::MotorCurrentLimit::new(
            vescpkg_rs::Current::from_amps(120.0),
        ))
        .unwrap();
    settings
        .set_minimum_electrical_speed(vescpkg_rs::ElectricalSpeed::new(
            vescpkg_rs::Rpm::from_revolutions_per_minute(100.0),
        ))
        .unwrap();
    settings
        .set_maximum_electrical_speed(vescpkg_rs::ElectricalSpeed::new(
            vescpkg_rs::Rpm::from_revolutions_per_minute(10_000.0),
        ))
        .unwrap();
    settings
        .set_input_voltage_min(vescpkg_rs::InputVoltage::new(
            vescpkg_rs::Voltage::from_volts(24.0),
        ))
        .unwrap();
    settings
        .set_input_voltage_max(vescpkg_rs::InputVoltage::new(
            vescpkg_rs::Voltage::from_volts(54.0),
        ))
        .unwrap();
    settings
        .set_battery_cut_start_voltage(vescpkg_rs::InputVoltage::new(
            vescpkg_rs::Voltage::from_volts(31.0),
        ))
        .unwrap();
    settings
        .set_battery_cut_end_voltage(vescpkg_rs::InputVoltage::new(
            vescpkg_rs::Voltage::from_volts(29.0),
        ))
        .unwrap();
    settings
        .set_mosfet_temperature_start(vescpkg_rs::TemperatureLimitStart::new(
            vescpkg_rs::Temperature::from_degrees_celsius(75.0),
        ))
        .unwrap();
    settings
        .set_mosfet_temperature_end(vescpkg_rs::TemperatureLimitEnd::new(
            vescpkg_rs::Temperature::from_degrees_celsius(85.0),
        ))
        .unwrap();
    settings
        .set_motor_temperature_start(vescpkg_rs::TemperatureLimitStart::new(
            vescpkg_rs::Temperature::from_degrees_celsius(80.0),
        ))
        .unwrap();
    settings
        .set_motor_temperature_end(vescpkg_rs::TemperatureLimitEnd::new(
            vescpkg_rs::Temperature::from_degrees_celsius(90.0),
        ))
        .unwrap();
    settings
        .set_duty_cycle_limit(vescpkg_rs::DutyCycleLimit::new(
            vescpkg_rs::Ratio::from_ratio_const(0.8),
        ))
        .unwrap();
    assert_eq!(settings.motor_current_max().current().as_amps(), 80.0);
    assert_eq!(settings.motor_current_min().current().as_amps(), 40.0);
    assert_eq!(settings.input_current_max().current().as_amps(), 30.0);
    assert_eq!(settings.input_current_min().current().as_amps(), -20.0);
    assert_eq!(settings.absolute_current_max().current().as_amps(), 120.0);
    assert_eq!(
        settings
            .minimum_electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        100.0
    );
    assert_eq!(
        settings
            .maximum_electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        10_000.0
    );
    assert_eq!(settings.input_voltage_min().voltage().as_volts(), 24.0);
    assert_eq!(settings.input_voltage_max().voltage().as_volts(), 54.0);
    assert_eq!(
        settings.battery_cut_start_voltage().voltage().as_volts(),
        31.0
    );
    assert_eq!(
        settings.battery_cut_end_voltage().voltage().as_volts(),
        29.0
    );
    assert_eq!(
        settings
            .mosfet_temperature_start()
            .temperature()
            .as_degrees_celsius(),
        75.0
    );
    assert_eq!(
        settings
            .mosfet_temperature_end()
            .temperature()
            .as_degrees_celsius(),
        85.0
    );
    assert_eq!(
        settings
            .motor_temperature_start()
            .temperature()
            .as_degrees_celsius(),
        80.0
    );
    assert_eq!(
        settings
            .motor_temperature_end()
            .temperature()
            .as_degrees_celsius(),
        90.0
    );
    assert_eq!(settings.duty_cycle_limit().ratio().as_ratio(), 0.8);
    settings
        .set_float(FirmwareFloatSetting::InputCurrentMax, 24.0)
        .unwrap();
    assert_eq!(
        settings.get_float(FirmwareFloatSetting::InputCurrentMax),
        24.0
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
    assert_eq!(settings.get_int(FirmwareIntSetting::AppCanMode), 2);
    settings.set_int(FirmwareIntSetting::AppCanMode, 1).unwrap();
    assert_eq!(settings.get_int(FirmwareIntSetting::AppCanMode), 1);
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
