//! Host-side fake firmware bindings for unit tests in dependent crates.

#![allow(clippy::cast_precision_loss)]

use core::cell::Cell;
use core::ffi::{CStr, c_char};

use crate::bindings::LbmBindings;
#[cfg(test)]
use crate::bindings::{AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings};
#[cfg(test)]
use crate::ffi::{CustomConfigGet, CustomConfigSet, CustomConfigXml, ImuReadCallback};
use crate::lifecycle_core::PackageLifecycle;
pub use crate::types::loader::LoaderInfo;
#[cfg(any(test, feature = "test-support"))]
#[cfg(test)]
use crate::{PackageArgument, PackageProgramAddress};
#[cfg(test)]
use vescpkg_rs_sys::AppDataHandler;

/// Host fixture that observes ordinary `Firmware` calls through the test FFI.
#[cfg(all(feature = "test-support", not(test)))]
pub struct FirmwareTest {
    firmware: crate::Firmware,
    _lock: crate::test_ffi::FirmwareLockGuard,
}

#[cfg(all(feature = "test-support", not(test)))]
impl FirmwareTest {
    /// Reset this thread's fake firmware state and construct normal capabilities.
    #[must_use]
    pub fn new() -> Self {
        crate::gpio::reset_leases();
        crate::can_bus::reset_receiver_registrations();
        let lock = crate::test_ffi::lock_firmware();
        Self {
            firmware: crate::Firmware::new(),
            _lock: lock,
        }
    }

    /// Borrow the same typed motor capability package code uses on hardware.
    #[must_use]
    pub fn motor(&self) -> &impl crate::MotorOutput {
        self.firmware.motor()
    }

    /// Borrow the same typed motor telemetry package code uses on hardware.
    #[must_use]
    pub fn telemetry(&self) -> &impl crate::MotorTelemetry {
        self.firmware.telemetry()
    }

    /// Borrow the same typed IMU capability package code uses on hardware.
    #[must_use]
    pub fn imu(&self) -> &impl crate::Imu {
        self.firmware.imu()
    }

    /// Borrow typed PPM and UART controller inputs.
    #[must_use]
    pub fn input(&self) -> &crate::ControllerInput {
        self.firmware.input()
    }

    /// Configure the latest PPM input sample.
    pub fn set_ppm_input(&self, input: crate::PpmInput, age: crate::PpmAge) {
        crate::test_ffi::set_ppm_input(input, age);
    }

    /// Configure the latest UART remote input sample.
    pub fn set_remote_input(&self, input: crate::JoystickY, age: crate::RemoteAge) {
        crate::test_ffi::set_remote_input(input, age);
    }

    /// Access the same package custom-EEPROM range used on hardware.
    #[must_use]
    pub const fn eeprom(&self) -> crate::CustomEeprom {
        crate::CustomEeprom::new()
    }

    /// Access the same byte-addressed NVM capability used on hardware.
    #[must_use]
    pub const fn nvm(&self) -> crate::Nvm {
        crate::Nvm::new()
    }

    /// Borrow the fake firmware CAN transport used by package tests.
    #[must_use]
    pub fn can(&self) -> &crate::CanBus {
        self.firmware.can()
    }

    /// Borrow the fake firmware GPIO capability used by package tests.
    #[must_use]
    pub fn gpio(&self) -> &crate::Gpio {
        self.firmware.gpio()
    }

    /// Borrow the typed controller input capability used by package code.
    #[must_use]
    pub fn inputs(&self) -> &crate::FirmwareInputs {
        self.firmware.inputs()
    }

    /// Borrow the firmware clock capability used on hardware.
    #[must_use]
    pub fn clock(&self) -> &crate::FirmwareClock {
        self.firmware.clock()
    }

    /// Set the fake firmware system timestamp.
    pub fn set_clock_ticks(&self, ticks: u32) {
        crate::test_ffi::set_clock_ticks(ticks);
    }

    /// Set the fake high-resolution timer counter.
    pub fn set_timer_ticks(&self, ticks: u32) {
        crate::test_ffi::set_timer_ticks(ticks);
    }

    /// Make the fake firmware reject every NVM operation.
    pub fn fail_nvm_operations(&self) {
        crate::test_ffi::fail_nvm_operations(true);
    }

    /// Make the fake LispBM evaluator reject every process message.
    pub fn fail_lisp_messages(&self) {
        crate::test_ffi::fail_lisp_messages(true);
    }

    /// Return the number of fake firmware mutex locks.
    #[must_use]
    pub fn mutex_lock_count(&self) -> usize {
        crate::test_ffi::mutex_lock_count()
    }

    /// Return the number of fake firmware mutex unlocks.
    #[must_use]
    pub fn mutex_unlock_count(&self) -> usize {
        crate::test_ffi::mutex_unlock_count()
    }

    /// Return the number of fake firmware frees for mutex ownership.
    #[must_use]
    pub fn mutex_free_count(&self) -> usize {
        crate::test_ffi::mutex_free_count()
    }

    /// Return the number of fake firmware semaphore waits.
    #[must_use]
    pub fn semaphore_wait_count(&self) -> usize {
        crate::test_ffi::semaphore_wait_count()
    }

    /// Return the most recent fake timed-wait timeout.
    #[must_use]
    pub fn semaphore_timed_wait_ticks(&self) -> Option<u32> {
        crate::test_ffi::semaphore_timed_wait_ticks()
    }

    /// Return the number of fake firmware semaphore signals.
    #[must_use]
    pub fn semaphore_signal_count(&self) -> usize {
        crate::test_ffi::semaphore_signal_count()
    }

    /// Return the number of fake firmware semaphore resets.
    #[must_use]
    pub fn semaphore_reset_count(&self) -> usize {
        crate::test_ffi::semaphore_reset_count()
    }

    /// Return the number of fake firmware semaphore releases.
    #[must_use]
    pub fn semaphore_free_count(&self) -> usize {
        crate::test_ffi::semaphore_free_count()
    }

    /// Make the fake firmware reject mutex creation.
    pub fn fail_mutex_creation(&self) {
        crate::test_ffi::fail_mutex_creation(true);
    }

    /// Make the fake firmware reject semaphore creation.
    pub fn fail_semaphore_creation(&self) {
        crate::test_ffi::fail_semaphore_creation(true);
    }

    /// Make the fake firmware time out semaphore waits.
    pub fn fail_semaphore_timeout(&self) {
        crate::test_ffi::fail_semaphore_timeout(true);
    }

    /// Configure whether the fake firmware exposes shutdown inhibition.
    pub fn set_shutdown_disable_supported(&self, supported: bool) {
        crate::test_ffi::set_shutdown_disable_supported(supported);
    }

    /// Return whether fake firmware automatic shutdown is currently inhibited.
    #[must_use]
    pub fn shutdown_disabled(&self) -> bool {
        crate::test_ffi::shutdown_disabled()
    }

    /// Make writes to one custom-EEPROM address fail.
    pub fn fail_eeprom_write(&self, address: crate::CustomEepromAddress) {
        crate::test_ffi::fail_eeprom_write(address);
    }

    /// Configure whether firmware IMU startup has completed.
    pub fn set_imu_ready(&self, done: bool) {
        crate::test_ffi::set_imu_startup_done(done);
    }

    /// Configure the typed firmware IMU attitude.
    pub fn set_imu_attitude(
        &self,
        roll: crate::ImuRoll,
        pitch: crate::ImuPitch,
        yaw: crate::ImuYaw,
    ) {
        crate::test_ffi::set_imu_attitude(roll, pitch, yaw);
    }

    /// Configure the typed firmware IMU angular rate.
    pub fn set_imu_angular_rate(&self, rate: crate::ImuAngularRate) {
        crate::test_ffi::set_imu_angular_rate(rate);
    }

    /// Configure the typed firmware IMU orientation.
    pub fn set_imu_orientation(&self, orientation: crate::ImuOrientation) {
        crate::test_ffi::set_imu_orientation(orientation);
    }

    /// Borrow the same typed thread capability package code uses on hardware.
    #[must_use]
    pub fn threads(&self) -> &impl crate::FirmwareThreads {
        self.firmware.threads()
    }

    /// Make the second firmware thread spawn fail.
    pub fn fail_second_thread_spawn(&self) {
        crate::test_ffi::fail_second_thread_spawn();
    }

    /// Request loop termination on the given poll.
    pub fn terminate_threads_after_checks(&self, checks: usize) {
        crate::test_ffi::terminate_threads_after_checks(checks);
    }

    #[must_use]
    /// Return the number of attempted firmware thread spawns.
    pub fn thread_spawn_count(&self) -> usize {
        crate::test_ffi::thread_spawn_count()
    }

    #[must_use]
    /// Return stack sizes from the first two firmware thread spawns.
    pub fn spawned_thread_working_area_sizes(&self) -> [Option<crate::ThreadWorkingAreaSize>; 2] {
        crate::test_ffi::thread_spawn_stacks().map(|bytes| {
            (bytes != 0).then(|| crate::ThreadWorkingAreaSize::try_from_bytes(bytes).unwrap())
        })
    }

    #[must_use]
    /// Return the number of requested thread terminations.
    pub fn thread_termination_count(&self) -> usize {
        crate::test_ffi::thread_termination_count()
    }

    #[must_use]
    /// Return raw addresses from the first two termination requests.
    pub fn terminated_thread_addresses(&self) -> [Option<usize>; 2] {
        crate::test_ffi::thread_terminated().map(|address| (address != 0).then_some(address))
    }

    #[must_use]
    /// Return the number of termination-condition polls.
    pub fn thread_termination_check_count(&self) -> usize {
        crate::test_ffi::thread_termination_check_count()
    }

    #[must_use]
    /// Return the number of firmware sleep requests.
    pub fn thread_sleep_count(&self) -> usize {
        crate::test_ffi::thread_sleep_count()
    }

    #[must_use]
    /// Return durations from the first two firmware sleep requests.
    pub fn thread_sleep_durations(&self) -> [core::time::Duration; 2] {
        crate::test_ffi::thread_sleep_micros()
            .map(|micros| core::time::Duration::from_micros(u64::from(micros)))
    }

    #[must_use]
    /// Return the number of firmware priority changes.
    pub fn thread_priority_change_count(&self) -> usize {
        crate::test_ffi::thread_priority_count()
    }

    #[must_use]
    /// Return priorities from the first two firmware priority changes.
    pub fn thread_priorities(&self) -> [Option<crate::ThreadPriority>; 2] {
        crate::test_ffi::thread_priorities().map(|priority| {
            i8::try_from(priority)
                .ok()
                .and_then(|priority| crate::ThreadPriority::try_new(priority).ok())
        })
    }

    #[must_use]
    /// Configure the typed runtime motor values returned by firmware telemetry.
    pub fn with_runtime_motor(
        self,
        electrical_speed: crate::ElectricalSpeed,
        vehicle_speed: crate::VehicleSpeed,
        motor_current: crate::TotalMotorCurrent,
        input_current: crate::InputCurrent,
        duty_cycle: crate::DutyCycle,
    ) -> Self {
        crate::test_ffi::set_runtime_motor(
            electrical_speed,
            vehicle_speed,
            motor_current,
            input_current,
            duty_cycle,
        );
        self
    }

    #[must_use]
    /// Configure the typed positive and braking motor-current limits.
    pub fn with_motor_current_limits(
        self,
        max: crate::MotorCurrentLimit,
        min: crate::MotorCurrentLimit,
    ) -> Self {
        crate::test_ffi::set_motor_current_limits(max, min);
        self
    }

    #[must_use]
    /// Configure the typed positive and regenerative input-current limits.
    pub fn with_input_current_limits(
        self,
        max: crate::InputCurrentLimit,
        min: crate::InputCurrentLimit,
    ) -> Self {
        crate::test_ffi::set_input_current_limits(max, min);
        self
    }

    #[must_use]
    /// Configure the typed maximum motor duty-cycle limit.
    pub fn with_duty_cycle_limit(self, limit: crate::DutyCycleLimit) -> Self {
        crate::test_ffi::set_duty_cycle_limit(limit);
        self
    }

    #[must_use]
    /// Configure the typed MOSFET and motor temperature limit-start thresholds.
    pub fn with_temperature_limit_starts(
        self,
        mosfet: crate::TemperatureLimitStart,
        motor: crate::TemperatureLimitStart,
    ) -> Self {
        crate::test_ffi::set_temperature_limit_starts(mosfet, motor);
        self
    }

    #[must_use]
    /// Configure the typed firmware battery cell count.
    pub fn with_battery_cell_count(self, count: crate::BatteryCellCount) -> Self {
        crate::test_ffi::set_battery_cell_count(count);
        self
    }

    #[must_use]
    /// Configure the filtered directional motor current.
    pub fn with_directional_motor_current(self, current: crate::DirectionalMotorCurrent) -> Self {
        crate::test_ffi::set_directional_motor_current(current);
        self
    }

    #[must_use]
    /// Configure the typed absolute trip distance.
    pub fn with_trip_distance(self, distance: crate::TripDistance) -> Self {
        crate::test_ffi::set_distance_abs(distance);
        self
    }

    #[must_use]
    /// Configure the typed MOSFET and motor temperatures.
    pub fn with_temperatures(
        self,
        mosfet: crate::MosfetTemperature,
        motor: crate::MotorTemperature,
    ) -> Self {
        crate::test_ffi::set_temperatures(mosfet, motor);
        self
    }

    #[must_use]
    /// Configure the typed accumulated ride totals.
    pub fn with_ride_totals(
        self,
        odometer: crate::OdometerMeters,
        amp_hours_discharged: crate::AmpHoursDischarged,
        amp_hours_charged: crate::AmpHoursCharged,
        watt_hours_discharged: crate::WattHoursDischarged,
        watt_hours_charged: crate::WattHoursCharged,
        battery_level: crate::BatteryLevel,
    ) -> Self {
        crate::test_ffi::set_ride_totals(
            odometer,
            amp_hours_discharged,
            amp_hours_charged,
            watt_hours_discharged,
            watt_hours_charged,
            battery_level,
        );
        self
    }

    #[must_use]
    /// Configure the typed firmware fault code.
    pub fn with_firmware_fault(self, fault: crate::FirmwareFaultCode) -> Self {
        crate::test_ffi::set_firmware_fault(fault);
        self
    }

    #[must_use]
    /// Configure the typed filtered input voltage.
    pub fn with_input_voltage(self, voltage: crate::InputVoltage) -> Self {
        crate::test_ffi::set_input_voltage(voltage);
        self
    }

    #[must_use]
    /// Configure filtered input voltage and motor temperatures together.
    pub fn with_input_voltage_and_temperatures(
        self,
        voltage: crate::InputVoltage,
        mosfet: crate::MosfetTemperature,
        motor: crate::MotorTemperature,
    ) -> Self {
        crate::test_ffi::set_input_voltage(voltage);
        crate::test_ffi::set_temperatures(mosfet, motor);
        self
    }

    #[must_use]
    /// Configure the optional typed FOC d-axis current.
    pub fn with_d_axis_current(self, current: Option<crate::DCurrent>) -> Self {
        crate::test_ffi::set_foc_id_current(current);
        self
    }

    #[must_use]
    /// Return the number of motor-watchdog resets.
    pub fn keep_alive_count(&self) -> usize {
        crate::test_ffi::motor_output().keep_alive_count
    }

    #[must_use]
    /// Return the number of current-off-delay writes.
    pub fn current_off_delay_count(&self) -> usize {
        crate::test_ffi::motor_output().current_off_delay_count
    }

    #[must_use]
    /// Return the number of motor-current writes.
    pub fn current_command_count(&self) -> usize {
        crate::test_ffi::motor_output().current_count
    }

    #[must_use]
    /// Return the number of duty-cycle writes.
    pub fn duty_command_count(&self) -> usize {
        crate::test_ffi::motor_output().duty_count
    }

    #[must_use]
    /// Return the number of brake-current writes.
    pub fn brake_current_command_count(&self) -> usize {
        crate::test_ffi::motor_output().brake_current_count
    }

    #[must_use]
    /// Return the number of attempted FOC tone writes.
    pub fn foc_tone_command_count(&self) -> usize {
        crate::test_ffi::motor_output().foc_tone_count
    }

    #[must_use]
    /// Return the latest current-off-delay write as the SDK domain type.
    pub fn commanded_current_off_delay(&self) -> crate::CurrentOffDelay {
        crate::CurrentOffDelay::new(crate::VescSeconds::from_seconds(
            crate::test_ffi::motor_output().current_off_delay,
        ))
    }

    #[must_use]
    /// Return the latest motor-current write as the SDK domain type.
    pub fn commanded_current(&self) -> crate::MotorCurrent {
        crate::MotorCurrent::new(crate::Current::from_amps(
            crate::test_ffi::motor_output().current,
        ))
    }

    #[must_use]
    /// Return the latest duty-cycle write as the SDK domain type.
    pub fn commanded_duty(&self) -> crate::DutyCycle {
        crate::DutyCycle::new(crate::SignedRatio::clamped(
            crate::test_ffi::motor_output().duty,
        ))
    }

    #[must_use]
    /// Return the latest brake-current write as the SDK domain type.
    pub fn commanded_brake_current(&self) -> crate::BrakeCurrent {
        crate::BrakeCurrent::new(crate::Current::from_amps(
            crate::test_ffi::motor_output().brake_current,
        ))
    }

    #[must_use]
    /// Return the latest FOC tone channel.
    pub fn commanded_foc_tone_channel(&self) -> crate::AudioChannel {
        crate::AudioChannel::try_new(crate::test_ffi::motor_output().foc_tone_channel as u8)
            .expect("fake firmware stores a valid audio channel")
    }

    #[must_use]
    /// Return the latest FOC tone frequency.
    pub fn commanded_foc_tone_frequency(&self) -> crate::AudioFrequency {
        crate::AudioFrequency::new(crate::Frequency::from_hertz(
            crate::test_ffi::motor_output().foc_tone_frequency,
        ))
    }

    #[must_use]
    /// Return the latest FOC tone voltage.
    pub fn commanded_foc_tone_voltage(&self) -> crate::AudioVoltage {
        crate::AudioVoltage::new(crate::Voltage::from_volts(
            crate::test_ffi::motor_output().foc_tone_voltage,
        ))
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl Default for FirmwareTest {
    fn default() -> Self {
        Self::new()
    }
}

use vescpkg_rs_sys::ExtensionHandler;
use vescpkg_rs_sys::LbmValue;

/// Install borrowed state for callback-focused host tests.
pub fn install_state<'a, T: Send + 'static>(
    store: &'a crate::PackageStateStore<T>,
    state: &'a mut T,
) -> impl Drop + 'a {
    unsafe { store.install(state) }.unwrap();
    struct ClearOnDrop<'a, T: Send + 'static>(&'a crate::PackageStateStore<T>);
    impl<T: Send + 'static> Drop for ClearOnDrop<'_, T> {
        fn drop(&mut self) {
            self.0.clear();
        }
    }
    ClearOnDrop(store)
}

/// Clear package state left behind by a callback-focused host test.
pub fn clear_state<T: Send + 'static>(store: &crate::PackageStateStore<T>) {
    store.clear();
}

/// Build a startup context for a typed loader fixture.
pub fn package_start(info: &mut crate::LoaderInfo) -> crate::PackageStart<'_> {
    crate::PackageStart::from_info(info)
}

/// Run and clear the loader-owned package stop hook.
pub fn stop_package(info: &mut crate::LoaderInfo) -> bool {
    info.stop_for_test()
}

/// Build a startup context with no loader metadata for rejection-path tests.
pub fn package_start_without_loader() -> crate::PackageStart<'static> {
    unsafe { crate::PackageStart::from_raw(core::ptr::null_mut()) }
}

/// Semantic extension registry for downstream package tests.
pub struct TestExtensionRegistry {
    bindings: FakeBindings,
}

impl TestExtensionRegistry {
    /// Create a registry that accepts extension registration.
    #[must_use]
    pub fn accepting() -> Self {
        Self {
            bindings: FakeBindings::new(),
        }
    }

    /// Create a registry that rejects extension registration.
    #[must_use]
    pub fn rejecting() -> Self {
        Self {
            bindings: FakeBindings::rejecting(),
        }
    }

    /// Register extension descriptors through the package loader test seam.
    pub fn register<const N: usize>(
        &self,
        start: &mut crate::PackageStart<'_>,
        descriptors: [crate::ExtensionDescriptor; N],
    ) -> Result<crate::ExtensionRegistration, crate::PackageStartError> {
        let lifecycle = PackageLifecycle::new(&self.bindings);
        start.register_extensions_with(&lifecycle, descriptors)
    }

    /// Number of registration calls observed.
    #[must_use]
    pub fn registration_count(&self) -> usize {
        self.bindings.add_calls.get()
    }

    /// Most recently registered extension name.
    #[must_use]
    pub fn last_registered_name(&self) -> Option<&'static str> {
        let pointer = self.bindings.last_name.get();
        if pointer == 0 {
            return None;
        }

        unsafe { CStr::from_ptr(pointer as *const c_char) }
            .to_str()
            .ok()
    }
}

/// Private extension registration bindings used by `TestExtensionRegistry`.
pub(crate) struct FakeBindings {
    /// Number of extension add calls observed.
    pub(crate) add_calls: Cell<usize>,
    /// Last extension name pointer passed to registration.
    pub(crate) last_name: Cell<usize>,
    /// Last handler pointer passed to registration.
    pub(crate) last_handler: Cell<usize>,
    add_results: Cell<[bool; 2]>,
}

impl Default for FakeBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBindings {
    /// Creates fake bindings that accept both extension registrations.
    pub fn new() -> Self {
        Self::with_add_results([true, true])
    }

    /// Creates fake bindings that reject extension registrations.
    pub fn rejecting() -> Self {
        Self::with_add_results([false, false])
    }

    /// Creates fake bindings with explicit add results for two registrations.
    pub fn with_add_results(add_results: [bool; 2]) -> Self {
        Self {
            add_calls: Cell::new(0),
            last_name: Cell::new(0),
            last_handler: Cell::new(0),
            add_results: Cell::new(add_results),
        }
    }
}

impl LbmBindings for FakeBindings {
    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
        self.add_calls.set(self.add_calls.get() + 1);
        self.last_name.set(name as usize);
        self.last_handler.set(handler as usize);
        let index = self.add_calls.get().saturating_sub(1).min(1);
        self.add_results.get()[index]
    }

    unsafe fn is_number(&self, _value: LbmValue) -> bool {
        false
    }

    unsafe fn decode_i32(&self, _value: LbmValue) -> i32 {
        unreachable!("extension registration does not decode LispBM values")
    }

    unsafe fn decode_f32(&self, _value: LbmValue) -> f32 {
        unreachable!("extension registration does not decode LispBM values")
    }

    #[cfg(not(test))]
    fn encode_true(&self) -> LbmValue {
        LbmValue(1)
    }

    #[cfg(not(test))]
    fn encode_nil(&self) -> LbmValue {
        LbmValue(0)
    }
}

/// Fake app-data bindings used by lifecycle and loopback runtime tests.
#[cfg(test)]
pub(crate) struct FakeAppDataBindings {
    /// Number of app-data handler invocations observed.
    pub handler_calls: Cell<usize>,
    /// Tick count returned by the fake timer binding.
    pub ticks: Cell<u32>,
    /// Number of app-data send calls observed.
    pub send_calls: Cell<usize>,
    /// Last app-data handler pointer passed to registration.
    pub last_handler: Cell<usize>,
    /// Last outbound data pointer passed to send.
    pub last_data: Cell<usize>,
    /// Last outbound data length passed to send.
    pub last_len: Cell<u32>,
    /// Number of custom-config registration calls observed.
    pub custom_config_register_calls: Cell<usize>,
    /// Last custom-config get callback pointer passed to registration.
    pub last_custom_config_get: Cell<usize>,
    /// Last custom-config set callback pointer passed to registration.
    pub last_custom_config_set: Cell<usize>,
    /// Last custom-config XML callback pointer passed to registration.
    pub last_custom_config_xml: Cell<usize>,
    /// Number of custom-config clear calls observed.
    pub custom_config_clear_calls: Cell<usize>,
    /// Number of IMU read callback registration calls observed.
    pub imu_read_callback_calls: Cell<usize>,
    /// Last IMU read callback pointer passed to registration.
    pub last_imu_read_callback: Cell<usize>,
    /// Fake package ARG pointer returned by the app-data binding.
    pub app_data_arg: Cell<usize>,
    set_handler_result: Cell<bool>,
}

#[derive(Clone, Copy)]
#[cfg(any(test, feature = "test-support"))]
#[cfg(test)]
enum FirmwareCallResult {
    Accept,
    Reject,
}

#[cfg(any(test, feature = "test-support"))]
#[cfg(test)]
impl FirmwareCallResult {
    const fn from_bool(value: bool) -> Self {
        if value { Self::Accept } else { Self::Reject }
    }

    const fn accepted(self) -> bool {
        matches!(self, Self::Accept)
    }
}

#[derive(Clone, Copy)]
#[cfg(any(test, feature = "test-support"))]
#[cfg(test)]
struct FakeAppDataResults {
    set_handler: FirmwareCallResult,
}

#[cfg(any(test, feature = "test-support"))]
#[cfg(test)]
impl FakeAppDataResults {
    const ACCEPT_ALL: Self = Self {
        set_handler: FirmwareCallResult::Accept,
    };
}

#[cfg(test)]
impl Default for FakeAppDataBindings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FakeAppDataBindings {
    /// Creates fake app-data bindings with zero timer ticks.
    pub fn new() -> Self {
        Self::with_ticks(0)
    }

    /// Creates fake app-data bindings returning `ticks` from the timer.
    pub fn with_ticks(ticks: u32) -> Self {
        Self::with_ticks_and_results(ticks, FakeAppDataResults::ACCEPT_ALL)
    }

    /// Creates fake app-data bindings with an explicit handler registration result.
    #[cfg(test)]
    pub fn with_set_handler_result(set_handler_result: bool) -> Self {
        Self::with_ticks_and_results(
            0,
            FakeAppDataResults {
                set_handler: FirmwareCallResult::from_bool(set_handler_result),
            },
        )
    }

    fn with_ticks_and_results(ticks: u32, results: FakeAppDataResults) -> Self {
        Self {
            handler_calls: Cell::new(0),
            ticks: Cell::new(ticks),
            send_calls: Cell::new(0),
            last_handler: Cell::new(0),
            last_data: Cell::new(0),
            last_len: Cell::new(0),
            custom_config_register_calls: Cell::new(0),
            last_custom_config_get: Cell::new(0),
            last_custom_config_set: Cell::new(0),
            last_custom_config_xml: Cell::new(0),
            custom_config_clear_calls: Cell::new(0),
            imu_read_callback_calls: Cell::new(0),
            last_imu_read_callback: Cell::new(0),
            app_data_arg: Cell::new(0),
            set_handler_result: Cell::new(results.set_handler.accepted()),
        }
    }
}

#[cfg(test)]
impl AppDataBindings for FakeAppDataBindings {
    unsafe fn set_app_data_handler(&self, handler: AppDataHandler) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(handler as *const () as usize);
        self.set_handler_result.get()
    }

    unsafe fn clear_app_data_handler(&self) -> bool {
        self.handler_calls.set(self.handler_calls.get() + 1);
        self.last_handler.set(0);
        true
    }

    fn system_time_ticks(&self) -> u32 {
        self.ticks.get()
    }

    fn system_time_seconds(&self) -> f32 {
        self.ticks.get() as f32 / 10_000.0
    }

    fn timestamp_age_seconds(&self, timestamp: u32) -> f32 {
        self.ticks.get().wrapping_sub(timestamp) as f32 / 10_000.0
    }

    fn timer_time_now(&self) -> u32 {
        self.ticks.get()
    }

    fn timer_seconds_elapsed_since(&self, timestamp: u32) -> f32 {
        self.ticks.get().wrapping_sub(timestamp) as f32 / 1_000_000.0
    }

    fn arg(&self, _prog_addr: PackageProgramAddress) -> Option<PackageArgument> {
        core::ptr::NonNull::new(self.app_data_arg.get() as *mut core::ffi::c_void)
            .map(PackageArgument::new)
    }

    unsafe fn send_app_data(&self, data: *const u8, len: u32) {
        self.send_calls.set(self.send_calls.get() + 1);
        self.last_data.set(data as usize);
        self.last_len.set(len);
    }
}

#[cfg(test)]
impl CustomConfigBindings for FakeAppDataBindings {
    unsafe fn register_custom_config(
        &self,
        get_cfg: CustomConfigGet,
        set_cfg: CustomConfigSet,
        get_cfg_xml: CustomConfigXml,
    ) {
        self.custom_config_register_calls
            .set(self.custom_config_register_calls.get() + 1);
        self.last_custom_config_get
            .set(get_cfg as *const () as usize);
        self.last_custom_config_set
            .set(set_cfg as *const () as usize);
        self.last_custom_config_xml
            .set(get_cfg_xml as *const () as usize);
    }

    unsafe fn clear_custom_configs(&self) {
        self.custom_config_clear_calls
            .set(self.custom_config_clear_calls.get() + 1);
        self.last_custom_config_get.set(0);
        self.last_custom_config_set.set(0);
        self.last_custom_config_xml.set(0);
    }
}

#[cfg(test)]
impl ImuReadCallbackBindings for FakeAppDataBindings {
    unsafe fn set_imu_read_callback(&self, callback: ImuReadCallback) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback
            .set(callback as *const () as usize);
    }

    unsafe fn clear_imu_read_callback(&self) {
        self.imu_read_callback_calls
            .set(self.imu_read_callback_calls.get() + 1);
        self.last_imu_read_callback.set(0);
    }
}

/// C ABI stubs linked by host-side tests.
#[cfg(any(test, feature = "test-support"))]
pub(crate) mod stubs {
    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real extension handler ABI.
    #[cfg(test)]
    pub unsafe extern "C" fn extension_handler(_args: *mut u32, _count: u32) -> u32 {
        0
    }

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real stop handler ABI.
    #[cfg(test)]
    pub unsafe extern "C" fn stop_handler(_arg: *mut core::ffi::c_void) {}

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real app-data handler ABI.
    #[cfg(test)]
    pub unsafe extern "C" fn app_data_handler(_data: *mut u8, _len: u32) {}

    /// # Safety
    ///
    /// Test-only no-op; callers must satisfy the real IMU callback ABI.
    #[cfg(test)]
    pub unsafe extern "C" fn imu_read_callback(
        _acc: *mut f32,
        _gyro: *mut f32,
        _mag: *mut f32,
        _dt: f32,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::{FakeAppDataBindings, FakeBindings, stubs};
    use crate::bindings::{
        AppDataBindings, CustomConfigBindings, ImuReadCallbackBindings, LbmBindings,
    };
    use crate::ffi::ImuReadCallback;
    use vescpkg_rs_sys::ExtensionHandler;

    struct OwnedTestState;

    static OWNED_TEST_STATE: crate::PackageStateStore<OwnedTestState> =
        crate::PackageStateStore::new();

    impl crate::PackageRuntimeState for OwnedTestState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &OWNED_TEST_STATE
        }
    }

    #[test]
    fn stop_package_runs_the_owned_state_stop_hook_once() {
        let mut info = crate::LoaderInfo::new();
        let mut start = super::package_start(&mut info);
        start.install_runtime_state(OwnedTestState).unwrap();
        assert!(start.finish_start(true));

        assert!(super::stop_package(&mut info));
        assert!(!OWNED_TEST_STATE.is_installed());
        assert!(!super::stop_package(&mut info));
    }

    #[test]
    fn fake_bindings_default_and_rejecting_paths() {
        let accepting = FakeBindings::default();
        let rejecting = FakeBindings::rejecting();

        unsafe {
            assert!(accepting.add_extension(
                c"ext-a".as_ptr(),
                stubs::extension_handler as ExtensionHandler
            ));
            assert!(!rejecting.add_extension(
                c"ext-b".as_ptr(),
                stubs::extension_handler as ExtensionHandler
            ));
        }

        assert_eq!(accepting.add_calls.get(), 1);
        assert_eq!(rejecting.add_calls.get(), 1);
    }

    #[test]
    fn fake_app_data_bindings_track_handler_send_and_ticks() {
        let bindings = FakeAppDataBindings::with_ticks(999);
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        assert_eq!(bindings.system_time_ticks(), 999);
        unsafe {
            assert!(bindings.set_app_data_handler(handler));
            bindings.send_app_data([1_u8, 2].as_ptr(), 2);
            assert!(bindings.clear_app_data_handler());
        }

        assert_eq!(bindings.handler_calls.get(), 2);
        assert_eq!(bindings.send_calls.get(), 1);
        assert_eq!(bindings.last_len.get(), 2);
        assert_eq!(bindings.last_handler.get(), 0);
    }

    #[test]
    fn fake_app_data_bindings_track_custom_config_registration() {
        let bindings = FakeAppDataBindings::new();

        unsafe extern "C" fn get_cfg(_data: *mut u8, _is_default: bool) -> core::ffi::c_int {
            0
        }

        unsafe extern "C" fn set_cfg(_data: *mut u8) -> bool {
            true
        }

        unsafe extern "C" fn get_cfg_xml(_data: *mut *mut u8) -> core::ffi::c_int {
            0
        }

        unsafe {
            // Float Out Boy v1.2.1 registers these three callbacks at `src/main.c:2456`;
            // the VESC function-table slots are declared in
            // `vesc_pkg_lib/vesc_c_if.h:549-553`.
            bindings.register_custom_config(get_cfg, set_cfg, get_cfg_xml);
            bindings.clear_custom_configs();
        }

        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn fake_app_data_bindings_track_imu_read_callback_registration() {
        let bindings = FakeAppDataBindings::new();

        unsafe {
            // Float Out Boy v1.2.1 registers `imu_ref_callback` at `src/main.c:2455`
            // and clears it during stop at `src/main.c:2401`.
            bindings.set_imu_read_callback(stubs::imu_read_callback as ImuReadCallback);
        }
        assert_eq!(
            bindings.last_imu_read_callback.get(),
            stubs::imu_read_callback as *const () as usize
        );
        unsafe {
            bindings.clear_imu_read_callback();
        }

        assert_eq!(bindings.imu_read_callback_calls.get(), 2);
        assert_eq!(bindings.last_imu_read_callback.get(), 0);
    }

    #[test]
    fn stub_handlers_are_callable() {
        unsafe {
            stubs::extension_handler(core::ptr::null_mut(), 0);
            stubs::stop_handler(core::ptr::null_mut());
            stubs::app_data_handler(core::ptr::null_mut(), 0);
        }
    }
}
