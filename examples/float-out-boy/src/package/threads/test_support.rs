use super::*;

/// Run Float Out Boy's source-backed main thread tick loop.
///
/// Upstream `float_out_boy_thd` calls `configure(d)` at
/// `third_party/float-out-boy/src/main.c:770`, then loops until `should_terminate()` at
/// `third_party/float-out-boy/src/main.c:772`. This narrow Rust tick ports the currently
/// source-backed caller tick, then sleeps the configured `loop_time_us` like
/// `third_party/float-out-boy/src/main.c:1080`.
pub(super) fn run_float_out_boy_main_thread_with<F: FnMut() -> u32>(
    threads: &impl FirmwareThreads,
    mut tick: F,
) {
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(u64::from(tick())));
    }
}

impl FloatOutBoyMainThreadTick {
    pub(super) const fn beeper_level(self) -> Option<vescpkg_rs::DigitalOutputLevel> {
        self.beeper_pin_level
    }
}

#[inline]
pub(super) fn tick_float_out_boy_main_thread_with(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    motor: &impl MotorOutput,
    footpad_adc1: AdcVoltage,
    footpad_adc2: AdcVoltage,
    system_time_ticks: TimestampTicks,
) -> FloatOutBoyMainThreadTick {
    let prepared = prepare_float_out_boy_main_thread_tick(
        state,
        telemetry,
        imu,
        motor,
        footpad_adc1,
        footpad_adc2,
        system_time_ticks,
    );
    if prepared.restore_flywheel_config {
        let loaded = vescpkg_rs::test_support::with_firmware_effects(
            super::super::state::load_persisted_config,
        );
        state.commit_flywheel_restore(&loaded, system_time_ticks);
        let migration = vescpkg_rs::test_support::with_firmware_effects(
            super::super::state::migrate_legacy_firmware_imu_settings,
        );
        state.finish_configure_active(migration);
    }
    finish_float_out_boy_main_thread_tick(state, motor, system_time_ticks, prepared)
}

/// Run Float Out Boy's source-backed auxiliary thread scheduler shell.
///
/// Upstream `aux_thd` optionally lowers its current thread priority at
/// `third_party/float-out-boy/src/main.c:1133-1135`, checks the non-running odometer backup
/// threshold from `main.c:1142-1146`, loops until `should_terminate()` at `main.c:1139`,
/// and sleeps at `1e6 / LEDS_REFRESH_RATE` at `main.c:1155`. The refresh rate is `30` in
/// `third_party/float-out-boy/src/leds.h:26`.
pub(super) fn run_float_out_boy_aux_thread_with(threads: &impl FirmwareThreads) {
    if let Ok(priority) = ThreadPriority::try_new(-1) {
        let _ = threads.set_priority(priority);
    }
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(u64::from(
            FLOAT_OUT_BOY_AUX_LOOP_TIME_US,
        )));
    }
}
