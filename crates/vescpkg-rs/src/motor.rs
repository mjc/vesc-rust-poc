//! Motor telemetry helpers built on firmware motor-control table slots.

#[cfg(not(test))]
use core::ffi::CStr;

#[cfg(not(test))]
use crate::types::FirmwareFaultWireCode;
use crate::types::{
    AbsoluteTachometerSteps, AmpHoursCharged, AmpHoursDischarged, AveragePower, BatteryCellCount,
    BatteryLevel, BrakeCurrent, CurrentOffDelay, DCurrent, DirectionalMotorCurrent, DutyCycle,
    DutyCycleLimit, ElectricalSpeed, FirmwareFaultCode, HandbrakeCurrent, HandbrakeRelative,
    InputCurrent, InputVoltage, MosfetTemperature, MotorCurrent, MotorCurrentLimit,
    MotorTemperature, PeakPower, TachometerSteps, TemperatureLimitStart, TotalMotorCurrent,
    TripDistance, VehicleSpeed, WattHoursCharged, WattHoursDischarged,
};
#[cfg(not(test))]
use crate::units::{Charge, Current, Distance, Energy, Power, Rpm, Speed, Temperature, Voltage};
use crate::units::{Frequency, OdometerMeters, Ratio, SignedRatio, VescSeconds};

#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MAX: core::ffi::c_int = 0;
#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MIN: core::ffi::c_int = 1;
#[cfg(not(test))]
const CFG_PARAM_L_IN_CURRENT_MAX: core::ffi::c_int = 2;
#[cfg(not(test))]
const CFG_PARAM_L_IN_CURRENT_MIN: core::ffi::c_int = 3;
#[cfg(not(test))]
const CFG_PARAM_L_TEMP_FET_START: core::ffi::c_int = 16;
#[cfg(not(test))]
const CFG_PARAM_L_TEMP_MOTOR_START: core::ffi::c_int = 18;
#[cfg(not(test))]
const CFG_PARAM_L_MAX_DUTY: core::ffi::c_int = 22;
#[cfg(not(test))]
const CFG_PARAM_SI_BATTERY_CELLS: core::ffi::c_int = 43;

/// Motor telemetry operations backed by firmware slots.
#[cfg(not(test))]
pub trait MotorTelemetryBindings {
    /// Return the current motor electrical RPM.
    ///
    /// Float Out Boy v1.2.1 reads `mc_get_rpm()` in `src/motor_data.c:108`; the VESC
    /// ABI slot is declared at `vesc_pkg_lib/vesc_c_if.h:450`.
    fn electrical_speed(&self) -> ElectricalSpeed;
    /// Return firmware-calculated vehicle speed.
    ///
    /// Float Out Boy v1.2.1 reads `mc_get_speed()` in `src/motor_data.c:118`; the
    /// VESC ABI slot is declared at `vesc_pkg_lib/vesc_c_if.h:470`.
    fn vehicle_speed(&self) -> VehicleSpeed;
    /// Return filtered total motor current.
    ///
    /// Float Out Boy v1.2.1 reads `mc_get_tot_current_filtered()` in
    /// `src/motor_data.c:120`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:456`.
    fn motor_current(&self) -> TotalMotorCurrent;
    /// Return instantaneous total motor current.
    fn motor_current_unfiltered(&self) -> TotalMotorCurrent;
    /// Return filtered motor current with the configured motor direction applied.
    fn directional_motor_current(&self) -> DirectionalMotorCurrent;
    /// Return instantaneous total motor current with motor direction applied.
    fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent;
    /// Return the configured positive motor-current limit.
    ///
    /// Float Out Boy v1.2.1 reads `CFG_PARAM_l_current_max` through `get_cfg_float`
    /// in `src/motor_data.c:91`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:243`.
    fn drive_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured braking-current magnitude.
    ///
    /// Float Out Boy v1.2.1 stores `fabsf(CFG_PARAM_l_current_min)` in
    /// `src/motor_data.c:90`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:244`.
    fn brake_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured positive battery/input-current limit.
    fn drive_input_current_limit(&self) -> InputCurrentLimit;
    /// Return the configured regenerative battery/input-current limit magnitude.
    fn brake_input_current_limit(&self) -> InputCurrentLimit;
    /// Return the configured MOSFET temperature limit-start threshold.
    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured motor temperature limit-start threshold.
    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured maximum duty-cycle limit.
    ///
    /// Float Out Boy v1.2.1 reads `CFG_PARAM_l_max_duty` through `get_cfg_float`
    /// in `src/motor_data.c:97`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:271` and has ABI value 22.
    fn duty_cycle_limit(&self) -> DutyCycleLimit;
    /// Return the configured battery cell count, when positive and representable.
    fn battery_cell_count(&self) -> Option<BatteryCellCount>;
    /// Return filtered input/battery current.
    ///
    /// Float Out Boy v1.2.1 reads `mc_get_tot_current_in_filtered()` in
    /// `src/motor_data.c:140`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:460`.
    fn battery_current(&self) -> InputCurrent;
    /// Return instantaneous input/battery current.
    fn battery_current_unfiltered(&self) -> InputCurrent;
    /// Return average motor power statistics.
    fn average_power(&self) -> AveragePower;
    /// Return peak motor power statistics.
    fn peak_power(&self) -> PeakPower;
    /// Return the current signed duty cycle.
    ///
    /// The firmware value is clamped to the signed ratio range, with NaN
    /// normalized to zero.
    ///
    /// VESC applies motor direction in `mc_interface_get_duty_cycle_now`; the
    /// ABI slot is declared at `vesc_pkg_lib/vesc_c_if.h:448`.
    fn duty_cycle(&self) -> DutyCycle;
    /// Return optional FOC d-axis Id current.
    ///
    /// Float Out Boy v1.2.1 reads optional `foc_get_id` while encoding compact
    /// all-data at `src/main.c:1364-1368`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:616`.
    fn d_axis_current(&self) -> Option<DCurrent>;
    /// Return the absolute distance travelled by the motor/vehicle.
    fn trip_distance(&self) -> TripDistance;
    /// Return the filtered MOSFET/FET temperature.
    fn mosfet_temperature(&self) -> MosfetTemperature;
    /// Return the filtered motor temperature.
    fn motor_temperature(&self) -> MotorTemperature;
    /// Return the stored odometer distance.
    fn odometer(&self) -> OdometerMeters;
    /// Return discharged amp-hours.
    fn amp_hours_discharged(&self) -> AmpHoursDischarged;
    /// Return charged amp-hours.
    fn amp_hours_charged(&self) -> AmpHoursCharged;
    /// Return discharged watt-hours.
    fn watt_hours_discharged(&self) -> WattHoursDischarged;
    /// Return charged watt-hours.
    fn watt_hours_charged(&self) -> WattHoursCharged;
    /// Return estimated battery level.
    fn battery_level(&self) -> BatteryLevel;
    /// Return the active firmware motor fault code.
    fn firmware_fault(&self) -> FirmwareFaultCode;
    /// Return the firmware-owned display name for a motor fault code.
    fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]>;
    /// Return the filtered controller input voltage.
    fn input_voltage(&self) -> InputVoltage;
    /// Return the relative motor tachometer, optionally resetting it.
    fn tachometer(&self, reset: bool) -> TachometerSteps;
    /// Return the absolute motor tachometer, optionally resetting it.
    fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps;
    /// Return the current motor-control sampling frequency.
    fn sampling_frequency(&self) -> Frequency;
}

#[cfg(not(test))]
impl<B: MotorTelemetryBindings + ?Sized> MotorTelemetryBindings for &B {
    fn electrical_speed(&self) -> ElectricalSpeed {
        (**self).electrical_speed()
    }

    fn vehicle_speed(&self) -> VehicleSpeed {
        (**self).vehicle_speed()
    }

    fn motor_current(&self) -> TotalMotorCurrent {
        (**self).motor_current()
    }

    fn motor_current_unfiltered(&self) -> TotalMotorCurrent {
        (**self).motor_current_unfiltered()
    }

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        (**self).directional_motor_current()
    }

    fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent {
        (**self).directional_motor_current_unfiltered()
    }

    fn drive_current_limit(&self) -> MotorCurrentLimit {
        (**self).drive_current_limit()
    }

    fn brake_current_limit(&self) -> MotorCurrentLimit {
        (**self).brake_current_limit()
    }

    fn drive_input_current_limit(&self) -> InputCurrentLimit {
        (**self).drive_input_current_limit()
    }

    fn brake_input_current_limit(&self) -> InputCurrentLimit {
        (**self).brake_input_current_limit()
    }

    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart {
        (**self).mosfet_temperature_limit_start()
    }

    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart {
        (**self).motor_temperature_limit_start()
    }

    fn duty_cycle_limit(&self) -> DutyCycleLimit {
        (**self).duty_cycle_limit()
    }

    fn battery_cell_count(&self) -> Option<BatteryCellCount> {
        (**self).battery_cell_count()
    }

    fn battery_current(&self) -> InputCurrent {
        (**self).battery_current()
    }

    fn battery_current_unfiltered(&self) -> InputCurrent {
        (**self).battery_current_unfiltered()
    }

    fn average_power(&self) -> AveragePower {
        (**self).average_power()
    }

    fn peak_power(&self) -> PeakPower {
        (**self).peak_power()
    }

    fn duty_cycle(&self) -> DutyCycle {
        (**self).duty_cycle()
    }

    fn d_axis_current(&self) -> Option<DCurrent> {
        (**self).d_axis_current()
    }

    fn trip_distance(&self) -> TripDistance {
        (**self).trip_distance()
    }

    fn mosfet_temperature(&self) -> MosfetTemperature {
        (**self).mosfet_temperature()
    }

    fn motor_temperature(&self) -> MotorTemperature {
        (**self).motor_temperature()
    }

    fn odometer(&self) -> OdometerMeters {
        (**self).odometer()
    }

    fn amp_hours_discharged(&self) -> AmpHoursDischarged {
        (**self).amp_hours_discharged()
    }

    fn amp_hours_charged(&self) -> AmpHoursCharged {
        (**self).amp_hours_charged()
    }

    fn watt_hours_discharged(&self) -> WattHoursDischarged {
        (**self).watt_hours_discharged()
    }

    fn watt_hours_charged(&self) -> WattHoursCharged {
        (**self).watt_hours_charged()
    }

    fn battery_level(&self) -> BatteryLevel {
        (**self).battery_level()
    }

    fn firmware_fault(&self) -> FirmwareFaultCode {
        (**self).firmware_fault()
    }

    fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]> {
        (**self).firmware_fault_name(fault)
    }

    fn input_voltage(&self) -> InputVoltage {
        (**self).input_voltage()
    }

    fn tachometer(&self, reset: bool) -> TachometerSteps {
        (**self).tachometer(reset)
    }

    fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps {
        (**self).absolute_tachometer(reset)
    }

    fn sampling_frequency(&self) -> Frequency {
        (**self).sampling_frequency()
    }
}

/// Motor-control operations backed by firmware slots.
#[cfg(not(test))]
pub trait MotorControlBindings {
    /// Reset the firmware motor-command safety timeout.
    ///
    /// Float Out Boy v1.2.1 calls this before motor-control output at
    /// `third_party/float-out-boy/src/motor_control.c:92-93`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:538`.
    fn timeout_reset(&self);
    /// Keep current control enabled after a current command.
    ///
    /// Float Out Boy v1.2.1 sets `0.05f` seconds before sending requested current at
    /// `third_party/float-out-boy/src/motor_control.c:96-99`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:476`.
    fn set_current_off_delay(&self, delay: CurrentOffDelay);
    /// Set motor current in amps.
    ///
    /// Float Out Boy v1.2.1 sends the requested current at
    /// `third_party/float-out-boy/src/motor_control.c:99`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:440`.
    fn set_current(&self, current: MotorCurrent);
    /// Set motor duty cycle.
    ///
    /// Float Out Boy v1.2.1 sends parking-brake duty zero at
    /// `third_party/float-out-boy/src/motor_control.c:112-114`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
    fn set_duty_cycle(&self, duty: DutyCycle);
    /// Set motor brake current in amps.
    ///
    /// Float Out Boy v1.2.1 sends idle brake current at
    /// `third_party/float-out-boy/src/motor_control.c:115-117`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
    fn set_brake_current(&self, current: BrakeCurrent);
    /// Set motor handbrake current in amps.
    fn set_handbrake(&self, current: HandbrakeCurrent);
    /// Set motor handbrake as a relative command.
    fn set_handbrake_relative(&self, current: HandbrakeRelative);
    /// Reset accumulated motor statistics.
    fn reset_statistics(&self);
    /// Release the motor output.
    fn release_motor(&self);
    /// Wait up to `timeout` for the motor output to be released.
    fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool;
}

#[cfg(not(test))]
impl<B: MotorControlBindings + ?Sized> MotorControlBindings for &B {
    fn timeout_reset(&self) {
        (**self).timeout_reset();
    }

    fn set_current_off_delay(&self, delay: CurrentOffDelay) {
        (**self).set_current_off_delay(delay);
    }

    fn set_current(&self, current: MotorCurrent) {
        (**self).set_current(current);
    }

    fn set_duty_cycle(&self, duty: DutyCycle) {
        (**self).set_duty_cycle(duty);
    }

    fn set_brake_current(&self, current: BrakeCurrent) {
        (**self).set_brake_current(current);
    }

    fn set_handbrake(&self, current: HandbrakeCurrent) {
        (**self).set_handbrake(current);
    }

    fn set_handbrake_relative(&self, current: HandbrakeRelative) {
        (**self).set_handbrake_relative(current);
    }

    fn reset_statistics(&self) {
        (**self).reset_statistics();
    }

    fn release_motor(&self) {
        (**self).release_motor();
    }

    fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool {
        (**self).wait_for_motor_release(timeout)
    }
}

#[cfg(not(test))]
/// Motor telemetry binding implementation that forwards to the live firmware ABI.
pub struct RealMotorTelemetryBindings;

#[cfg(not(test))]
/// Motor-control binding implementation that forwards to the live firmware ABI.
pub struct RealMotorControlBindings;

#[cfg(not(test))]
impl MotorTelemetryBindings for RealMotorTelemetryBindings {
    fn electrical_speed(&self) -> ElectricalSpeed {
        ElectricalSpeed::new(Rpm::from_revolutions_per_minute(unsafe {
            crate::ffi::mc_get_rpm()
        }))
    }

    fn vehicle_speed(&self) -> VehicleSpeed {
        VehicleSpeed::new(Speed::from_meters_per_second(unsafe {
            crate::ffi::mc_get_speed()
        }))
    }

    fn motor_current(&self) -> TotalMotorCurrent {
        TotalMotorCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_filtered()
        }))
    }

    fn motor_current_unfiltered(&self) -> TotalMotorCurrent {
        TotalMotorCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current()
        }))
    }

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        DirectionalMotorCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_directional_filtered()
        }))
    }

    fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent {
        DirectionalMotorCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_directional()
        }))
    }

    fn drive_current_limit(&self) -> MotorCurrentLimit {
        MotorCurrentLimit::from_positive_current(Current::from_amps(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_CURRENT_MAX)
        }))
    }

    fn brake_current_limit(&self) -> MotorCurrentLimit {
        MotorCurrentLimit::new(Current::from_amps(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_CURRENT_MIN)
        }))
    }

    fn drive_input_current_limit(&self) -> InputCurrentLimit {
        InputCurrentLimit::new(Current::from_amps(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_IN_CURRENT_MAX)
        }))
    }

    fn brake_input_current_limit(&self) -> InputCurrentLimit {
        InputCurrentLimit::new(Current::from_amps(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_IN_CURRENT_MIN)
        }))
    }

    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart {
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_TEMP_FET_START)
        }))
    }

    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart {
        TemperatureLimitStart::new(Temperature::from_degrees_celsius(unsafe {
            crate::ffi::get_cfg_float(CFG_PARAM_L_TEMP_MOTOR_START)
        }))
    }

    fn duty_cycle_limit(&self) -> DutyCycleLimit {
        duty_cycle_limit_from_firmware(unsafe { crate::ffi::get_cfg_float(CFG_PARAM_L_MAX_DUTY) })
    }

    fn battery_cell_count(&self) -> Option<BatteryCellCount> {
        battery_cell_count_from_firmware(unsafe {
            crate::ffi::get_cfg_int(CFG_PARAM_SI_BATTERY_CELLS)
        })
    }

    fn battery_current(&self) -> InputCurrent {
        InputCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_in_filtered()
        }))
    }

    fn battery_current_unfiltered(&self) -> InputCurrent {
        InputCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_in()
        }))
    }

    fn average_power(&self) -> AveragePower {
        AveragePower::new(Power::from_watts(unsafe {
            crate::ffi::mc_stat_power_avg()
        }))
    }

    fn peak_power(&self) -> PeakPower {
        PeakPower::new(Power::from_watts(unsafe {
            crate::ffi::mc_stat_power_max()
        }))
    }

    fn duty_cycle(&self) -> DutyCycle {
        duty_cycle_from_firmware(unsafe { crate::ffi::mc_get_duty_cycle_now() })
    }

    fn d_axis_current(&self) -> Option<DCurrent> {
        unsafe { crate::ffi::foc_get_id() }.map(|amps| DCurrent::new(Current::from_amps(amps)))
    }

    fn trip_distance(&self) -> TripDistance {
        TripDistance::new(Distance::from_meters(unsafe {
            crate::ffi::mc_get_distance_abs()
        }))
    }

    fn mosfet_temperature(&self) -> MosfetTemperature {
        MosfetTemperature::new(Temperature::from_degrees_celsius(unsafe {
            crate::ffi::mc_temp_fet_filtered()
        }))
    }

    fn motor_temperature(&self) -> MotorTemperature {
        MotorTemperature::new(Temperature::from_degrees_celsius(unsafe {
            crate::ffi::mc_temp_motor_filtered()
        }))
    }

    fn odometer(&self) -> OdometerMeters {
        OdometerMeters::from_meters(unsafe { crate::ffi::mc_get_odometer() })
    }

    fn amp_hours_discharged(&self) -> AmpHoursDischarged {
        AmpHoursDischarged::new(Charge::from_amp_hours(unsafe {
            crate::ffi::mc_get_amp_hours(false)
        }))
    }

    fn amp_hours_charged(&self) -> AmpHoursCharged {
        AmpHoursCharged::new(Charge::from_amp_hours(unsafe {
            crate::ffi::mc_get_amp_hours_charged(false)
        }))
    }

    fn watt_hours_discharged(&self) -> WattHoursDischarged {
        WattHoursDischarged::new(Energy::from_watt_hours(unsafe {
            crate::ffi::mc_get_watt_hours(false)
        }))
    }

    fn watt_hours_charged(&self) -> WattHoursCharged {
        WattHoursCharged::new(Energy::from_watt_hours(unsafe {
            crate::ffi::mc_get_watt_hours_charged(false)
        }))
    }

    fn battery_level(&self) -> BatteryLevel {
        BatteryLevel::from_fraction(unsafe {
            crate::ffi::mc_get_battery_level(core::ptr::null_mut())
        })
    }

    fn firmware_fault(&self) -> FirmwareFaultCode {
        FirmwareFaultCode::from_raw_code(unsafe { crate::ffi::mc_get_fault() })
    }

    fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]> {
        let code = FirmwareFaultWireCode::try_from(fault).ok()?.wire_code();
        let pointer = unsafe { crate::ffi::mc_fault_to_string(u32::from(code)) };
        #[cfg(all(feature = "test-support", not(target_arch = "arm")))]
        let pointer = Some(pointer);
        // SAFETY: VESC returns a null-terminated string in firmware-owned static storage.
        let bytes = unsafe { CStr::from_ptr(pointer?).to_bytes() };
        Some(bytes.strip_prefix(b"FAULT_CODE_").unwrap_or(bytes))
    }

    fn input_voltage(&self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(unsafe {
            crate::ffi::mc_get_input_voltage_filtered()
        }))
    }

    fn tachometer(&self, reset: bool) -> TachometerSteps {
        TachometerSteps::new(crate::units::TachometerSteps::from_steps(unsafe {
            crate::ffi::mc_get_tachometer_value(reset)
        }))
    }

    fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps {
        AbsoluteTachometerSteps::new(crate::units::TachometerSteps::from_steps(unsafe {
            crate::ffi::mc_get_tachometer_abs_value(reset)
        }))
    }

    fn sampling_frequency(&self) -> Frequency {
        Frequency::from_hertz(unsafe { crate::ffi::mc_get_sampling_frequency_now() })
    }
}

#[cfg(not(test))]
impl MotorControlBindings for RealMotorControlBindings {
    fn timeout_reset(&self) {
        unsafe { crate::ffi::timeout_reset() };
    }

    fn set_current_off_delay(&self, delay: CurrentOffDelay) {
        unsafe { crate::ffi::mc_set_current_off_delay(delay.duration().as_seconds()) };
    }

    fn set_current(&self, current: MotorCurrent) {
        unsafe { crate::ffi::mc_set_current(current.current().as_amps()) };
    }

    fn set_duty_cycle(&self, duty: DutyCycle) {
        unsafe { crate::ffi::mc_set_duty(duty.ratio().as_ratio()) };
    }

    fn set_brake_current(&self, current: BrakeCurrent) {
        unsafe { crate::ffi::mc_set_brake_current(current.current().as_amps()) };
    }

    fn set_handbrake(&self, current: HandbrakeCurrent) {
        unsafe { crate::ffi::mc_set_handbrake(current.current().as_amps()) };
    }

    fn set_handbrake_relative(&self, current: HandbrakeRelative) {
        unsafe { crate::ffi::mc_set_handbrake_rel(current.ratio().as_ratio()) };
    }

    fn reset_statistics(&self) {
        unsafe { crate::ffi::mc_stat_reset() };
    }

    fn release_motor(&self) {
        unsafe { crate::ffi::mc_release_motor() };
    }

    fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool {
        unsafe { crate::ffi::mc_wait_for_motor_release(timeout.as_seconds()) }
    }
}

fn duty_cycle_from_firmware(raw_duty: f32) -> DutyCycle {
    DutyCycle::new(SignedRatio::clamped(if raw_duty.is_nan() {
        0.0
    } else {
        raw_duty
    }))
}

fn duty_cycle_limit_from_firmware(raw_limit: f32) -> DutyCycleLimit {
    DutyCycleLimit::new(Ratio::clamped(if raw_limit.is_nan() {
        0.0
    } else {
        raw_limit
    }))
}

fn battery_cell_count_from_firmware(raw_count: i32) -> Option<BatteryCellCount> {
    u16::try_from(raw_count)
        .ok()
        .and_then(|count| BatteryCellCount::try_new(count).ok())
}

/// High-level motor telemetry API built on a binding implementation.
#[cfg(not(test))]
pub struct MotorTelemetryApi<B> {
    bindings: B,
}

mod private {
    pub trait MotorTelemetry {}
    pub trait MotorOutput {}
}

/// Semantic motor telemetry capability used by package code.
pub trait MotorTelemetry: private::MotorTelemetry {
    /// Return the current motor electrical speed.
    fn electrical_speed(&self) -> ElectricalSpeed;
    /// Return firmware-calculated vehicle speed.
    fn vehicle_speed(&self) -> VehicleSpeed;
    /// Return filtered total motor current.
    fn motor_current(&self) -> TotalMotorCurrent;
    /// Return instantaneous total motor current.
    fn motor_current_unfiltered(&self) -> TotalMotorCurrent;
    /// Return filtered motor current with the configured motor direction applied.
    fn directional_motor_current(&self) -> DirectionalMotorCurrent;
    /// Return instantaneous total motor current with motor direction applied.
    fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent;
    /// Return the configured positive motor-current limit.
    fn drive_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured braking-current magnitude.
    fn brake_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured positive battery/input-current limit.
    fn drive_input_current_limit(&self) -> InputCurrentLimit;
    /// Return the configured regenerative battery/input-current limit magnitude.
    fn brake_input_current_limit(&self) -> InputCurrentLimit;
    /// Return the configured MOSFET temperature limit-start threshold.
    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured motor temperature limit-start threshold.
    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured maximum duty-cycle limit.
    fn duty_cycle_limit(&self) -> DutyCycleLimit;
    /// Return the configured battery cell count, when available.
    fn battery_cell_count(&self) -> Option<BatteryCellCount>;
    /// Return filtered input/battery current.
    fn battery_current(&self) -> InputCurrent;
    /// Return instantaneous input/battery current.
    fn battery_current_unfiltered(&self) -> InputCurrent;
    /// Return average motor power statistics.
    fn average_power(&self) -> AveragePower;
    /// Return peak motor power statistics.
    fn peak_power(&self) -> PeakPower;
    /// Return the current signed duty cycle.
    fn duty_cycle(&self) -> DutyCycle;
    /// Return optional FOC d-axis current.
    fn d_axis_current(&self) -> Option<DCurrent>;
    /// Return the absolute distance travelled by the motor/vehicle.
    fn trip_distance(&self) -> TripDistance;
    /// Return the filtered MOSFET/FET temperature.
    fn mosfet_temperature(&self) -> MosfetTemperature;
    /// Return the filtered motor temperature.
    fn motor_temperature(&self) -> MotorTemperature;
    /// Return the stored odometer distance.
    fn odometer(&self) -> OdometerMeters;
    /// Return discharged amp-hours.
    fn amp_hours_discharged(&self) -> AmpHoursDischarged;
    /// Return charged amp-hours.
    fn amp_hours_charged(&self) -> AmpHoursCharged;
    /// Return discharged watt-hours.
    fn watt_hours_discharged(&self) -> WattHoursDischarged;
    /// Return charged watt-hours.
    fn watt_hours_charged(&self) -> WattHoursCharged;
    /// Return estimated battery level.
    fn battery_level(&self) -> BatteryLevel;
    /// Return the active firmware motor fault code.
    fn firmware_fault(&self) -> FirmwareFaultCode;
    /// Return the firmware display name for a motor fault code without its `FAULT_CODE_` prefix.
    fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]>;
    /// Return the filtered controller input voltage.
    fn input_voltage(&self) -> InputVoltage;
    /// Return the relative motor tachometer, optionally resetting it.
    fn tachometer(&self, reset: bool) -> TachometerSteps;
    /// Return the absolute motor tachometer, optionally resetting it.
    fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps;
    /// Return the current motor-control sampling frequency.
    fn sampling_frequency(&self) -> Frequency;
}

/// Semantic motor-output capability used by package code.
pub trait MotorOutput: private::MotorOutput {
    /// Keep the controller's motor command watchdog alive.
    fn keep_alive(&self);

    /// Set the delay used when the controller turns current off.
    fn set_current_off_delay(&self, delay: CurrentOffDelay);

    /// Apply a signed motor-current command.
    fn set_current(&self, current: MotorCurrent);

    /// Apply a duty-cycle command.
    fn set_duty_cycle(&self, duty: DutyCycle);

    /// Apply a braking-current command.
    fn set_brake_current(&self, current: BrakeCurrent);
    /// Apply a handbrake-current command.
    fn set_handbrake(&self, current: HandbrakeCurrent);
    /// Apply a relative handbrake command.
    fn set_handbrake_relative(&self, current: HandbrakeRelative);
    /// Reset accumulated motor statistics.
    fn reset_statistics(&self);
    /// Release the motor output.
    fn release_motor(&self);
    /// Wait up to `timeout` for the motor output to be released.
    fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool;
}

/// High-level motor-control API built on a binding implementation.
#[cfg(not(test))]
pub struct MotorControlApi<B> {
    bindings: B,
}

#[cfg(test)]
mod tests {
    use super::{
        battery_cell_count_from_firmware, duty_cycle_from_firmware, duty_cycle_limit_from_firmware,
    };
    use crate::{DutyCycle, Ratio, SignedRatio};

    #[test]
    fn duty_cycle_preserves_direction_and_normalizes_invalid_values() {
        assert_eq!(duty_cycle_from_firmware(f32::NAN).ratio().as_ratio(), 0.0);
        assert_eq!(duty_cycle_from_firmware(-0.42).ratio().as_ratio(), -0.42);
        assert_eq!(duty_cycle_from_firmware(2.0).ratio().as_ratio(), 1.0);
    }

    #[test]
    fn battery_cell_count_accepts_only_positive_u16_firmware_values() {
        assert_eq!(battery_cell_count_from_firmware(-1), None);
        assert_eq!(battery_cell_count_from_firmware(0), None);
        assert_eq!(battery_cell_count_from_firmware(65_536), None);
        assert_eq!(
            battery_cell_count_from_firmware(18),
            Some(crate::BatteryCellCount::try_new(18).expect("positive count")),
        );
    }

    #[test]
    fn duty_cycle_limit_normalizes_firmware_config_at_the_boundary() {
        assert_eq!(
            duty_cycle_limit_from_firmware(f32::NAN).ratio().as_ratio(),
            0.0
        );
        assert_eq!(
            duty_cycle_limit_from_firmware(0.95).ratio().as_ratio(),
            0.95
        );
        assert_eq!(duty_cycle_limit_from_firmware(2.0).ratio().as_ratio(), 1.0);
        assert_eq!(
            duty_cycle_limit_from_firmware(0.95)
                .reduced_by(Ratio::from_ratio_const(0.05))
                .ratio()
                .as_ratio(),
            0.9
        );
        assert_eq!(
            DutyCycle::new(SignedRatio::from_ratio_const(-0.85))
                .magnitude()
                .as_ratio(),
            0.85
        );
    }
}

#[cfg(not(test))]
impl<B: MotorTelemetryBindings> MotorTelemetryApi<B> {
    /// Construct a new motor telemetry API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the current motor electrical RPM.
    pub fn electrical_speed(&self) -> ElectricalSpeed {
        self.bindings.electrical_speed()
    }

    /// Return firmware-calculated vehicle speed.
    pub fn vehicle_speed(&self) -> VehicleSpeed {
        self.bindings.vehicle_speed()
    }

    /// Return filtered total motor current.
    pub fn motor_current(&self) -> TotalMotorCurrent {
        self.bindings.motor_current()
    }

    /// Return instantaneous total motor current.
    pub fn motor_current_unfiltered(&self) -> TotalMotorCurrent {
        self.bindings.motor_current_unfiltered()
    }

    /// Return filtered motor current with the configured motor direction applied.
    pub fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        self.bindings.directional_motor_current()
    }

    /// Return instantaneous total motor current with motor direction applied.
    pub fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent {
        self.bindings.directional_motor_current_unfiltered()
    }

    /// Return the configured positive motor-current limit.
    pub fn drive_current_limit(&self) -> MotorCurrentLimit {
        self.bindings.drive_current_limit()
    }

    /// Return the configured braking-current magnitude.
    pub fn brake_current_limit(&self) -> MotorCurrentLimit {
        self.bindings.brake_current_limit()
    }

    /// Return the configured positive battery/input-current limit.
    pub fn drive_input_current_limit(&self) -> InputCurrentLimit {
        self.bindings.drive_input_current_limit()
    }

    /// Return the configured regenerative battery/input-current limit magnitude.
    pub fn brake_input_current_limit(&self) -> InputCurrentLimit {
        self.bindings.brake_input_current_limit()
    }

    /// Return the configured MOSFET temperature limit-start threshold.
    pub fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart {
        self.bindings.mosfet_temperature_limit_start()
    }

    /// Return the configured motor temperature limit-start threshold.
    pub fn motor_temperature_limit_start(&self) -> TemperatureLimitStart {
        self.bindings.motor_temperature_limit_start()
    }

    /// Return the configured maximum duty-cycle limit.
    pub fn duty_cycle_limit(&self) -> DutyCycleLimit {
        self.bindings.duty_cycle_limit()
    }

    /// Return the configured battery cell count, when available.
    pub fn battery_cell_count(&self) -> Option<BatteryCellCount> {
        self.bindings.battery_cell_count()
    }

    /// Return filtered input/battery current.
    pub fn battery_current(&self) -> InputCurrent {
        self.bindings.battery_current()
    }

    /// Return instantaneous input/battery current.
    pub fn battery_current_unfiltered(&self) -> InputCurrent {
        self.bindings.battery_current_unfiltered()
    }

    /// Return average motor power statistics.
    pub fn average_power(&self) -> AveragePower {
        self.bindings.average_power()
    }

    /// Return peak motor power statistics.
    pub fn peak_power(&self) -> PeakPower {
        self.bindings.peak_power()
    }

    /// Return the current signed duty cycle.
    pub fn duty_cycle(&self) -> DutyCycle {
        self.bindings.duty_cycle()
    }

    /// Return optional FOC d-axis Id current.
    pub fn d_axis_current(&self) -> Option<DCurrent> {
        self.bindings.d_axis_current()
    }

    /// Return the absolute distance travelled by the motor/vehicle.
    pub fn trip_distance(&self) -> TripDistance {
        self.bindings.trip_distance()
    }

    /// Return the filtered MOSFET/FET temperature.
    pub fn mosfet_temperature(&self) -> MosfetTemperature {
        self.bindings.mosfet_temperature()
    }

    /// Return the filtered motor temperature.
    pub fn motor_temperature(&self) -> MotorTemperature {
        self.bindings.motor_temperature()
    }

    /// Return the stored odometer distance.
    pub fn odometer(&self) -> OdometerMeters {
        self.bindings.odometer()
    }

    /// Return discharged amp-hours.
    pub fn amp_hours_discharged(&self) -> AmpHoursDischarged {
        self.bindings.amp_hours_discharged()
    }

    /// Return charged amp-hours.
    pub fn amp_hours_charged(&self) -> AmpHoursCharged {
        self.bindings.amp_hours_charged()
    }

    /// Return discharged watt-hours.
    pub fn watt_hours_discharged(&self) -> WattHoursDischarged {
        self.bindings.watt_hours_discharged()
    }

    /// Return charged watt-hours.
    pub fn watt_hours_charged(&self) -> WattHoursCharged {
        self.bindings.watt_hours_charged()
    }

    /// Return estimated battery level.
    pub fn battery_level(&self) -> BatteryLevel {
        self.bindings.battery_level()
    }

    /// Return the active firmware motor fault code.
    pub fn firmware_fault(&self) -> FirmwareFaultCode {
        self.bindings.firmware_fault()
    }

    /// Return the firmware display name for a motor fault code.
    pub fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]> {
        self.bindings.firmware_fault_name(fault)
    }

    /// Return the filtered controller input voltage.
    pub fn input_voltage(&self) -> InputVoltage {
        self.bindings.input_voltage()
    }

    /// Return the relative motor tachometer, optionally resetting it.
    pub fn tachometer(&self, reset: bool) -> TachometerSteps {
        self.bindings.tachometer(reset)
    }

    /// Return the absolute motor tachometer, optionally resetting it.
    pub fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps {
        self.bindings.absolute_tachometer(reset)
    }

    /// Return the current motor-control sampling frequency.
    pub fn sampling_frequency(&self) -> Frequency {
        self.bindings.sampling_frequency()
    }
}

#[cfg(not(test))]
impl<B: MotorTelemetryBindings> private::MotorTelemetry for MotorTelemetryApi<B> {}

#[cfg(not(test))]
impl<B: MotorTelemetryBindings> MotorTelemetry for MotorTelemetryApi<B> {
    fn electrical_speed(&self) -> ElectricalSpeed {
        self.electrical_speed()
    }

    fn vehicle_speed(&self) -> VehicleSpeed {
        self.vehicle_speed()
    }

    fn motor_current(&self) -> TotalMotorCurrent {
        self.motor_current()
    }

    fn motor_current_unfiltered(&self) -> TotalMotorCurrent {
        self.motor_current_unfiltered()
    }

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        self.directional_motor_current()
    }

    fn directional_motor_current_unfiltered(&self) -> DirectionalMotorCurrent {
        self.directional_motor_current_unfiltered()
    }

    fn drive_current_limit(&self) -> MotorCurrentLimit {
        self.drive_current_limit()
    }

    fn brake_current_limit(&self) -> MotorCurrentLimit {
        self.brake_current_limit()
    }

    fn drive_input_current_limit(&self) -> InputCurrentLimit {
        self.drive_input_current_limit()
    }

    fn brake_input_current_limit(&self) -> InputCurrentLimit {
        self.brake_input_current_limit()
    }

    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart {
        self.mosfet_temperature_limit_start()
    }

    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart {
        self.motor_temperature_limit_start()
    }

    fn duty_cycle_limit(&self) -> DutyCycleLimit {
        self.duty_cycle_limit()
    }

    fn battery_cell_count(&self) -> Option<BatteryCellCount> {
        self.battery_cell_count()
    }

    fn battery_current(&self) -> InputCurrent {
        self.battery_current()
    }

    fn battery_current_unfiltered(&self) -> InputCurrent {
        self.battery_current_unfiltered()
    }

    fn average_power(&self) -> AveragePower {
        self.average_power()
    }

    fn peak_power(&self) -> PeakPower {
        self.peak_power()
    }

    fn duty_cycle(&self) -> DutyCycle {
        self.duty_cycle()
    }

    fn d_axis_current(&self) -> Option<DCurrent> {
        self.d_axis_current()
    }

    fn trip_distance(&self) -> TripDistance {
        self.trip_distance()
    }

    fn mosfet_temperature(&self) -> MosfetTemperature {
        self.mosfet_temperature()
    }

    fn motor_temperature(&self) -> MotorTemperature {
        self.motor_temperature()
    }

    fn odometer(&self) -> OdometerMeters {
        self.odometer()
    }

    fn amp_hours_discharged(&self) -> AmpHoursDischarged {
        self.amp_hours_discharged()
    }

    fn amp_hours_charged(&self) -> AmpHoursCharged {
        self.amp_hours_charged()
    }

    fn watt_hours_discharged(&self) -> WattHoursDischarged {
        self.watt_hours_discharged()
    }

    fn watt_hours_charged(&self) -> WattHoursCharged {
        self.watt_hours_charged()
    }

    fn battery_level(&self) -> BatteryLevel {
        self.battery_level()
    }

    fn firmware_fault(&self) -> FirmwareFaultCode {
        self.firmware_fault()
    }

    fn firmware_fault_name(&self, fault: FirmwareFaultCode) -> Option<&'static [u8]> {
        self.firmware_fault_name(fault)
    }

    fn input_voltage(&self) -> InputVoltage {
        self.input_voltage()
    }

    fn tachometer(&self, reset: bool) -> TachometerSteps {
        self.tachometer(reset)
    }

    fn absolute_tachometer(&self, reset: bool) -> AbsoluteTachometerSteps {
        self.absolute_tachometer(reset)
    }

    fn sampling_frequency(&self) -> Frequency {
        self.sampling_frequency()
    }
}

#[cfg(not(test))]
impl<B: MotorControlBindings> MotorControlApi<B> {
    pub(crate) fn from_firmware(bindings: B) -> Self {
        Self { bindings }
    }

    /// Reset the firmware motor-command safety timeout.
    pub fn timeout_reset(&self) {
        self.bindings.timeout_reset();
    }

    /// Keep current control enabled after a current command.
    pub fn set_current_off_delay(&self, delay: CurrentOffDelay) {
        self.bindings.set_current_off_delay(delay);
    }

    /// Set motor current.
    pub fn set_current(&self, current: MotorCurrent) {
        self.bindings.set_current(current);
    }

    /// Set motor duty cycle.
    ///
    /// Float Out Boy uses this for parking brake duty zero at
    /// `third_party/float-out-boy/src/motor_control.c:112-114`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
    pub fn set_duty_cycle(&self, duty: DutyCycle) {
        self.bindings.set_duty_cycle(duty);
    }

    /// Set motor brake current.
    ///
    /// Float Out Boy uses this for idle brake current at
    /// `third_party/float-out-boy/src/motor_control.c:115-117`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
    pub fn set_brake_current(&self, current: BrakeCurrent) {
        self.bindings.set_brake_current(current);
    }

    /// Set motor handbrake current.
    pub fn set_handbrake(&self, current: HandbrakeCurrent) {
        self.bindings.set_handbrake(current);
    }

    /// Set motor handbrake as a relative command.
    pub fn set_handbrake_relative(&self, current: HandbrakeRelative) {
        self.bindings.set_handbrake_relative(current);
    }

    /// Reset accumulated motor statistics.
    pub fn reset_statistics(&self) {
        self.bindings.reset_statistics();
    }

    /// Release the motor output.
    pub fn release_motor(&self) {
        self.bindings.release_motor();
    }

    /// Wait until the motor output has been released.
    pub fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool {
        self.bindings.wait_for_motor_release(timeout)
    }
}

#[cfg(not(test))]
impl<B: MotorControlBindings> private::MotorOutput for MotorControlApi<B> {}

#[cfg(not(test))]
impl<B: MotorControlBindings> MotorOutput for MotorControlApi<B> {
    fn keep_alive(&self) {
        self.timeout_reset();
    }

    fn set_current_off_delay(&self, delay: CurrentOffDelay) {
        MotorControlApi::set_current_off_delay(self, delay);
    }

    fn set_current(&self, current: MotorCurrent) {
        MotorControlApi::set_current(self, current);
    }

    fn set_duty_cycle(&self, duty: DutyCycle) {
        MotorControlApi::set_duty_cycle(self, duty);
    }

    fn set_brake_current(&self, current: BrakeCurrent) {
        MotorControlApi::set_brake_current(self, current);
    }

    fn set_handbrake(&self, current: HandbrakeCurrent) {
        MotorControlApi::set_handbrake(self, current);
    }

    fn set_handbrake_relative(&self, current: HandbrakeRelative) {
        MotorControlApi::set_handbrake_relative(self, current);
    }

    fn reset_statistics(&self) {
        MotorControlApi::reset_statistics(self);
    }

    fn release_motor(&self) {
        MotorControlApi::release_motor(self);
    }

    fn wait_for_motor_release(&self, timeout: VescSeconds) -> bool {
        MotorControlApi::wait_for_motor_release(self, timeout)
    }
}
