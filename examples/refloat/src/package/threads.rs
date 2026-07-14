//! Refloat runtime-thread startup helpers.
//!
//! Source oracle: Refloat v1.2.1 `third_party/refloat/src/main.c:2439-2449`
//! spawns the main and aux threads after loader metadata setup and before the registration tail.

#[cfg(any(test, target_arch = "arm"))]
use super::state::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use core::time::Duration;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::AnalogPin;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::ThreadStackSize;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::ThreadPriority;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{FirmwareThreads, Imu, MotorOutput, MotorTelemetry};

#[cfg(any(test, target_arch = "arm"))]
// C map: `LEDS_REFRESH_RATE` is `30` at `third_party/refloat/src/leds.h:26`;
// `aux_thd` sleeps `1e6 / LEDS_REFRESH_RATE` at `third_party/refloat/src/main.c:1155`.
const REFLOAT_LEDS_REFRESH_RATE_HZ: u32 = 30;
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_AUX_LOOP_TIME_US: u32 = 1_000_000 / REFLOAT_LEDS_REFRESH_RATE_HZ;

#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::Voltage;
#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefloatRuntimeThread {
    Main,
    Aux,
}

#[cfg(any(test, target_arch = "arm"))]
impl RefloatRuntimeThread {
    const ALL: [Self; 2] = [Self::Main, Self::Aux];

    const fn stack_bytes(self) -> usize {
        match self {
            Self::Main => 1536,
            Self::Aux => 1024,
        }
    }

    const fn stack_size(self) -> ThreadStackSize {
        ThreadStackSize::from_bytes(self.stack_bytes())
    }

    fn name(self) -> vescpkg_rs::ThreadName {
        match self {
            Self::Main => vescpkg_rs::thread_name!("Refloat Main"),
            Self::Aux => vescpkg_rs::thread_name!("Refloat Aux"),
        }
    }
}

/// Start the Refloat runtime threads and store their handles in package state.
///
/// Upstream passes its position-independent refloat_thd, aux_thd, and
/// thread-name addresses directly to spawn with stacks of 1536 and
/// 1024 bytes at third_party/refloat/src/main.c:2438-2445. VESC forwards
/// those runtime addresses and byte counts to chThdCreateStatic at
/// third_party/vesc/lispBM/lispif_c_lib.c:98-125.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn start_refloat_runtime_threads_with(
    threads: &impl FirmwareThreads,
    state: &mut RefloatPackageState,
) -> bool {
    let main_thread = RefloatRuntimeThread::Main;
    let aux_thread = RefloatRuntimeThread::Aux;
    let runtime_threads = vescpkg_rs::ThreadPairSpec::new(
        vescpkg_rs::ThreadSpec::<RefloatPackageState>::new::<RefloatMainThread>(
            main_thread.stack_size(),
            main_thread.name(),
        ),
        vescpkg_rs::ThreadSpec::<()>::stateless::<RefloatAuxThread>(
            aux_thread.stack_size(),
            aux_thread.name(),
        ),
    );
    let Some(runtime_threads) =
        (unsafe { threads.spawn_thread_pair_with_state(runtime_threads, state) })
    else {
        return false;
    };

    state.set_runtime_threads(runtime_threads);
    true
}

/// Request runtime thread termination in Refloat stop order.
///
/// Upstream stops aux first at `third_party/refloat/src/main.c:2404-2406`, then main at
/// `third_party/refloat/src/main.c:2407-2409`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn request_refloat_runtime_thread_termination_with(
    threads: &impl FirmwareThreads,
    state: &RefloatPackageState,
) {
    state.runtime_threads().terminate_reverse(threads);
}

/// Run Refloat's source-backed main thread tick loop.
///
/// Upstream `refloat_thd` calls `configure(d)` at
/// `third_party/refloat/src/main.c:770`, then loops until `should_terminate()` at
/// `third_party/refloat/src/main.c:772`. This narrow Rust tick ports the currently
/// source-backed caller tick, then sleeps the configured `loop_time_us` like
/// `third_party/refloat/src/main.c:1080`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_refloat_main_thread_with<F: FnMut() -> u32>(
    threads: &impl FirmwareThreads,
    mut tick: F,
) {
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(tick() as u64));
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
pub(crate) fn tick_refloat_main_thread_with(
    state: &mut RefloatPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    motor: &impl MotorOutput,
    footpad_adc1: Voltage,
    footpad_adc2: Voltage,
    system_time_ticks: u32,
) -> u32 {
    // C map: `refloat_thd` refreshes runtime inputs, executes state/control
    // logic, applies motor control, then sleeps `loop_time_us` through
    // `third_party/refloat/src/main.c:772-1080`.
    state.refresh_main_loop_runtime_state(
        telemetry,
        imu,
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

    state.configured_loop_time_us()
}

/// Run Refloat's source-backed auxiliary thread scheduler shell.
///
/// Upstream `aux_thd` optionally lowers its current thread priority at
/// `third_party/refloat/src/main.c:1133-1135`, loops until `should_terminate()` at
/// `third_party/refloat/src/main.c:1139`, and sleeps at `1e6 / LEDS_REFRESH_RATE` at
/// `third_party/refloat/src/main.c:1155`. The refresh rate is `30` in
/// `third_party/refloat/src/leds.h:26`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_refloat_aux_thread_with(threads: &impl FirmwareThreads) {
    if let Ok(priority) = ThreadPriority::try_new(-1) {
        let _ = threads.set_priority(priority);
    }
    while !threads.should_terminate() {
        threads.sleep_for(Duration::from_micros(REFLOAT_AUX_LOOP_TIME_US as u64));
    }
}

/// Start Refloat runtime threads from loader-owned package state.
///
/// Upstream performs this between loader metadata setup
/// (third_party/refloat/src/main.c:2431-2432) and callback registration
/// (third_party/refloat/src/main.c:2455-2459).
#[cfg(all(not(test), target_arch = "arm"))]
pub fn start_refloat_runtime_threads(start: &mut vescpkg_rs::PackageStart) -> bool {
    let firmware = vescpkg_rs::Firmware::new();
    unsafe {
        start.with_runtime_state::<RefloatPackageState, _>(|state| {
            state.initialize_balance_filter(firmware.imu().orientation());
            start_refloat_runtime_threads_with(firmware.threads(), state)
        })
    }
    .unwrap_or(false)
}

/// Request runtime thread termination with live firmware bindings.
///
/// Mirrors upstream Refloat stop cleanup at `third_party/refloat/src/main.c:2404-2408`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn request_refloat_runtime_thread_termination(state: &RefloatPackageState) {
    let firmware = vescpkg_rs::Firmware::new();
    request_refloat_runtime_thread_termination_with(firmware.threads(), state);
}

#[cfg(any(test, target_arch = "arm"))]
struct RefloatMainThread;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::FirmwareThread for RefloatMainThread {
    type State = RefloatPackageState;

    fn run(ctx: vescpkg_rs::ThreadContext<'_, Self::State>) {
        // C map: Refloat v1.2.1 `refloat_thd` starts at
        // `third_party/refloat/src/main.c:767`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let (state, firmware) = ctx.into_parts();
            run_refloat_main_thread_with(firmware.threads(), || {
                let system_time_ticks = firmware.app_data().system_time_ticks().as_ticks();
                // C map: Refloat `footpad_sensor_update` reads ADC1/ADC2 at
                // `third_party/refloat/src/footpad_sensor.c:28-31`; VESC
                // defines those enum slots at `third_party/vesc/lispBM/c_libs/vesc_c_if.h:219-220`.
                let (footpad_voltage1, footpad_voltage2) = firmware
                    .gpio()
                    .read_analog_pair(AnalogPin::new(7), AnalogPin::new(8));
                tick_refloat_main_thread_with(
                    state,
                    firmware.telemetry(),
                    firmware.imu(),
                    firmware.motor(),
                    footpad_voltage1,
                    footpad_voltage2,
                    system_time_ticks,
                )
            });
        }

        #[cfg(test)]
        {
            let _ = ctx;
        }
    }
}

#[cfg(any(test, target_arch = "arm"))]
struct RefloatAuxThread;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::StatelessFirmwareThread for RefloatAuxThread {
    fn run(ctx: vescpkg_rs::StatelessThreadContext) {
        // C map: Refloat v1.2.1 `aux_thd` starts at
        // `third_party/refloat/src/main.c:1130`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            run_refloat_aux_thread_with(ctx.threads());
        }

        #[cfg(test)]
        {
            let _ = ctx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::RefloatPackageState;
    use crate::domain::{FootpadSensorState, RefloatAllDataPayloads, RefloatRunState};
    use core::time::Duration;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn refloat_runtime_spawns_threads_with_refloat_startup_stack_sizes() {
        let firmware = FirmwareTest::new();
        let threads = firmware.threads();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        assert!(super::start_refloat_runtime_threads_with(
            threads, &mut state
        ));

        assert_eq!(firmware.thread_spawn_count(), 2);
        let runtime_threads = super::RefloatRuntimeThread::ALL;
        assert_eq!(
            runtime_threads.map(|thread| ThreadStackSize::from_bytes(thread.stack_bytes())),
            firmware.spawned_thread_stack_sizes()
        );
        assert_eq!(
            runtime_threads.map(super::RefloatRuntimeThread::name),
            [
                vescpkg_rs::thread_name!("Refloat Main"),
                vescpkg_rs::thread_name!("Refloat Aux"),
            ]
        );
        assert!(state.runtime_threads().first().is_some());
        assert!(state.runtime_threads().second().is_some());
        assert_eq!(firmware.thread_termination_count(), 0);
    }

    #[test]
    fn refloat_runtime_keeps_main_thread_when_aux_spawn_fails_like_refloat() {
        let firmware = FirmwareTest::new();
        firmware.fail_second_thread_spawn();
        let threads = firmware.threads();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        assert!(super::start_refloat_runtime_threads_with(
            threads, &mut state
        ));

        // C map: Refloat stores each spawn result independently and does not
        // terminate main_thread when aux_thread is null at
        // third_party/refloat/src/main.c:2439-2445.
        assert_eq!(firmware.thread_spawn_count(), 2);
        assert_eq!(firmware.thread_termination_count(), 0);
        assert!(state.runtime_threads().first().is_some());
        assert_eq!(state.runtime_threads().second(), None);
    }

    #[test]
    fn refloat_runtime_stop_terminates_aux_before_main_like_refloat() {
        let firmware = FirmwareTest::new();
        let threads = firmware.threads();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        assert!(super::start_refloat_runtime_threads_with(
            threads, &mut state
        ));
        let expected = [
            state.runtime_threads().second(),
            state.runtime_threads().first(),
        ];
        super::request_refloat_runtime_thread_termination_with(threads, &state);

        assert_eq!(firmware.thread_termination_count(), 2);
        assert_eq!(firmware.terminated_threads(), expected);
    }

    #[test]
    fn refloat_main_thread_tick_refreshes_runtime_state_and_sleeps_like_refloat_loop() {
        let telemetry = FirmwareTest::new().with_runtime_motor(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
            MotorCurrent::new(Current::from_amps(12.25)),
            BatteryCurrent::new(Current::from_amps(6.5)),
            DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
        );
        telemetry.set_imu_startup_done(true);
        telemetry.terminate_threads_after_checks(2);
        let threads = telemetry.threads();
        telemetry.set_imu_attitude(
            ImuRoll::new(AngleRadians::from_radians(0.9)),
            ImuPitch::new(AngleRadians::from_radians(14.0)),
            ImuYaw::new(AngleRadians::from_radians(0.0)),
        );
        let imu = telemetry.imu();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

        super::run_refloat_main_thread_with(threads, || {
            state.refresh_runtime_state(telemetry.telemetry(), imu, 0);
            state.configured_loop_time_us()
        });

        let payloads = state.all_data_payloads();
        assert_eq!(
            payloads.base().status().ride_state().run_state(),
            RefloatRunState::Ready,
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
    fn refloat_main_thread_tick_applies_motor_control_like_refloat_loop() {
        let telemetry = FirmwareTest::new();
        telemetry.terminate_threads_after_checks(2);
        let threads = telemetry.threads();
        telemetry.set_imu_startup_done(true);
        let imu = telemetry.imu();
        let bindings = telemetry.motor();
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        state.request_motor_current(MotorCurrent::new(Current::from_amps(3.5)));

        super::run_refloat_main_thread_with(threads, || {
            super::tick_refloat_main_thread_with(
                &mut state,
                telemetry.telemetry(),
                imu,
                bindings,
                Voltage::from_volts(2.5),
                Voltage::from_volts(0.0),
                0,
            )
        });

        // Upstream `refloat_thd` applies motor control after the state switch at
        // `third_party/refloat/src/main.c:1075`, before sleeping at
        // `third_party/refloat/src/main.c:1080`.
        assert_eq!(telemetry.current_command_count(), 1);
        assert_eq!(telemetry.commanded_current().current().as_amps(), 3.5);
        assert_eq!(
            state.all_data_payloads().base().footpad().state(),
            FootpadSensorState::Left,
        );
    }

    #[test]
    fn refloat_main_thread_sleeps_with_configured_loop_time_like_refloat_loop() {
        let firmware = FirmwareTest::new();
        firmware.terminate_threads_after_checks(2);
        let threads = firmware.threads();
        let mut tick_calls = 0;

        super::run_refloat_main_thread_with(threads, || {
            tick_calls += 1;
            // Upstream `configure(d)` stores `d->loop_time_us` from
            // `d->float_conf.hertz` at `third_party/refloat/src/main.c:190-191`, then
            // `refloat_thd` sleeps that configured value at `third_party/refloat/src/main.c:1080`.
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
    fn refloat_aux_thread_lowers_priority_and_sleeps_like_refloat_aux_loop() {
        let firmware = FirmwareTest::new();
        firmware.terminate_threads_after_checks(2);
        let threads = firmware.threads();

        super::run_refloat_aux_thread_with(threads);

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
