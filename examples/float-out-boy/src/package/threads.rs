//! Float Out Boy runtime-thread startup helpers.
//!
//! Source oracle: Float Out Boy v1.2.1 `third_party/float-out-boy/src/main.c:2439-2449`
//! spawns the main and aux threads after loader metadata setup and before the registration tail.

use super::state::FloatOutBoyPackageState;
use core::time::Duration;
use vescpkg_rs::ThreadWorkingAreaSize;
use vescpkg_rs::prelude::{
    OdometerMeters, SYSTEM_TICK_RATE_HZ, SampleRate, ThreadPriority, TimestampTicks, VescSeconds,
};
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::{AnalogPin, DigitalPin, GpioMode};
use vescpkg_rs::{FirmwareThreads, Imu, MotorOutput, MotorTelemetry};

#[cfg(test)]
use test_support::{
    run_float_out_boy_aux_thread_with, run_float_out_boy_main_thread_with,
    tick_float_out_boy_main_thread_with,
};

// C map: `LEDS_REFRESH_RATE` is `30` at `third_party/float-out-boy/src/leds.h:26`;
// `aux_thd` sleeps `1e6 / LEDS_REFRESH_RATE` at `third_party/float-out-boy/src/main.c:1155`.
const FLOAT_OUT_BOY_LEDS_REFRESH_RATE_HZ: u32 = 30;
const FLOAT_OUT_BOY_AUX_LOOP_TIME_US: u32 = 1_000_000 / FLOAT_OUT_BOY_LEDS_REFRESH_RATE_HZ;

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyMainLoopTiming {
    nominal_ticks: u32,
}

#[cfg(any(test, target_arch = "arm"))]
impl FloatOutBoyMainLoopTiming {
    fn from_sample_rate(sample_rate: SampleRate) -> Self {
        let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).map_or(f32::NAN, f32::from);
        let nominal_ticks =
            crate::wire::saturating_trunc_f32_to_u32(tick_rate / sample_rate.as_hertz()).max(1);
        Self { nominal_ticks }
    }

    fn nominal_sleep(self) -> Duration {
        Self::ticks_to_duration(self.nominal_ticks)
    }

    fn sleep_after_work(self, elapsed: VescSeconds) -> Duration {
        let elapsed = elapsed.as_seconds();
        let tick_rate = u16::try_from(SYSTEM_TICK_RATE_HZ).map_or(f32::NAN, f32::from);
        if !elapsed.is_finite() || elapsed < 0.0 {
            return self.nominal_sleep();
        }
        // C map: Refloat rounds work time to system ticks with `lrintf`, then
        // retains at least one sleep tick at `src/main.c` in `fa5d9f73`.
        let work_ticks = crate::wire::saturating_trunc_f32_to_u32(elapsed * tick_rate + 0.5);
        Self::ticks_to_duration(self.nominal_ticks.saturating_sub(work_ticks).max(1))
    }

    fn ticks_to_duration(ticks: u32) -> Duration {
        Duration::from_micros(u64::from(ticks).saturating_mul(1_000_000) / SYSTEM_TICK_RATE_HZ)
    }
}

use vescpkg_rs::prelude::AdcVoltage;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyRuntimeThread {
    Main,
    Aux,
}

impl FloatOutBoyRuntimeThread {
    const fn stack_bytes(self) -> usize {
        match self {
            Self::Main | Self::Aux => 3072,
        }
    }

    const fn working_area_size(
        self,
    ) -> Result<ThreadWorkingAreaSize, vescpkg_rs::ThreadWorkingAreaSizeError> {
        ThreadWorkingAreaSize::try_from_bytes(self.stack_bytes())
    }

    #[cfg(target_arch = "arm")]
    fn name(self) -> vescpkg_rs::ThreadName {
        match self {
            Self::Main => vescpkg_rs::thread_name!("FOB main"),
            Self::Aux => vescpkg_rs::thread_name!("FOB aux"),
        }
    }
}

/// Describe the Float Out Boy runtime threads.
///
/// Upstream passes its position-independent `float_out_boy_thd` and `aux_thd` to spawn
/// with working areas of 1536 and 1024 bytes at
/// third_party/float-out-boy/src/main.c:2438-2445. The Rust main loop reserves
/// 3072 bytes because its entry also performs the persisted-config read moved off
/// VESC's undersized evaluator stack. The auxiliary thread reserves 3072 bytes
/// because it performs Refloat's post-spawn LED hardware setup without
/// consuming the loader's fixed 2048-byte evaluator stack. Its linked LED
/// reconfiguration chain measures 1948 bytes, so it also requests 3072 bytes
/// and retains 2656 usable bytes after VESC/ChibiOS working-area overhead. VESC
/// forwards these byte counts directly to chThdCreateStatic at
/// third_party/vesc/lispBM/lispif_c_lib.c:98-125.
#[cfg(target_arch = "arm")]
fn float_out_boy_runtime_threads() -> Result<
    [vescpkg_rs::ThreadSpec<FloatOutBoyPackageState>; 2],
    vescpkg_rs::ThreadWorkingAreaSizeError,
> {
    let main_thread = FloatOutBoyRuntimeThread::Main;
    let aux_thread = FloatOutBoyRuntimeThread::Aux;
    Ok([
        vescpkg_rs::ThreadSpec::<FloatOutBoyPackageState>::new::<FloatOutBoyMainThread>(
            main_thread.working_area_size()?,
            main_thread.name(),
        ),
        vescpkg_rs::ThreadSpec::<FloatOutBoyPackageState>::new::<FloatOutBoyAuxThread>(
            aux_thread.working_area_size()?,
            aux_thread.name(),
        ),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloatOutBoyMainThreadTick {
    sleep_us: u32,
    beeper_pin_level: Option<vescpkg_rs::DigitalOutputLevel>,
}

impl FloatOutBoyMainThreadTick {
    const fn new(sleep_us: u32, beeper_pin_level: Option<vescpkg_rs::DigitalOutputLevel>) -> Self {
        Self {
            sleep_us,
            beeper_pin_level,
        }
    }

    #[cfg(test)]
    const fn sleep_us(self) -> u32 {
        self.sleep_us
    }
}

#[derive(Clone, Copy)]
struct FloatOutBoyMainThreadPrepare {
    alert_level: Option<crate::beeper::FloatOutBoyBeeperLevel>,
    restore_flywheel_config: bool,
}

fn prepare_float_out_boy_main_thread_tick(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    motor: &impl MotorOutput,
    footpads: (AdcVoltage, AdcVoltage),
    system_time_ticks: TimestampTicks,
    elapsed: VescSeconds,
) -> FloatOutBoyMainThreadPrepare {
    // C map: `float_out_boy_thd` refreshes runtime inputs, executes state/control
    // logic, applies motor control, then sleeps `loop_time_us` through
    // `third_party/float-out-boy/src/main.c:772-1080`.
    // C calls `beeper_update` before its state switch at
    // `third_party/float-out-boy/src/main.c:776-824`.
    let alert_level = state.tick_beeper_at(system_time_ticks);
    let restore_flywheel_config = state.refresh_main_loop_runtime_state_elapsed(
        telemetry,
        imu,
        motor,
        footpads,
        system_time_ticks,
        elapsed,
    );
    FloatOutBoyMainThreadPrepare {
        alert_level,
        restore_flywheel_config,
    }
}

fn finish_float_out_boy_main_thread_tick(
    state: &mut FloatOutBoyPackageState,
    motor: &impl MotorOutput,
    system_time_ticks: TimestampTicks,
    prepared: FloatOutBoyMainThreadPrepare,
) -> FloatOutBoyMainThreadTick {
    #[cfg(test)]
    {
        let run_state = state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state();
        // Host main-loop fixtures preserve their existing deterministic control
        // step. The ARM artifact applies motor output and records samples from
        // the IMU callback, matching Refloat main.
        state.apply_motor_control(motor, run_state, system_time_ticks);
        state.sample_data_recorder(system_time_ticks);
    }
    #[cfg(not(test))]
    let _ = (motor, system_time_ticks);
    let beeper_level = state.take_beeper_level().or(prepared.alert_level);

    let configure_beeper = state.take_beeper_configuration_request();
    let beeper_pin_level = match beeper_level {
        None if configure_beeper => Some(crate::beeper::FloatOutBoyBeeperLevel::Low),
        level => level,
    };

    FloatOutBoyMainThreadTick::new(state.configured_loop_time_us(), beeper_pin_level)
}

/// Refresh the source-backed auxiliary state and persist a backup when its threshold is due.
///
/// `aux_thd` renders LEDs, conditionally stores the backup, then refreshes motor
/// configuration after a strict half-second interval at
/// `third_party/float-out-boy/src/main.c:1131-1155`.
#[cfg(test)]
pub(crate) fn tick_float_out_boy_aux_thread_with(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    odometer: OdometerMeters,
    system_time_ticks: TimestampTicks,
    current_time: f32,
    paint_leds: impl FnOnce(&crate::leds::FloatOutBoyLedRenderer),
    store_backup: impl FnOnce() -> bool,
) -> Option<bool> {
    let running = matches!(
        state
            .all_data_payloads()
            .base()
            .status()
            .ride_state()
            .run_state(),
        crate::domain::FloatOutBoyRunState::Running
    );
    state.check_frequency_tracking(running, system_time_ticks);
    state.apply_pending_internal_led_refresh();
    state.render_internal_leds(telemetry, current_time, paint_leds);
    let stored = state.aux_backup_due(odometer).then(|| {
        let stored = store_backup();
        if stored {
            state.record_aux_backup(odometer);
        } else {
            state.record_aux_backup_failure();
        }
        stored
    });
    state.refresh_aux_motor_config_runtime_state(telemetry, system_time_ticks);
    stored
}

/// Start Float Out Boy runtime threads from loader-owned package state.
///
/// Upstream performs this between loader metadata setup
/// (third_party/float-out-boy/src/main.c:2431-2432) and callback registration
/// (third_party/float-out-boy/src/main.c:2455-2459).
fn initialize_float_out_boy_runtime_state(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    orientation: vescpkg_rs::ImuOrientation,
    odometer: OdometerMeters,
) {
    state.refresh_motor_config_runtime_state(telemetry);
    state.initialize_balance_filter(orientation);
    state.initialize_aux_odometer(odometer);
}

#[cfg(all(not(test), target_arch = "arm"))]
fn read_float_out_boy_footpad(gpio: &vescpkg_rs::Gpio, pin: AnalogPin) -> AdcVoltage {
    gpio.acquire_analog(pin)
        .ok()
        .and_then(|pin| {
            pin.set_mode(GpioMode::Analog)
                .ok()
                .and_then(|()| pin.read().ok().flatten())
        })
        .unwrap_or_else(|| AdcVoltage::new(vescpkg_rs::Voltage::ZERO))
}

#[cfg(all(not(test), target_arch = "arm"))]
fn read_float_out_boy_footpads(gpio: &vescpkg_rs::Gpio) -> (AdcVoltage, AdcVoltage) {
    (
        read_float_out_boy_footpad(gpio, AnalogPin::ADC1),
        read_float_out_boy_footpad(gpio, AnalogPin::ADC2),
    )
}

#[cfg(all(not(test), target_arch = "arm"))]
pub fn start_float_out_boy_runtime_threads(
    start: &mut vescpkg_rs::PackageStart<'_>,
) -> Result<(), vescpkg_rs::PackageStartError> {
    let firmware = vescpkg_rs::Firmware::new();
    let odometer = firmware.telemetry().odometer();
    if start
        .with_runtime_state::<FloatOutBoyPackageState, _>(|state| {
            initialize_float_out_boy_runtime_state(
                state,
                firmware.telemetry(),
                firmware.imu().orientation(),
                odometer,
            );
        })
        .is_none()
    {
        return Err(vescpkg_rs::PackageStartError::StateTypeMismatch);
    }
    let threads = float_out_boy_runtime_threads()
        .map_err(|_| vescpkg_rs::PackageStartError::ThreadSpawnFailed)?;
    start.spawn_threads(threads)
}

#[cfg(target_arch = "arm")]
struct FloatOutBoyMainThread;

#[cfg(target_arch = "arm")]
impl vescpkg_rs::FirmwareThread for FloatOutBoyMainThread {
    type State = FloatOutBoyPackageState;

    fn run(mut ctx: vescpkg_rs::ThreadContext<Self::State>) {
        // C map: Float Out Boy v1.2.1 `float_out_boy_thd` starts at
        // `third_party/float-out-boy/src/main.c:767`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let loaded = ctx.with_effects(super::state::load_persisted_config);
            let startup_time = ctx.firmware().clock().now();
            let _ = ctx.with_state_mut(|state| {
                state.begin_startup_configure(&loaded, startup_time);
            });
            let migration = ctx.with_effects(super::state::migrate_legacy_firmware_imu_settings);
            let _ = ctx.with_state_mut(|state| state.finish_startup_configure(migration));
            let imu_frequency = vescpkg_rs::FirmwareSettings.imu_sample_rate().sample_rate();
            let frequency_epoch = ctx.firmware().clock().now();
            let _ = ctx.with_state_mut(|state| {
                state.initialize_frequency_tracking(imu_frequency, frequency_epoch);
            });

            let mut timing = ctx
                .with_state_mut(|state| {
                    FloatOutBoyMainLoopTiming::from_sample_rate(
                        state.configured_main_loop_sample_rate(),
                    )
                })
                .unwrap_or_else(|| {
                    FloatOutBoyMainLoopTiming::from_sample_rate(
                        crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE,
                    )
                });
            let mut next_sleep = timing.nominal_sleep();
            let mut loop_timer = ctx.firmware().clock().timer_now();
            while !ctx.firmware().threads().should_terminate() {
                let firmware = ctx.firmware();
                firmware.threads().sleep_for(next_sleep);
                let elapsed = firmware.clock().timer_elapsed_since(loop_timer);
                loop_timer = firmware.clock().timer_now();
                let system_time_ticks = firmware.clock().now();
                // C map: Float Out Boy `footpad_sensor_update` reads ADC1/ADC2 at
                // `third_party/float-out-boy/src/footpad_sensor.c:28-31`; VESC
                // defines those enum slots at `third_party/vesc/lispBM/c_libs/vesc_c_if.h:219-220`.
                let prepared = {
                    let firmware = ctx.firmware();
                    let (footpad_voltage1, footpad_voltage2) =
                        read_float_out_boy_footpads(firmware.gpio());
                    ctx.with_state_mut(|state| {
                        state.refresh_controller_input(firmware.inputs());
                        prepare_float_out_boy_main_thread_tick(
                            state,
                            firmware.telemetry(),
                            firmware.imu(),
                            firmware.motor(),
                            (footpad_voltage1, footpad_voltage2),
                            system_time_ticks,
                            elapsed,
                        )
                    })
                };

                if prepared
                    .as_ref()
                    .is_some_and(|prepared| prepared.restore_flywheel_config)
                {
                    let loaded = ctx.with_effects(super::state::load_persisted_config);
                    let now = ctx.firmware().clock().now();
                    let _ = ctx.with_state_mut(|state| state.commit_flywheel_restore(&loaded, now));
                    let migration =
                        ctx.with_effects(super::state::migrate_legacy_firmware_imu_settings);
                    let _ = ctx.with_state_mut(|state| state.finish_configure_active(migration));
                }

                let tick = prepared.and_then(|prepared| {
                    let firmware = ctx.firmware();
                    ctx.with_state_mut(|state| {
                        finish_float_out_boy_main_thread_tick(
                            state,
                            firmware.motor(),
                            system_time_ticks,
                            prepared,
                        )
                    })
                });
                if let Some(tick) = tick {
                    if let Some(level) = tick.beeper_pin_level {
                        if let Ok(pin) = ctx.firmware().gpio().acquire_digital(DigitalPin::PPM) {
                            let _ = pin.set_mode(GpioMode::Output);
                            let _ = pin.write(level);
                        }
                    }
                }
                if let Some(configured) = ctx.with_state_mut(|state| {
                    FloatOutBoyMainLoopTiming::from_sample_rate(
                        state.configured_main_loop_sample_rate(),
                    )
                }) {
                    timing = configured;
                }
                next_sleep =
                    timing.sleep_after_work(ctx.firmware().clock().timer_elapsed_since(loop_timer));
            }
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
impl vescpkg_rs::FirmwareThread for FloatOutBoyAuxThread {
    type State = FloatOutBoyPackageState;

    fn run(ctx: vescpkg_rs::ThreadContext<Self::State>) {
        // C map: Float Out Boy v1.2.1 `aux_thd` starts at
        // `third_party/float-out-boy/src/main.c:1130`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let firmware = ctx.firmware();
            let threads = firmware.threads();
            while !threads.should_terminate()
                && !ctx
                    .with_state_mut(|state| state.startup_configured())
                    .unwrap_or(false)
            {
                threads.sleep_for(Duration::from_millis(1));
            }
            if threads.should_terminate() {
                return;
            }
            let (footpad_voltage1, footpad_voltage2) = read_float_out_boy_footpads(firmware.gpio());
            let _ = ctx.with_state_mut(|state| {
                state.setup_loaded_led_hardware_after_threads(footpad_voltage1, footpad_voltage2);
            });
            if let Ok(priority) = ThreadPriority::try_new(-1) {
                let _ = threads.set_priority(priority);
            }
            while !threads.should_terminate() {
                let telemetry = firmware.telemetry();
                let odometer = telemetry.odometer();
                let clock = firmware.clock();
                let system_time_ticks = clock.now();
                let current_time = clock.uptime().as_seconds();
                let prepared = ctx.with_state_mut(|state| {
                    let running = matches!(
                        state
                            .all_data_payloads()
                            .base()
                            .status()
                            .ride_state()
                            .run_state(),
                        crate::domain::FloatOutBoyRunState::Running
                    );
                    state.check_frequency_tracking(running, system_time_ticks);
                    let backup_due = state.aux_backup_due(odometer);
                    let leds = state.prepare_internal_led_aux_work(telemetry, current_time);
                    (leds, backup_due)
                });
                if let Some((leds, backup_due)) = prepared {
                    let mut leds = Some(leds.execute());
                    let stored = backup_due.then(|| firmware.inputs().store_backup().is_ok());
                    let committed = ctx.with_state_mut(|state| {
                        if let Some(leds) = leds.take() {
                            state.commit_internal_led_aux_work(leds);
                        }
                        match stored {
                            Some(true) => state.record_aux_backup(odometer),
                            Some(false) => state.record_aux_backup_failure(),
                            None => {}
                        }
                        state.refresh_aux_motor_config_runtime_state(telemetry, system_time_ticks);
                    });
                    if committed.is_none()
                        && let Some(leds) = leds
                    {
                        leds.destroy_after_rejected_commit();
                    }
                }
                threads.sleep_for(Duration::from_micros(u64::from(
                    FLOAT_OUT_BOY_AUX_LOOP_TIME_US,
                )));
            }
        }

        #[cfg(test)]
        {
            let _ = ctx;
        }
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
