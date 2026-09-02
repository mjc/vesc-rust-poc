use super::super::state::FloatOutBoyPackageState;
use super::tick_float_out_boy_aux_thread_with;
use crate::beeper::{FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount};
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyBeepReason, FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyRideState,
    FloatOutBoyRunState, FloatOutBoySetpointAdjustment, FloatOutBoyStopCondition,
};
use crate::package::test_support::{
    default_float_out_boy_config_bytes, sample_all_data_payloads_with_ride_state,
};
use core::time::Duration;
use vescpkg_rs::prelude::*;
use vescpkg_rs::test_support::FirmwareTest;

fn state_in(run_state: FloatOutBoyRunState) -> FloatOutBoyPackageState {
    FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        run_state,
        FloatOutBoyMode::Normal,
    ))
}

fn tick_with_footpad_voltage(
    state: &mut FloatOutBoyPackageState,
    firmware: &FirmwareTest,
    voltage: Voltage,
    now: TimestampTicks,
) -> super::FloatOutBoyMainThreadTick {
    let voltage = AdcVoltage::new(voltage);
    super::tick_float_out_boy_main_thread_with(
        state,
        firmware.telemetry(),
        firmware.imu(),
        firmware.motor(),
        voltage,
        voltage,
        now,
    )
}

fn first_beeper_level_tick(
    state: &mut FloatOutBoyPackageState,
    firmware: &FirmwareTest,
    mut ticks: core::ops::RangeInclusive<u32>,
    voltage: Voltage,
    expected: DigitalOutputLevel,
) -> Option<u32> {
    ticks.find(|tick| {
        tick_with_footpad_voltage(state, firmware, voltage, TimestampTicks::from_ticks(*tick))
            .beeper_level()
            == Some(expected)
    })
}

#[test]
fn float_out_boy_runtime_threads_reserve_their_measured_rust_working_areas() {
    // The persisted-config call chain measured 1976 bytes before ChibiOS's
    // thread metadata, saved contexts, and interrupt reserve. The aux LED
    // reconfiguration chain measures 1948 bytes.
    for thread in [
        super::FloatOutBoyRuntimeThread::Main,
        super::FloatOutBoyRuntimeThread::Aux,
    ] {
        assert_eq!(
            thread
                .working_area_size()
                .expect("valid working area")
                .usable_stack_bytes(),
            2_656,
        );
    }
}

#[test]
fn float_out_boy_main_thread_tick_refreshes_runtime_state_and_sleeps_like_float_out_boy_loop() {
    let telemetry = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
        VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
        TotalMotorCurrent::new(Current::from_amps(12.25)),
        InputCurrent::new(Current::from_amps(6.5)),
        DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
    );
    telemetry.set_imu_ready(true);
    telemetry.terminate_threads_after_checks(2);
    let threads = telemetry.threads();
    telemetry.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.9)),
        ImuPitch::new(AngleRadians::from_radians(14.0)),
        ImuYaw::new(AngleRadians::from_radians(0.0)),
    );
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::default();

    super::run_float_out_boy_main_thread_with(threads, || {
        state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));
        state.configured_loop_time_us()
    });

    let payloads = state.all_data_payloads();
    assert_eq!(
        payloads.base().status().ride_state().run_state(),
        FloatOutBoyRunState::Ready,
    );
    assert_eq!(
        payloads.base().motor().electrical_speed(),
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0))
    );
    assert_eq!(
        payloads.base().attitude().roll(),
        ImuRoll::new(AngleRadians::from_radians(0.9)),
    );
    assert_eq!(
        payloads.base().attitude().pitch(),
        ImuPitch::new(AngleRadians::from_radians(14.0)),
    );
    assert_eq!(telemetry.thread_termination_check_count(), 2);
    assert_eq!(telemetry.thread_sleep_count(), 1);
    assert_eq!(
        telemetry.thread_sleep_durations(),
        [Duration::from_millis(2), Duration::ZERO]
    );
}

#[test]
fn float_out_boy_main_thread_tick_applies_motor_control_like_float_out_boy_loop() {
    let telemetry = FirmwareTest::new();
    telemetry.terminate_threads_after_checks(2);
    let threads = telemetry.threads();
    telemetry.set_imu_ready(true);
    let imu = telemetry.imu();
    let bindings = telemetry.motor();
    let mut state = FloatOutBoyPackageState::default();
    state.request_motor_current(MotorCurrent::new(Current::from_amps(3.5)));

    super::run_float_out_boy_main_thread_with(threads, || {
        super::tick_float_out_boy_main_thread_with(
            &mut state,
            telemetry.telemetry(),
            imu,
            bindings,
            AdcVoltage::new(Voltage::from_volts(2.5)),
            AdcVoltage::new(Voltage::from_volts(0.0)),
            TimestampTicks::from_ticks(0),
        )
        .sleep_us()
    });

    // Upstream `float_out_boy_thd` applies motor control after the state switch at
    // `third_party/float-out-boy/src/main.c:1075`, before sleeping at
    // `third_party/float-out-boy/src/main.c:1080`.
    assert_eq!(telemetry.current_command_count(), 1);
    assert_f32_eq!(telemetry.commanded_current().current().as_amps(), 3.5);
    assert_eq!(
        state.all_data_payloads().base().footpad().state(),
        FloatOutBoyFootpadState::Left,
    );
}

#[test]
fn float_out_boy_main_thread_tick_drives_duty_haptic_through_typed_motor_audio() {
    let firmware = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1200.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.81)),
    );
    firmware.set_imu_ready(true);
    let payloads = sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Running,
        FloatOutBoyMode::Normal,
    );
    let base = payloads.base();
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(
            FloatOutBoyRideState::new(
                FloatOutBoyRunState::Running,
                FloatOutBoyMode::Normal,
                FloatOutBoySetpointAdjustment::PushbackDuty,
                FloatOutBoyStopCondition::None,
            ),
            base.status().beep_reason(),
        ),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
        base,
        payloads.mode2(),
        payloads.mode3(),
        payloads.mode4(),
    ));

    tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::from_volts(2.5),
        TimestampTicks::from_ticks(0),
    );

    assert_eq!(firmware.foc_tone_command_count(), 1);
    assert_f32_eq!(
        firmware
            .commanded_foc_tone_frequency()
            .frequency()
            .as_hertz(),
        495.0
    );
}

#[test]
fn float_out_boy_main_thread_drives_typed_ppm_beeper_levels_like_float_out_boy_loop() {
    let telemetry = FirmwareTest::new();
    let imu = telemetry.imu();
    let mut state = FloatOutBoyPackageState::default();
    let mut config = default_float_out_boy_config_bytes();
    config[248] = 1;
    assert!(state.store_serialized_config(&config));
    state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
    let mut changes = std::vec::Vec::new();

    for tick in 1..=42 {
        let result = tick_with_footpad_voltage(
            &mut state,
            &telemetry,
            Voltage::ZERO,
            TimestampTicks::from_ticks(tick * 100),
        );
        if let Some(level) = result.beeper_level() {
            changes.push((tick, level));
        }
    }

    assert_eq!(
        changes,
        [
            (1, DigitalOutputLevel::Low),
            (6, DigitalOutputLevel::Low),
            (12, DigitalOutputLevel::High),
            (18, DigitalOutputLevel::Low),
            (24, DigitalOutputLevel::High),
            (30, DigitalOutputLevel::Low),
            (36, DigitalOutputLevel::High),
            (42, DigitalOutputLevel::Low),
        ]
    );
}

#[test]
fn main_thread_consumes_beeper_pin_setup_when_a_level_wins_the_same_tick() {
    let telemetry = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::default();
    let mut config = default_float_out_boy_config_bytes();
    config[248] = 1;
    assert!(state.store_serialized_config(&config));
    state.force_beeper_on();

    tick_with_footpad_voltage(
        &mut state,
        &telemetry,
        Voltage::ZERO,
        TimestampTicks::from_ticks(0),
    );

    assert!(!state.take_beeper_configuration_request());
}

#[test]
fn beeper_pin_setup_preserves_disabled_ppm_input_like_refloat_startup() {
    let _firmware = FirmwareTest::new();
    for (remote_type, expected) in [(0, true), (1, true), (2, false)] {
        let mut state = FloatOutBoyPackageState::default();
        let mut config = default_float_out_boy_config_bytes();
        config[99] = remote_type;
        config[248] = 0;
        assert!(state.store_serialized_config(&config));

        assert_eq!(
            state.take_beeper_configuration_request(),
            expected,
            "remote_type={remote_type}"
        );
        assert!(!state.take_beeper_configuration_request());
    }
}

#[test]
fn enabling_the_beeper_after_startup_acquires_ppm_instead_of_reproducing_refloats_bug() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::default();
    let mut config = default_float_out_boy_config_bytes();
    config[99] = 2;
    config[248] = 0;
    assert!(state.store_serialized_config(&config));
    assert!(!state.take_beeper_configuration_request());

    config[248] = 1;
    assert!(state.store_serialized_config(&config));

    assert!(state.take_beeper_configuration_request());
    assert!(!state.take_beeper_configuration_request());
}

#[test]
fn float_out_boy_main_thread_forces_footpad_warning_on_and_off_like_float_out_boy() {
    let firmware = FirmwareTest::new().with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(3_000.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    firmware.set_imu_ready(true);
    let mut state = state_in(FloatOutBoyRunState::Running);
    let mut config = default_float_out_boy_config_bytes();
    config[248] = 1;
    assert!(state.store_serialized_config(&config));
    for _ in 0..=240 {
        let _ = state.tick_beeper();
    }

    let warning = tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::ZERO,
        TimestampTicks::from_ticks(1),
    );
    assert_eq!(warning.beeper_level(), Some(DigitalOutputLevel::High));
    assert_eq!(
        state.all_data_payloads().base().status().beep_reason(),
        FloatOutBoyBeepReason::Sensors
    );

    let restored = tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::from_volts(3.0),
        TimestampTicks::from_ticks(2),
    );
    assert_eq!(restored.beeper_level(), Some(DigitalOutputLevel::Low));
}

#[test]
fn float_out_boy_main_thread_holds_duty_warning_for_duty_pushback_like_float_out_boy() {
    let mut firmware = FirmwareTest::new()
        .with_runtime_motor(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1_200.0)),
            VehicleSpeed::new(Speed::ZERO),
            TotalMotorCurrent::new(Current::ZERO),
            InputCurrent::new(Current::ZERO),
            DutyCycle::new(SignedRatio::from_ratio_const(0.9)),
        )
        .with_input_voltage(InputVoltage::new(Voltage::from_volts(72.0)))
        .with_battery_cell_count(BatteryCellCount::try_new(18).expect("18s battery"));
    firmware.set_imu_ready(true);
    let mut state = state_in(FloatOutBoyRunState::Running);
    let mut config = default_float_out_boy_config_bytes();
    config[70] = 1;
    config[248] = 1;
    assert!(state.store_serialized_config(&config));
    for _ in 0..=240 {
        let _ = state.tick_beeper();
    }

    let warning_tick = first_beeper_level_tick(
        &mut state,
        &firmware,
        1..=400,
        Voltage::from_volts(3.0),
        DigitalOutputLevel::High,
    );

    let status = state.all_data_payloads().base().status();
    assert_eq!(
        status.ride_state().run_state(),
        FloatOutBoyRunState::Running
    );
    let duty = state
        .all_data_payloads()
        .base()
        .motor()
        .duty_cycle()
        .ratio()
        .as_ratio();
    assert!(duty > 0.8, "duty={duty}, warning_tick={warning_tick:?}");
    assert_eq!(
        status.ride_state().setpoint_adjustment(),
        FloatOutBoySetpointAdjustment::PushbackDuty
    );
    assert_eq!(status.beep_reason(), FloatOutBoyBeepReason::Duty);
    assert!(warning_tick.is_some());

    firmware = firmware.with_runtime_motor(
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1_200.0)),
        VehicleSpeed::new(Speed::ZERO),
        TotalMotorCurrent::new(Current::ZERO),
        InputCurrent::new(Current::ZERO),
        DutyCycle::new(SignedRatio::from_ratio_const(0.0)),
    );
    let release_tick = first_beeper_level_tick(
        &mut state,
        &firmware,
        401..=800,
        Voltage::from_volts(3.0),
        DigitalOutputLevel::Low,
    );
    assert!(release_tick.is_some());
}

#[test]
fn float_out_boy_main_thread_sleeps_with_fixed_loop_time_like_refloat() {
    let firmware = FirmwareTest::new();
    firmware.terminate_threads_after_checks(2);
    let threads = firmware.threads();
    let mut tick_calls = 0;

    super::run_float_out_boy_main_thread_with(threads, || {
        tick_calls += 1;
        // Refloat 7c72c6d3 hardcodes the main thread to 500 Hz.
        2000
    });

    assert_eq!(tick_calls, 1);
    assert_eq!(firmware.thread_termination_check_count(), 2);
    assert_eq!(firmware.thread_sleep_count(), 1);
    assert_eq!(
        firmware.thread_sleep_durations(),
        [Duration::from_millis(2), Duration::ZERO]
    );
}

#[test]
fn refloat_main_loop_timing_uses_system_ticks_and_compensates_work() {
    let timing = super::FloatOutBoyMainLoopTiming::from_sample_rate(SampleRate::from_hertz(832.0));

    assert_eq!(timing.nominal_sleep(), Duration::from_micros(1_200));
    assert_eq!(
        timing.sleep_after_work(VescSeconds::from_seconds(0.000_31)),
        Duration::from_micros(900),
    );
}

#[test]
fn refloat_main_loop_timing_keeps_one_tick_after_a_delayed_iteration() {
    let timing = super::FloatOutBoyMainLoopTiming::from_sample_rate(SampleRate::from_hertz(500.0));

    assert_eq!(
        timing.sleep_after_work(VescSeconds::from_seconds(0.025)),
        Duration::from_micros(100),
    );
    assert_eq!(
        timing.sleep_after_work(VescSeconds::from_seconds(f32::NAN)),
        timing.nominal_sleep(),
    );
}

#[test]
fn float_out_boy_aux_thread_lowers_priority_and_sleeps_like_float_out_boy_aux_loop() {
    let firmware = FirmwareTest::new();
    firmware.terminate_threads_after_checks(2);
    let threads = firmware.threads();

    super::run_float_out_boy_aux_thread_with(threads);

    assert_eq!(firmware.thread_priority_change_count(), 1);
    assert_eq!(
        firmware.thread_priorities()[0],
        ThreadPriority::try_new(-1).ok()
    );
    assert_eq!(firmware.thread_termination_check_count(), 2);
    assert_eq!(firmware.thread_sleep_count(), 1);
    assert_eq!(
        firmware.thread_sleep_durations(),
        [Duration::from_micros(33_333), Duration::ZERO]
    );
}

#[test]
fn startup_seeds_aux_backup_threshold_from_required_firmware_odometer() {
    let firmware = FirmwareTest::new();
    let mut state = state_in(FloatOutBoyRunState::Ready);

    super::initialize_float_out_boy_runtime_state(
        &mut state,
        firmware.telemetry(),
        firmware.imu().orientation(),
        OdometerMeters::from_meters(42_000),
    );

    assert!(!state.aux_backup_due(OdometerMeters::from_meters(42_200)));
    assert!(state.aux_backup_due(OdometerMeters::from_meters(42_201)));
}

#[test]
fn float_out_boy_aux_backup_threshold_matches_source_and_run_state() {
    for (run_state, distance, expected) in [
        (FloatOutBoyRunState::Ready, 1_200, false),
        (FloatOutBoyRunState::Ready, 1_201, true),
        (FloatOutBoyRunState::Running, 1_201, false),
    ] {
        let mut state = state_in(run_state);
        state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
        assert_eq!(
            state.aux_backup_due(OdometerMeters::from_meters(distance)),
            expected,
            "run_state={run_state:?}, distance={distance}"
        );
    }
}

#[test]
fn aux_backup_claim_rechecks_running_state() {
    let mut state = state_in(FloatOutBoyRunState::Running);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let mut stores = 0;

    let result = super::store_aux_backup_if_due(
        &mut state,
        OdometerMeters::from_meters(1_201),
        TimestampTicks::from_ticks(0),
        || {
            stores += 1;
            true
        },
    );

    assert_eq!(result, None);
    assert_eq!(stores, 0);
}

#[test]
fn aux_backup_claim_blocks_a_second_claim_until_the_effect_finishes() {
    let mut state = state_in(FloatOutBoyRunState::Ready);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let odometer = OdometerMeters::from_meters(1_201);

    assert!(state.begin_aux_backup(odometer));
    assert!(!state.begin_aux_backup(odometer));
    assert!(!state.aux_backup_due(odometer));

    let completed_at = TimestampTicks::from_ticks(10);
    state.finish_aux_backup(odometer, true, completed_at);

    assert!(!state.aux_backup_due(odometer));
    assert!(!state.aux_backup_allows_engagement(completed_at));
    assert!(
        state.aux_backup_allows_engagement(TimestampTicks::from_ticks(
            completed_at
                .as_ticks()
                .saturating_add(
                    u32::try_from(5 * SYSTEM_TICK_RATE_HZ).expect("five seconds fits in ticks"),
                )
                .saturating_add(1),
        ))
    );
}

#[test]
fn failed_aux_backup_is_diagnosable_and_advances_threshold_like_refloat() {
    let mut state = state_in(FloatOutBoyRunState::Ready);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let odometer = OdometerMeters::from_meters(1_201);
    assert!(state.begin_aux_backup(odometer));
    state.finish_aux_backup(odometer, false, TimestampTicks::from_ticks(0));

    assert_eq!(state.aux_backup_failures(), 1);
    assert!(!state.aux_backup_due(odometer));
}

#[test]
fn aux_backup_write_and_cooldown_block_ready_engagement() {
    let firmware = FirmwareTest::new();
    firmware.set_imu_ready(true);
    let mut state = state_in(FloatOutBoyRunState::Ready);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let odometer = OdometerMeters::from_meters(1_201);
    assert!(state.begin_aux_backup(odometer));

    tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::from_volts(3.0),
        TimestampTicks::from_ticks(1),
    );
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready
    );

    let completed_at = TimestampTicks::from_ticks(2);
    state.finish_aux_backup(odometer, true, completed_at);
    tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::from_volts(3.0),
        TimestampTicks::from_ticks(3),
    );
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Ready
    );

    tick_with_footpad_voltage(
        &mut state,
        &firmware,
        Voltage::from_volts(3.0),
        TimestampTicks::from_ticks(
            completed_at
                .as_ticks()
                .saturating_add(
                    u32::try_from(5 * SYSTEM_TICK_RATE_HZ).expect("five seconds fits in ticks"),
                )
                .saturating_add(1),
        ),
    );
    assert_eq!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        FloatOutBoyRunState::Running
    );
}

#[test]
fn aux_tick_stores_after_strict_distance_threshold() {
    let firmware = FirmwareTest::new().with_motor_current_limits(
        MotorCurrentLimit::new(Current::from_amps(42.0)),
        MotorCurrentLimit::new(Current::from_amps(17.0)),
    );
    let mut state = state_in(FloatOutBoyRunState::Ready);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let mut stores = 0;

    let result = tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        OdometerMeters::from_meters(1_201),
        TimestampTicks::from_ticks(0),
        0.0,
        |_| {},
        || {
            stores += 1;
            true
        },
    );

    assert_eq!(result, Some(true));
    assert_eq!(stores, 1);
}

#[test]
fn aux_tick_renders_and_paints_one_internal_led_frame() {
    let firmware = FirmwareTest::new();
    let mut state = state_in(FloatOutBoyRunState::Ready);
    let bar = crate::leds::FloatOutBoyLedBarConfig::new(
        Ratio::from_ratio_const(1.0),
        crate::leds::FloatOutBoyLedColor::Blue,
        crate::leds::FloatOutBoyLedColor::Black,
        crate::leds::FloatOutBoyLedAnimationMode::Solid,
        crate::leds::FloatOutBoyLedAnimationSpeed::from_units(1.0),
    );
    let status = crate::leds::FloatOutBoyStatusBarConfig::new(
        crate::leds::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
        Ratio::from_ratio_const(0.9),
        Ratio::from_ratio_const(0.1),
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(1.0),
    );
    let config = crate::leds::FloatOutBoyLedsConfig::new(bar, bar, bar, bar, status, bar).enabled();
    let strip = crate::leds::FloatOutBoyLedStripConfig::new(
        crate::leds::FloatOutBoyLedStripOrder::First,
        1,
        crate::leds::FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_front_strip(strip);
    state.configure_internal_leds(hardware, config);
    let mut paints = 0;
    let mut painted = [0; 4];

    let result = tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        OdometerMeters::from_meters(0),
        TimestampTicks::from_ticks(0),
        1.0 / 30.0,
        |renderer| {
            paints += 1;
            painted = renderer
                .front()
                .physical_pixel(0)
                .map(crate::leds::FloatOutBoyLedPixel::channels)
                .unwrap_or_default();
        },
        || true,
    );

    assert_eq!(result, None);
    assert_eq!(paints, 1);
    assert_eq!(painted, [0, 0, 0x1a, 0]);
}

#[test]
fn aux_tick_does_not_touch_backup_before_threshold() {
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::default();
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));
    let mut stores = 0;

    let result = tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        OdometerMeters::from_meters(1_200),
        TimestampTicks::from_ticks(0),
        0.0,
        |_| {},
        || {
            stores += 1;
            true
        },
    );

    assert_eq!(result, None);
    assert_eq!(stores, 0);
}

#[test]
fn aux_tick_records_a_rejected_backup_without_retrying_the_same_distance() {
    let firmware = FirmwareTest::new();
    let mut state = state_in(FloatOutBoyRunState::Ready);
    state.initialize_aux_odometer(OdometerMeters::from_meters(1_000));

    let result = tick_float_out_boy_aux_thread_with(
        &mut state,
        firmware.telemetry(),
        OdometerMeters::from_meters(1_201),
        TimestampTicks::from_ticks(0),
        0.0,
        |_| {},
        || false,
    );

    assert_eq!(result, Some(false));
    assert_eq!(state.aux_backup_failures(), 1);
    assert!(!state.aux_backup_due(OdometerMeters::from_meters(1_201)));
}
