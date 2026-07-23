#![cfg(feature = "test-support")]
//! Integration coverage for typed motor handbrake commands.

use vescpkg_rs::{
    AmpHoursCharged, AmpHoursDischarged, AngleDegrees, BatteryCellCount, BatteryLevel,
    BrakeCurrent, BrakeCurrentRelative, Charge, Current, CurrentOffDelay, CurrentRelative,
    DCurrent, DirectionalMotorCurrent, DutyCycle, DutyCycleLimit, ElectricalSpeed, Energy,
    EnergyCounterReset, FirmwareFault, FirmwareFaultId, HandbrakeCurrent, HandbrakeRelative,
    InputCurrent, MotorCurrentLimit, MotorOutput, MotorReleaseOutcome, MotorSelection,
    MotorTelemetry, OdometerMeters, OpenLoopCurrent, OpenLoopPhase, PidPosition,
    PidPositionOffsetPersistence, PwmCallbackError, Ratio, Rpm, SignedRatio, Speed,
    TachometerReset, TachometerSteps, Temperature, TemperatureLimitStart, TotalMotorCurrent,
    VehicleSpeed, VescSeconds, WattHoursRemaining,
};

unsafe extern "C" fn test_pwm_callback() {}

#[test]
#[allow(clippy::too_many_lines)]
fn motor_exposes_typed_handbrake_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new()
        .with_runtime_motor(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1500.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(4.5)),
            TotalMotorCurrent::new(Current::from_amps(10.0)),
            InputCurrent::new(Current::from_amps(6.0)),
            DutyCycle::new(SignedRatio::from_ratio_const(-0.2)),
        )
        .with_motor_current_limits(
            MotorCurrentLimit::new(Current::from_amps(40.0)),
            MotorCurrentLimit::new(Current::from_amps(30.0)),
        )
        .with_duty_cycle_limit(DutyCycleLimit::new(Ratio::from_ratio_const(0.85)))
        .with_temperature_limit_starts(
            TemperatureLimitStart::new(Temperature::from_degrees_celsius(70.0)),
            TemperatureLimitStart::new(Temperature::from_degrees_celsius(80.0)),
        )
        .with_battery_cell_count(BatteryCellCount::try_new(12).unwrap())
        .with_ride_totals(
            OdometerMeters::from_meters(0),
            AmpHoursDischarged::new(Charge::from_amp_hours(1.25)),
            AmpHoursCharged::new(Charge::from_amp_hours(2.5)),
            vescpkg_rs::WattHoursDischarged::new(Energy::from_watt_hours(10.0)),
            vescpkg_rs::WattHoursCharged::new(Energy::from_watt_hours(4.0)),
            BatteryLevel::from_fraction(0.0),
        )
        .with_battery_level_remaining(WattHoursRemaining::new(Energy::from_watt_hours(321.0)))
        .with_directional_motor_current(DirectionalMotorCurrent::new(Current::from_amps(-11.0)))
        .with_d_axis_current(Some(DCurrent::new(Current::from_amps(1.5))))
        .with_firmware_fault(FirmwareFault::Active(FirmwareFaultId::OverTemperatureFet));
    firmware
        .motor()
        .set_handbrake(HandbrakeCurrent::new(Current::from_amps(2.0)));
    firmware
        .motor()
        .set_handbrake_relative(HandbrakeRelative::new(Ratio::from_ratio_const(0.25)));
    firmware
        .motor()
        .set_current_relative(CurrentRelative::new(SignedRatio::from_ratio_const(0.4)));
    firmware
        .motor()
        .set_brake_current_relative(BrakeCurrentRelative::new(Ratio::from_ratio_const(0.3)));

    let telemetry = firmware.telemetry();
    assert!(firmware.motor().dc_calibration_done());
    assert_eq!(firmware.motor().selected_motor().index(), 1);
    assert_eq!(
        telemetry
            .electrical_speed()
            .rpm()
            .as_revolutions_per_minute(),
        1500.0
    );
    assert_eq!(
        telemetry.vehicle_speed().speed().as_meters_per_second(),
        4.5
    );
    assert_eq!(telemetry.motor_current().current().as_amps(), 10.0);
    assert_eq!(
        telemetry.directional_motor_current().current().as_amps(),
        -11.0
    );
    assert_eq!(telemetry.drive_current_limit().current().as_amps(), 40.0);
    assert_eq!(telemetry.brake_current_limit().current().as_amps(), 30.0);
    assert_eq!(
        telemetry
            .mosfet_temperature_limit_start()
            .temperature()
            .as_degrees_celsius(),
        70.0
    );
    assert_eq!(
        telemetry
            .motor_temperature_limit_start()
            .temperature()
            .as_degrees_celsius(),
        80.0
    );
    assert_eq!(telemetry.duty_cycle_limit().ratio().as_ratio(), 0.85);
    assert_eq!(telemetry.battery_cell_count().unwrap().as_u16(), 12);
    assert_eq!(telemetry.battery_current().current().as_amps(), 6.0);
    assert_eq!(
        telemetry
            .amp_hours_discharged_with(EnergyCounterReset::Reset)
            .charge()
            .as_amp_hours(),
        1.25
    );
    assert_eq!(
        telemetry
            .amp_hours_charged_with(EnergyCounterReset::Reset)
            .charge()
            .as_amp_hours(),
        2.5
    );
    assert_eq!(
        telemetry
            .watt_hours_discharged_with(EnergyCounterReset::Reset)
            .energy()
            .as_watt_hours(),
        10.0
    );
    assert_eq!(
        telemetry
            .watt_hours_charged_with(EnergyCounterReset::Reset)
            .energy()
            .as_watt_hours(),
        4.0
    );
    assert_eq!(
        telemetry
            .amp_hours_discharged_with(EnergyCounterReset::Preserve)
            .charge()
            .as_amp_hours(),
        0.0
    );
    assert_eq!(
        telemetry
            .amp_hours_charged_with(EnergyCounterReset::Preserve)
            .charge()
            .as_amp_hours(),
        0.0
    );
    assert_eq!(
        telemetry
            .watt_hours_discharged_with(EnergyCounterReset::Preserve)
            .energy()
            .as_watt_hours(),
        0.0
    );
    assert_eq!(
        telemetry
            .watt_hours_charged_with(EnergyCounterReset::Preserve)
            .energy()
            .as_watt_hours(),
        0.0
    );
    let battery = telemetry.battery_level_snapshot();
    assert_eq!(battery.level().as_fraction(), 0.0);
    assert_eq!(
        battery.watt_hours_remaining().energy().as_watt_hours(),
        321.0
    );
    assert_eq!(telemetry.duty_cycle().ratio().as_ratio(), -0.2);
    let pwm_lease = unsafe {
        firmware
            .motor()
            .register_pwm_callback(test_pwm_callback)
            .unwrap()
    };
    drop(pwm_lease);
    assert_eq!(telemetry.firmware_fault_description(), Some("TEST_FAULT"));
    assert_eq!(
        telemetry.motor_current_unfiltered().current().as_amps(),
        12.0
    );
    assert_eq!(
        telemetry
            .directional_motor_current_unfiltered()
            .current()
            .as_amps(),
        -12.5
    );
    assert_eq!(
        telemetry.battery_current_unfiltered().current().as_amps(),
        8.0
    );
    assert_eq!(telemetry.average_power().power().as_watts(), 120.0);
    assert_eq!(telemetry.peak_power().power().as_watts(), 240.0);
    assert_eq!(
        telemetry.average_speed().speed().as_meters_per_second(),
        4.0
    );
    assert_eq!(telemetry.peak_speed().speed().as_meters_per_second(), 8.0);
    assert_eq!(telemetry.average_motor_current().current().as_amps(), 6.0);
    assert_eq!(telemetry.peak_motor_current().current().as_amps(), 18.0);
    assert_eq!(
        telemetry
            .average_mosfet_temperature()
            .temperature()
            .as_degrees_celsius(),
        45.0
    );
    assert_eq!(
        telemetry
            .peak_mosfet_temperature()
            .temperature()
            .as_degrees_celsius(),
        60.0
    );
    assert_eq!(
        telemetry
            .average_motor_temperature()
            .temperature()
            .as_degrees_celsius(),
        40.0
    );
    assert_eq!(
        telemetry
            .peak_motor_temperature()
            .temperature()
            .as_degrees_celsius(),
        55.0
    );
    assert_eq!(
        telemetry.statistics_count_time().duration().as_seconds(),
        90.0
    );
    assert_eq!(
        telemetry.signed_trip_distance().distance().as_meters(),
        -3.5
    );
    assert_eq!(telemetry.pid_position_setpoint().angle().as_degrees(), 42.0);
    assert_eq!(telemetry.pid_position().angle().as_degrees(), 12.0);
    assert_eq!(telemetry.d_axis_current().unwrap().current().as_amps(), 1.5);
    assert_eq!(telemetry.q_axis_current().unwrap().current().as_amps(), 2.5);
    assert_eq!(
        telemetry.d_axis_voltage().unwrap().voltage().as_volts(),
        3.5
    );
    assert_eq!(
        telemetry.q_axis_voltage().unwrap().voltage().as_volts(),
        4.5
    );
    assert_eq!(
        telemetry
            .tachometer(TachometerReset::Preserve)
            .steps()
            .as_steps(),
        1234
    );
    assert_eq!(
        telemetry
            .absolute_tachometer(TachometerReset::Reset)
            .steps()
            .as_steps(),
        5678
    );
    assert_eq!(
        firmware
            .telemetry()
            .absolute_tachometer(TachometerReset::Preserve)
            .steps()
            .as_steps(),
        0
    );
    assert_eq!(telemetry.sampling_frequency().as_hertz(), 20_000.0);
    firmware.motor().release_motor();
    assert_eq!(
        firmware
            .motor()
            .wait_for_motor_release(VescSeconds::from_seconds(0.1)),
        MotorReleaseOutcome::Released
    );
    firmware.motor().reset_statistics();
    firmware.motor().update_pid_position_offset(
        PidPosition::new(AngleDegrees::from_degrees(5.0)),
        PidPositionOffsetPersistence::Persistent,
    );
    assert_eq!(firmware.pid_position_offset().angle().as_degrees(), 5.0);
    assert!(firmware.pid_position_offset_was_stored());
    firmware.motor().update_pid_position_offset(
        PidPosition::new(AngleDegrees::from_degrees(6.0)),
        PidPositionOffsetPersistence::Volatile,
    );
    assert_eq!(firmware.pid_position_offset().angle().as_degrees(), 6.0);
    assert!(!firmware.pid_position_offset_was_stored());
    firmware
        .motor()
        .set_odometer(OdometerMeters::from_meters(12_345));
    assert_eq!(firmware.telemetry().odometer().as_meters(), 12_345);
    assert_eq!(
        firmware
            .motor()
            .set_tachometer(TachometerSteps::from_steps(777))
            .steps()
            .as_steps(),
        1234
    );
    assert_eq!(
        firmware
            .telemetry()
            .tachometer(TachometerReset::Preserve)
            .steps()
            .as_steps(),
        777
    );
    firmware
        .motor()
        .set_pid_speed(ElectricalSpeed::new(Rpm::from_revolutions_per_minute(
            1500.0,
        )));
    firmware
        .motor()
        .set_pid_position(PidPosition::new(AngleDegrees::from_degrees(90.0)));
    firmware.motor().select_motor(MotorSelection::new(2));
    assert_eq!(firmware.motor().selected_motor().index(), 2);
    firmware
        .motor()
        .set_duty_cycle_without_ramping(DutyCycle::new(SignedRatio::from_ratio_const(0.2)));
    let advanced = firmware.advanced_foc();
    unsafe {
        advanced
            .set_open_loop_current(
                OpenLoopCurrent::new(Current::from_amps(3.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(300.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_phase(
                OpenLoopCurrent::new(Current::from_amps(2.0)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(45.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_duty(
                DutyCycle::new(SignedRatio::from_ratio_const(0.1)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(200.0)),
            )
            .unwrap();
        advanced
            .set_open_loop_duty_phase(
                DutyCycle::new(SignedRatio::from_ratio_const(0.15)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(90.0)),
            )
            .unwrap();
    }
}

#[test]
fn motor_release_outcomes_have_named_predicates() {
    assert!(MotorReleaseOutcome::Released.is_released());
    assert!(!MotorReleaseOutcome::Released.is_timed_out());
    assert!(!MotorReleaseOutcome::TimedOut.is_released());
    assert!(MotorReleaseOutcome::TimedOut.is_timed_out());
}

#[test]
fn motor_output_preserves_typed_command_forwarding() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.motor().keep_alive();
    firmware
        .motor()
        .set_current_off_delay(CurrentOffDelay::new(VescSeconds::from_seconds(0.05)));
    firmware
        .motor()
        .set_current(vescpkg_rs::MotorCurrent::new(Current::from_amps(8.0)));
    firmware
        .motor()
        .set_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(-0.25)));
    firmware
        .motor()
        .set_brake_current(BrakeCurrent::new(Current::from_amps(3.0)));

    assert_eq!(firmware.keep_alive_count(), 1);
    assert_eq!(firmware.current_off_delay_count(), 1);
    assert_eq!(
        firmware
            .commanded_current_off_delay()
            .duration()
            .as_seconds(),
        0.05
    );
    assert_eq!(firmware.current_command_count(), 1);
    assert_eq!(firmware.commanded_current().current().as_amps(), 8.0);
    assert_eq!(firmware.duty_command_count(), 1);
    assert_eq!(firmware.commanded_duty().ratio().as_ratio(), -0.25);
    assert_eq!(firmware.brake_current_command_count(), 1);
    assert_eq!(firmware.commanded_brake_current().current().as_amps(), 3.0);
}

#[test]
fn pwm_registration_reports_absent_optional_slot() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.set_pwm_available(false);

    assert!(matches!(
        unsafe { firmware.motor().register_pwm_callback(test_pwm_callback) },
        Err(PwmCallbackError::Unavailable)
    ));
}

#[test]
fn advanced_foc_reports_absent_optional_slots() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware.set_open_loop_foc_available(false);
    let advanced = firmware.advanced_foc();

    assert_eq!(
        unsafe {
            advanced.set_open_loop_current(
                OpenLoopCurrent::new(Current::from_amps(1.0)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
            )
        },
        Err(vescpkg_rs::AdvancedFocError::Unavailable)
    );
    assert_eq!(
        unsafe {
            advanced.set_open_loop_phase(
                OpenLoopCurrent::new(Current::from_amps(1.0)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(30.0)),
            )
        },
        Err(vescpkg_rs::AdvancedFocError::Unavailable)
    );
    assert_eq!(
        unsafe {
            advanced.set_open_loop_duty(
                DutyCycle::new(SignedRatio::from_ratio_const(0.1)),
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(100.0)),
            )
        },
        Err(vescpkg_rs::AdvancedFocError::Unavailable)
    );
    assert_eq!(
        unsafe {
            advanced.set_open_loop_duty_phase(
                DutyCycle::new(SignedRatio::from_ratio_const(0.1)),
                OpenLoopPhase::new(AngleDegrees::from_degrees(30.0)),
            )
        },
        Err(vescpkg_rs::AdvancedFocError::Unavailable)
    );
}
