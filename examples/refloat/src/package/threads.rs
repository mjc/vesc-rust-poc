//! Refloat runtime-thread startup helpers.
//!
//! Source oracle: Refloat v1.2.1 `src/main.c:2439-2449` spawns the main and
//! aux threads after loader metadata setup and before the registration tail.

#[cfg(any(test, target_arch = "arm"))]
use super::state::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::ThreadPriority;
use vescpkg_rs::{FirmwareThreadHandle, FirmwareThreadPair};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{
    ImuApi, ImuBindings, MotorControlApi, MotorControlBindings, MotorTelemetryApi,
    MotorTelemetryBindings, ThreadApi, ThreadBindings,
};

#[cfg(any(test, target_arch = "arm"))]
use core::ffi::CStr;

#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_MAIN_THREAD_STACK_BYTES: usize = 4096;
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_AUX_THREAD_STACK_BYTES: usize = 1024;
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_MAIN_THREAD_NAME: &CStr = c"Refloat Main";
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_AUX_THREAD_NAME: &CStr = c"Refloat Aux";
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_LEDS_REFRESH_RATE_HZ: u32 = 30;
#[cfg(any(test, target_arch = "arm"))]
const REFLOAT_AUX_LOOP_TIME_US: u32 = 1_000_000 / REFLOAT_LEDS_REFRESH_RATE_HZ;

/// Refloat runtime thread handles owned by package state.
///
/// Upstream stores these in `Data.main_thread` and `Data.aux_thread` after
/// spawning at `src/main.c:2439-2445`, then requests termination at
/// `src/main.c:2404-2408`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatRuntimeThreads(FirmwareThreadPair);

impl RefloatRuntimeThreads {
    /// Return an empty thread-handle set.
    pub const fn empty() -> Self {
        Self(FirmwareThreadPair::empty())
    }

    /// Build a thread-handle set after both source runtime threads spawned.
    pub const fn new(main_thread: FirmwareThreadHandle, aux_thread: FirmwareThreadHandle) -> Self {
        Self(FirmwareThreadPair::new(main_thread, aux_thread))
    }

    /// Build a Refloat thread-handle set from a generic SDK pair.
    pub const fn from_pair(pair: FirmwareThreadPair) -> Self {
        Self(pair)
    }

    /// Return the main runtime thread handle.
    pub const fn main_thread(self) -> Option<FirmwareThreadHandle> {
        self.0.first()
    }

    /// Return the auxiliary runtime thread handle.
    pub const fn aux_thread(self) -> Option<FirmwareThreadHandle> {
        self.0.second()
    }

    /// Request thread termination in Refloat stop order.
    #[cfg(any(test, target_arch = "arm"))]
    pub fn terminate_in_refloat_order<B: ThreadBindings>(self, threads: &ThreadApi<B>) {
        self.0.terminate_reverse(threads);
    }
}

impl Default for RefloatRuntimeThreads {
    fn default() -> Self {
        Self::empty()
    }
}

/// Start the Refloat runtime threads and store their handles in package state.
///
/// Upstream spawns `refloat_thd` with stack `1536` bytes at `src/main.c:2439`,
/// then spawns `aux_thd` with stack `1024` bytes at `src/main.c:2445`. BLDC's
/// `lispif_spawn` forwards that byte count to `chThdCreateStatic` at
/// `lispBM/lispif_c_lib.c:99-127`; Rust keeps the aux thread at the upstream
/// size and gives the larger generated main-thread frame more room.
///
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn start_refloat_runtime_threads_with<B: ThreadBindings>(
    threads: &ThreadApi<B>,
    state: &mut RefloatPackageState,
) -> bool {
    let Some(runtime_threads) = threads.spawn_pair_with_state(
        vescpkg_rs::FirmwareThreadSpec::<RefloatPackageState>::new::<RefloatMainThread>(
            REFLOAT_MAIN_THREAD_STACK_BYTES,
            REFLOAT_MAIN_THREAD_NAME,
        ),
        vescpkg_rs::FirmwareThreadSpec::<RefloatPackageState>::new::<RefloatAuxThread>(
            REFLOAT_AUX_THREAD_STACK_BYTES,
            REFLOAT_AUX_THREAD_NAME,
        ),
        state,
    ) else {
        return false;
    };

    state.set_runtime_threads(RefloatRuntimeThreads::from_pair(runtime_threads));
    true
}

/// Request runtime thread termination in Refloat stop order.
///
/// Upstream stops aux first at `src/main.c:2404-2406`, then main at
/// `src/main.c:2407-2409`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn request_refloat_runtime_thread_termination_with<B: ThreadBindings>(
    threads: &ThreadApi<B>,
    state: &RefloatPackageState,
) {
    state.runtime_threads().terminate_in_refloat_order(threads);
}

/// Run Refloat's source-backed main thread tick loop.
///
/// Upstream `refloat_thd` calls `configure(d)` at `src/main.c:770`, then loops
/// until `should_terminate()` at `src/main.c:772`. This narrow Rust tick ports
/// the currently source-backed caller tick, then sleeps the configured
/// `loop_time_us` like `src/main.c:1080`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_refloat_main_thread_with<B: ThreadBindings, F: FnMut() -> u32>(
    threads: &ThreadApi<B>,
    mut tick: F,
) {
    while !threads.should_terminate() {
        threads.sleep_us(tick());
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[inline(always)]
pub(crate) fn tick_refloat_main_thread_with<
    M: MotorTelemetryBindings,
    I: ImuBindings,
    C: MotorControlBindings,
>(
    state: &mut RefloatPackageState,
    telemetry: &MotorTelemetryApi<M>,
    imu: &ImuApi<I>,
    motor: &MotorControlApi<C>,
    footpad_adc1: f32,
    footpad_adc2: f32,
    system_time_ticks: u32,
) -> u32 {
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
/// `src/main.c:1133-1135`, loops until `should_terminate()` at
/// `src/main.c:1139`, and sleeps at `1e6 / LEDS_REFRESH_RATE` at
/// `src/main.c:1155`. The refresh rate is `30` in `src/leds.h:26`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn run_refloat_aux_thread_with<B: ThreadBindings>(threads: &ThreadApi<B>) {
    if let Ok(priority) = ThreadPriority::try_new(-1) {
        let _ = threads.set_priority(priority);
    }
    while !threads.should_terminate() {
        threads.sleep_us(REFLOAT_AUX_LOOP_TIME_US);
    }
}

/// Start Refloat runtime threads from loader-owned package state.
///
/// Upstream performs this between loader metadata setup (`src/main.c:2431-2432`)
/// and callback registration (`src/main.c:2455-2459`).
#[cfg(all(not(test), target_arch = "arm"))]
pub fn start_refloat_runtime_threads(start: &mut vescpkg_rs::PackageStart) -> bool {
    let Some(info) = start.loader_info_mut() else {
        return false;
    };
    let Some(state) = RefloatPackageState::from_info_arg(info) else {
        return false;
    };
    let threads = ThreadApi::new(vescpkg_rs::RealThreadBindings);
    start_refloat_runtime_threads_with(&threads, state)
}

/// Request runtime thread termination with live firmware bindings.
///
/// Mirrors upstream Refloat stop cleanup at `src/main.c:2404-2408`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn request_refloat_runtime_thread_termination(state: &RefloatPackageState) {
    let threads = ThreadApi::new(vescpkg_rs::RealThreadBindings);
    request_refloat_runtime_thread_termination_with(&threads, state);
}

#[cfg(any(test, target_arch = "arm"))]
struct RefloatMainThread;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::FirmwareThread for RefloatMainThread {
    type State = RefloatPackageState;

    fn run(state: Option<&'static mut Self::State>) {
        // C map: Refloat v1.2.1 `refloat_thd` starts at `src/main.c:767`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let Some(state) = state else {
                return;
            };
            let threads = ThreadApi::new(vescpkg_rs::RealThreadBindings);
            let telemetry = MotorTelemetryApi::new(vescpkg_rs::RealMotorTelemetryBindings);
            let imu = ImuApi::new(vescpkg_rs::RealImuBindings);
            let motor = MotorControlApi::new(vescpkg_rs::RealMotorControlBindings);
            let app_data = vescpkg_rs::RealBindings;
            let gpio = vescpkg_rs::GpioApi::new(vescpkg_rs::RealGpioBindings);
            run_refloat_main_thread_with(&threads, || {
                let system_time_ticks = vescpkg_rs::AppDataBindings::system_time_ticks(&app_data);
                // C map: Refloat `footpad_sensor_update` reads ADC1/ADC2 at
                // `/Users/mjc/projects/refloat/src/footpad_sensor.c:28-31`; BLDC
                // defines those enum slots at `/Users/mjc/projects/bldc/lispBM/c_libs/vesc_c_if.h:219-220`.
                let (footpad_adc1, footpad_adc2) =
                    gpio.read_analog_pair(vescpkg_rs::ffi::VescPin(7), vescpkg_rs::ffi::VescPin(8));
                tick_refloat_main_thread_with(
                    &mut *state,
                    &telemetry,
                    &imu,
                    &motor,
                    footpad_adc1,
                    footpad_adc2,
                    system_time_ticks,
                )
            });
        }

        #[cfg(test)]
        {
            let _ = state;
        }
    }
}

#[cfg(any(test, target_arch = "arm"))]
struct RefloatAuxThread;

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::FirmwareThread for RefloatAuxThread {
    type State = RefloatPackageState;

    fn run(state: Option<&'static mut Self::State>) {
        // C map: Refloat v1.2.1 `aux_thd` starts at `src/main.c:1130`.
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let _ = state;
            let threads = ThreadApi::new(vescpkg_rs::RealThreadBindings);
            run_refloat_aux_thread_with(&threads);
        }

        #[cfg(test)]
        {
            let _ = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::RefloatPackageState;
    use crate::domain::{FootpadSensorState, RefloatAllDataPayloads, RefloatRunState};
    use core::ffi::CStr;
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::{
        FakeImuBindings, FakeMotorControlBindings, FakeMotorTelemetryBindings, FakeThreadBindings,
    };

    #[test]
    fn refloat_runtime_spawns_main_with_rust_stack_and_aux_like_refloat_startup() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0x2000]);
        let threads = ThreadApi::new(&bindings);
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

        assert!(super::start_refloat_runtime_threads_with(
            &threads, &mut state
        ));

        assert_eq!(bindings.spawn_calls.get(), 2);
        assert_eq!(bindings.spawn_stacks.get(), [4096, 1024]);
        assert_eq!(
            unsafe { CStr::from_ptr(bindings.spawn_names.get()[0].cast()) },
            c"Refloat Main",
        );
        assert_eq!(
            unsafe { CStr::from_ptr(bindings.spawn_names.get()[1].cast()) },
            c"Refloat Aux",
        );
        let state_arg = core::ptr::from_mut(&mut state).cast::<core::ffi::c_void>() as usize;
        assert_eq!(bindings.spawn_args.get(), [state_arg, state_arg]);
        assert_eq!(
            state
                .runtime_threads()
                .main_thread()
                .map(|thread| thread.as_ptr() as usize),
            Some(0x1000),
        );
        assert_eq!(
            state
                .runtime_threads()
                .aux_thread()
                .map(|thread| thread.as_ptr() as usize),
            Some(0x2000),
        );
        assert_eq!(bindings.terminate_calls.get(), 0);
    }

    #[test]
    fn refloat_runtime_terminates_main_thread_when_aux_spawn_fails_like_refloat() {
        let bindings = FakeThreadBindings::with_spawn_results([0x1000, 0]);
        let threads = ThreadApi::new(&bindings);
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

        assert!(!super::start_refloat_runtime_threads_with(
            &threads, &mut state
        ));

        assert_eq!(bindings.spawn_calls.get(), 2);
        assert_eq!(bindings.terminate_calls.get(), 1);
        assert_eq!(bindings.terminated_threads.get(), [0x1000, 0]);
        assert_eq!(state.runtime_threads().main_thread(), None);
        assert_eq!(state.runtime_threads().aux_thread(), None);
    }

    #[test]
    fn refloat_runtime_stop_terminates_aux_before_main_like_refloat() {
        let bindings = FakeThreadBindings::new();
        let threads = ThreadApi::new(&bindings);
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        let main_thread = unsafe { FirmwareThreadHandle::from_raw(0x1000 as *mut _) }
            .expect("nonnull main thread");
        let aux_thread = unsafe { FirmwareThreadHandle::from_raw(0x2000 as *mut _) }
            .expect("nonnull aux thread");
        state.set_runtime_threads(super::RefloatRuntimeThreads::new(main_thread, aux_thread));

        super::request_refloat_runtime_thread_termination_with(&threads, &state);

        assert_eq!(bindings.terminate_calls.get(), 2);
        assert_eq!(bindings.terminated_threads.get(), [0x2000, 0x1000]);
    }

    #[test]
    fn refloat_main_thread_tick_refreshes_runtime_state_and_sleeps_like_refloat_loop() {
        let bindings = FakeThreadBindings::with_should_terminate_after_calls(2);
        let threads = ThreadApi::new(&bindings);
        let telemetry =
            MotorTelemetryApi::new(FakeMotorTelemetryBindings::new().with_runtime_motor(
                ElectricalSpeed::new(Rpm::from_revolutions_per_minute(1234.0)),
                VehicleSpeed::new(Speed::from_meters_per_second(5.5)),
                MotorCurrent::new(Current::from_amps(12.25)),
                BatteryCurrent::new(Current::from_amps(6.5)),
                DutyCycle::new(SignedRatio::from_ratio_const(0.375)),
            ));
        let imu = ImuApi::new(
            FakeImuBindings::new()
                .with_startup_done(true)
                .with_attitude(
                    ImuRoll::new(AngleRadians::from_radians(0.9)),
                    ImuPitch::new(AngleRadians::from_radians(14.0)),
                    ImuYaw::new(AngleRadians::from_radians(0.0)),
                ),
        );
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

        super::run_refloat_main_thread_with(&threads, || {
            state.refresh_runtime_state(&telemetry, &imu, 0);
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
        assert_eq!(bindings.should_terminate_calls.get(), 2);
        assert_eq!(bindings.sleep_calls.get(), 1);
        assert_eq!(bindings.sleep_micros.get(), [1201, 0]);
    }

    #[test]
    fn refloat_main_thread_tick_applies_motor_control_like_refloat_loop() {
        let bindings = FakeThreadBindings::with_should_terminate_after_calls(2);
        let threads = ThreadApi::new(&bindings);
        let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::new());
        let imu = ImuApi::new(FakeImuBindings::new().with_startup_done(true));
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
        state.request_motor_current(MotorCurrent::new(Current::from_amps(3.5)));

        super::run_refloat_main_thread_with(&threads, || {
            super::tick_refloat_main_thread_with(&mut state, &telemetry, &imu, &motor, 2.5, 0.0, 0)
        });

        // Upstream `refloat_thd` applies motor control after the state switch at
        // `src/main.c:1075`, before sleeping at `src/main.c:1080`.
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().current().current().as_amps(), 3.5);
        assert_eq!(
            state.all_data_payloads().base().footpad().state(),
            FootpadSensorState::Left,
        );
    }

    #[test]
    fn refloat_main_thread_sleeps_with_configured_loop_time_like_refloat_loop() {
        let bindings = FakeThreadBindings::with_should_terminate_after_calls(2);
        let threads = ThreadApi::new(&bindings);
        let mut tick_calls = 0;

        super::run_refloat_main_thread_with(&threads, || {
            tick_calls += 1;
            // Upstream `configure(d)` stores `d->loop_time_us` from
            // `d->float_conf.hertz` at `src/main.c:190-191`, then
            // `refloat_thd` sleeps that configured value at `src/main.c:1080`.
            2000
        });

        assert_eq!(tick_calls, 1);
        assert_eq!(bindings.should_terminate_calls.get(), 2);
        assert_eq!(bindings.sleep_calls.get(), 1);
        assert_eq!(bindings.sleep_micros.get(), [2000, 0]);
    }

    #[test]
    fn refloat_aux_thread_lowers_priority_and_sleeps_like_refloat_aux_loop() {
        let bindings = FakeThreadBindings::with_should_terminate_after_calls(2);
        let threads = ThreadApi::new(&bindings);

        super::run_refloat_aux_thread_with(&threads);

        assert_eq!(bindings.priority_calls.get(), 1);
        assert_eq!(bindings.priorities.get(), [-1, 0]);
        assert_eq!(bindings.should_terminate_calls.get(), 2);
        assert_eq!(bindings.sleep_calls.get(), 1);
        assert_eq!(bindings.sleep_micros.get(), [33_333, 0]);
    }
}
