//! Float Out Boy runtime-thread startup helpers.
//!
//! Source oracle: Float Out Boy v1.2.1 `third_party/float-out-boy/src/main.c:2439-2449`
//! spawns the main and aux threads after loader metadata setup and before the registration tail.

#[cfg(any(test, target_arch = "arm"))]
use super::state::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use core::time::Duration;
#[cfg(target_arch = "arm")]
use vescpkg_rs::ThreadWorkingAreaSize;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{ThreadPriority, TimestampTicks};
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::{AnalogPin, DigitalPin, GpioMode};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{FirmwareThreads, Imu, MotorOutput, MotorTelemetry};

#[cfg(any(test, target_arch = "arm"))]
// C map: `LEDS_REFRESH_RATE` is `30` at `third_party/float-out-boy/src/leds.h:26`;
// `aux_thd` sleeps `1e6 / LEDS_REFRESH_RATE` at `third_party/float-out-boy/src/main.c:1155`.
const FLOAT_OUT_BOY_LEDS_REFRESH_RATE_HZ: u32 = 30;
#[cfg(any(test, target_arch = "arm"))]
const FLOAT_OUT_BOY_AUX_LOOP_TIME_US: u32 = 1_000_000 / FLOAT_OUT_BOY_LEDS_REFRESH_RATE_HZ;

#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::AdcVoltage;
#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyRuntimeThread {
    Main,
    Aux,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyRuntimeThread {
    const fn stack_bytes(self) -> usize {
        match self {
            Self::Main => 2048,
            Self::Aux => 1024,
        }
    }

    #[cfg(target_arch = "arm")]
    const fn working_area_size(self) -> ThreadWorkingAreaSize {
        match ThreadWorkingAreaSize::try_from_bytes(self.stack_bytes()) {
            Ok(size) => size,
            Err(_) => panic!("Float Out Boy thread working-area size must satisfy ChibiOS"),
        }
    }

    #[cfg(target_arch = "arm")]
    fn name(self) -> vescpkg_rs::ThreadName {
        match self {
            Self::Main => vescpkg_rs::thread_name!("Float Out Boy Main"),
            Self::Aux => vescpkg_rs::thread_name!("Float Out Boy Aux"),
        }
    }
}

/// Describe the Float Out Boy runtime threads.
///
/// Upstream passes its position-independent float_out_boy_thd and aux_thd to spawn
/// with working areas of 1536 and 1024 bytes at
/// third_party/float-out-boy/src/main.c:2438-2445. The Rust main-loop call chain is
/// larger, so it reserves 2048 bytes. VESC forwards these byte counts directly
/// to chThdCreateStatic at third_party/vesc/lispBM/lispif_c_lib.c:98-125.
#[cfg(target_arch = "arm")]
fn float_out_boy_runtime_threads() -> [vescpkg_rs::ThreadSpec<FloatOutBoyPackageState>; 2] {
    let main_thread = FloatOutBoyRuntimeThread::Main;
    let aux_thread = FloatOutBoyRuntimeThread::Aux;
    [
        vescpkg_rs::ThreadSpec::<FloatOutBoyPackageState>::new::<FloatOutBoyMainThread>(
            main_thread.working_area_size(),
            main_thread.name(),
        ),
        vescpkg_rs::ThreadSpec::<FloatOutBoyPackageState>::stateless::<FloatOutBoyAuxThread>(
            aux_thread.working_area_size(),
            aux_thread.name(),
        ),
    ]
}

/// Run Float Out Boy's source-backed main thread tick loop.
///
/// Upstream `float_out_boy_thd` calls `configure(d)` at
/// `third_party/float-out-boy/src/main.c:770`, then loops until `should_terminate()` at
/// `third_party/float-out-boy/src/main.c:772`. This narrow Rust tick ports the currently
/// source-backed caller tick, then sleeps the configured `loop_time_us` like
/// `third_party/float-out-boy/src/main.c:1080`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_float_out_boy_main_thread_with<F: FnMut() -> u32>(
    threads: &impl FirmwareThreads,
    mut tick: F,
) {
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(tick() as u64));
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyMainThreadTick {
    sleep_us: u32,
    configure_beeper: bool,
    beeper_level: Option<vescpkg_rs::DigitalOutputLevel>,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyMainThreadTick {
    const fn new(
        sleep_us: u32,
        configure_beeper: bool,
        beeper_level: Option<vescpkg_rs::DigitalOutputLevel>,
    ) -> Self {
        Self {
            sleep_us,
            configure_beeper,
            beeper_level,
        }
    }

    const fn sleep_us(self) -> u32 {
        self.sleep_us
    }

    const fn beeper_level(self) -> Option<vescpkg_rs::DigitalOutputLevel> {
        self.beeper_level
    }

    const fn configure_beeper(self) -> bool {
        self.configure_beeper
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
pub(crate) fn tick_float_out_boy_main_thread_with(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    motor: &impl MotorOutput,
    footpad_adc1: AdcVoltage,
    footpad_adc2: AdcVoltage,
    system_time_ticks: TimestampTicks,
) -> FloatOutBoyMainThreadTick {
    // C map: `float_out_boy_thd` refreshes runtime inputs, executes state/control
    // logic, applies motor control, then sleeps `loop_time_us` through
    // `third_party/float-out-boy/src/main.c:772-1080`.
    // C calls `beeper_update` before its state switch at
    // `third_party/float-out-boy/src/main.c:776-824`.
    let alert_level = state
        .tick_beeper()
        .map(crate::beeper::FloatOutBoyBeeperLevel::digital_output);
    state.refresh_main_loop_runtime_state(
        telemetry,
        imu,
        motor,
        footpad_adc1,
        footpad_adc2,
        system_time_ticks,
    );
    let run_state = state
        .all_data_payloads()
        .base()
        .status()
        .ride_state()
        .run_state();
    state.apply_motor_control(motor, run_state, system_time_ticks);
    let beeper_level = state
        .take_beeper_level()
        .map(crate::beeper::FloatOutBoyBeeperLevel::digital_output)
        .or(alert_level);

    FloatOutBoyMainThreadTick::new(
        state.configured_loop_time_us(),
        state.take_beeper_configuration_request(),
        beeper_level,
    )
}

/// Run Float Out Boy's source-backed auxiliary thread scheduler shell.
///
/// Upstream `aux_thd` optionally lowers its current thread priority at
/// `third_party/float-out-boy/src/main.c:1133-1135`, loops until `should_terminate()` at
/// `third_party/float-out-boy/src/main.c:1139`, and sleeps at `1e6 / LEDS_REFRESH_RATE` at
/// `third_party/float-out-boy/src/main.c:1155`. The refresh rate is `30` in
/// `third_party/float-out-boy/src/leds.h:26`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_float_out_boy_aux_thread_with(threads: &impl FirmwareThreads) {
    if let Ok(priority) = ThreadPriority::try_new(-1) {
        let _ = threads.set_priority(priority);
    }
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(FLOAT_OUT_BOY_AUX_LOOP_TIME_US as u64));
    }
}

/// Start Float Out Boy runtime threads from loader-owned package state.
///
/// Upstream performs this between loader metadata setup
/// (third_party/float-out-boy/src/main.c:2431-2432) and callback registration
/// (third_party/float-out-boy/src/main.c:2455-2459).
#[cfg(all(not(test), target_arch = "arm"))]
pub fn start_float_out_boy_runtime_threads(
    start: &mut vescpkg_rs::PackageStart<'_>,
) -> Result<(), vescpkg_rs::PackageStartError> {
    let firmware = vescpkg_rs::Firmware::new();
    if start
        .with_runtime_state::<FloatOutBoyPackageState, _>(|state| {
            state.initialize_balance_filter(firmware.imu().orientation());
        })
        .is_none()
    {
        return Err(vescpkg_rs::PackageStartError::StateTypeMismatch);
    }
    start.spawn_threads(float_out_boy_runtime_threads())
}

#[cfg(target_arch = "arm")]
struct FloatOutBoyMainThread;

#[cfg(target_arch = "arm")]
impl vescpkg_rs::FirmwareThread for FloatOutBoyMainThread {
    type State = FloatOutBoyPackageState;

    fn run(ctx: vescpkg_rs::ThreadContext<Self::State>) {
        // C map: Float Out Boy v1.2.1 `float_out_boy_thd` starts at
        // `third_party/float-out-boy/src/main.c:767`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let firmware = ctx.firmware();
            run_float_out_boy_main_thread_with(firmware.threads(), || {
                let system_time_ticks = firmware.clock().now();
                // C map: Float Out Boy `footpad_sensor_update` reads ADC1/ADC2 at
                // `third_party/float-out-boy/src/footpad_sensor.c:28-31`; VESC
                // defines those enum slots at `third_party/vesc/lispBM/c_libs/vesc_c_if.h:219-220`.
                let footpad_adc1 = firmware.gpio().acquire_analog(AnalogPin::ADC1).ok();
                let footpad_adc2 = firmware.gpio().acquire_analog(AnalogPin::ADC2).ok();
                let footpad_voltage1 = footpad_adc1
                    .as_ref()
                    .and_then(|pin| {
                        pin.set_mode(GpioMode::Analog)
                            .ok()
                            .and_then(|_| pin.read().ok())
                    })
                    .unwrap_or_else(|| AdcVoltage::new(vescpkg_rs::Voltage::ZERO));
                let footpad_voltage2 = footpad_adc2
                    .as_ref()
                    .and_then(|pin| {
                        pin.set_mode(GpioMode::Analog)
                            .ok()
                            .and_then(|_| pin.read().ok())
                    })
                    .unwrap_or_else(|| AdcVoltage::new(vescpkg_rs::Voltage::ZERO));
                let tick = ctx.with_state_mut(|state| {
                    state.refresh_controller_input(&vescpkg_rs::ControllerInput);
                    tick_float_out_boy_main_thread_with(
                        state,
                        firmware.telemetry(),
                        firmware.imu(),
                        firmware.motor(),
                        footpad_voltage1,
                        footpad_voltage2,
                        system_time_ticks,
                    )
                });
                tick.map_or(1, |tick| {
                    if tick.configure_beeper() {
                        let _ = firmware
                            .gpio()
                            .acquire_digital(DigitalPin::PPM)
                            .and_then(|pin| {
                                pin.set_mode(GpioMode::Output)?;
                                pin.write(vescpkg_rs::DigitalOutputLevel::High)
                            });
                    }
                    if let Some(level) = tick.beeper_level() {
                        if let Ok(pin) = firmware.gpio().acquire_digital(DigitalPin::PPM) {
                            let _ = pin.set_mode(GpioMode::Output);
                            let _ = pin.write(level);
                        }
                    }
                    tick.sleep_us()
                })
            });
        }

        #[cfg(test)]
        {
            let _ = ctx;
        }
    }
}

#[cfg(target_arch = "arm")]
struct FloatOutBoyAuxThread;

#[cfg(target_arch = "arm")]
impl vescpkg_rs::StatelessFirmwareThread for FloatOutBoyAuxThread {
    fn run(ctx: vescpkg_rs::StatelessThreadContext) {
        // C map: Float Out Boy v1.2.1 `aux_thd` starts at
        // `third_party/float-out-boy/src/main.c:1130`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            run_float_out_boy_aux_thread_with(ctx.threads());
        }

        #[cfg(test)]
        {
            let _ = ctx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::FloatOutBoyPackageState;
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

    #[test]
    fn float_out_boy_main_thread_reserves_the_generated_rust_working_area() {
        // The current ARM call chain reaches 1480 bytes before ChibiOS's
        // thread metadata, saved contexts, and interrupt reserve.
        assert_eq!(super::FloatOutBoyRuntimeThread::Main.stack_bytes(), 2048);
        assert_eq!(super::FloatOutBoyRuntimeThread::Aux.stack_bytes(), 1024);
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
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());

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
            [Duration::from_micros(1201), Duration::ZERO]
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
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
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
        assert_eq!(telemetry.commanded_current().current().as_amps(), 3.5);
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

        super::tick_float_out_boy_main_thread_with(
            &mut state,
            firmware.telemetry(),
            firmware.imu(),
            firmware.motor(),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            AdcVoltage::new(Voltage::from_volts(2.5)),
            TimestampTicks::from_ticks(0),
        );

        assert_eq!(firmware.foc_tone_command_count(), 1);
        assert_eq!(
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
        let motor = telemetry.motor();
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
        let mut config = default_float_out_boy_config_bytes();
        config[242] = 1;
        assert!(state.store_serialized_config(&config));
        state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));
        state.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
        let mut changes = std::vec::Vec::new();
        let mut configure_ticks = std::vec::Vec::new();

        for tick in 1..=160 {
            let result = super::tick_float_out_boy_main_thread_with(
                &mut state,
                telemetry.telemetry(),
                imu,
                motor,
                AdcVoltage::new(Voltage::ZERO),
                AdcVoltage::new(Voltage::ZERO),
                TimestampTicks::from_ticks(0),
            );
            if let Some(level) = result.beeper_level() {
                changes.push((tick, level));
            }
            if result.configure_beeper() {
                configure_ticks.push(tick);
            }
        }

        assert_eq!(configure_ticks, [1]);
        assert_eq!(
            changes,
            [
                (80, DigitalOutputLevel::Low),
                (160, DigitalOutputLevel::High),
            ]
        );
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
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
        ));
        let mut config = default_float_out_boy_config_bytes();
        config[242] = 1;
        assert!(state.store_serialized_config(&config));
        for _ in 0..=240 {
            let _ = state.tick_beeper();
        }

        let warning = super::tick_float_out_boy_main_thread_with(
            &mut state,
            firmware.telemetry(),
            firmware.imu(),
            firmware.motor(),
            AdcVoltage::new(Voltage::ZERO),
            AdcVoltage::new(Voltage::ZERO),
            TimestampTicks::from_ticks(1),
        );
        assert_eq!(warning.beeper_level(), Some(DigitalOutputLevel::High));
        assert_eq!(
            state.all_data_payloads().base().status().beep_reason(),
            FloatOutBoyBeepReason::Sensors
        );

        let restored = super::tick_float_out_boy_main_thread_with(
            &mut state,
            firmware.telemetry(),
            firmware.imu(),
            firmware.motor(),
            AdcVoltage::new(Voltage::from_volts(3.0)),
            AdcVoltage::new(Voltage::from_volts(3.0)),
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
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
        ));
        let mut config = default_float_out_boy_config_bytes();
        config[50] = 1;
        config[242] = 1;
        assert!(state.store_serialized_config(&config));
        for _ in 0..=240 {
            let _ = state.tick_beeper();
        }

        let warning_tick = (1..=400).find(|tick| {
            super::tick_float_out_boy_main_thread_with(
                &mut state,
                firmware.telemetry(),
                firmware.imu(),
                firmware.motor(),
                AdcVoltage::new(Voltage::from_volts(3.0)),
                AdcVoltage::new(Voltage::from_volts(3.0)),
                TimestampTicks::from_ticks(*tick),
            )
            .beeper_level()
                == Some(DigitalOutputLevel::High)
        });

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
        let release_tick = (401..=800).find(|tick| {
            super::tick_float_out_boy_main_thread_with(
                &mut state,
                firmware.telemetry(),
                firmware.imu(),
                firmware.motor(),
                AdcVoltage::new(Voltage::from_volts(3.0)),
                AdcVoltage::new(Voltage::from_volts(3.0)),
                TimestampTicks::from_ticks(*tick),
            )
            .beeper_level()
                == Some(DigitalOutputLevel::Low)
        });
        assert!(release_tick.is_some());
    }

    #[test]
    fn float_out_boy_main_thread_sleeps_with_configured_loop_time_like_float_out_boy_loop() {
        let firmware = FirmwareTest::new();
        firmware.terminate_threads_after_checks(2);
        let threads = firmware.threads();
        let mut tick_calls = 0;

        super::run_float_out_boy_main_thread_with(threads, || {
            tick_calls += 1;
            // Upstream `configure(d)` stores `d->loop_time_us` from
            // `d->float_conf.hertz` at `third_party/float-out-boy/src/main.c:190-191`, then
            // `float_out_boy_thd` sleeps that configured value at `third_party/float-out-boy/src/main.c:1080`.
            2000
        });

        assert_eq!(tick_calls, 1);
        assert_eq!(firmware.thread_termination_check_count(), 2);
        assert_eq!(firmware.thread_sleep_count(), 1);
        assert_eq!(
            firmware.thread_sleep_durations(),
            [Duration::from_micros(2000), Duration::ZERO]
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
}
