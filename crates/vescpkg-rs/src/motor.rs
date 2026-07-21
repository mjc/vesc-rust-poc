//! Motor telemetry helpers built on firmware motor-control table slots.

use crate::types::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCellCount, BatteryLevel, BrakeCurrent,
    CurrentOffDelay, DCurrent, DirectionalMotorCurrent, DutyCycle, DutyCycleLimit, ElectricalSpeed,
    FirmwareFaultCode, InputCurrent, InputVoltage, MosfetTemperature, MotorCurrent,
    MotorCurrentLimit, MotorTemperature, TemperatureLimitStart, TotalMotorCurrent, TripDistance,
    VehicleSpeed, WattHoursCharged, WattHoursDischarged,
};
#[cfg(not(test))]
use crate::units::{Charge, Current, Distance, Energy, Rpm, Speed, Temperature, Voltage};
use crate::units::{OdometerMeters, Ratio, SignedRatio};

#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MAX: core::ffi::c_int = 0;
#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MIN: core::ffi::c_int = 1;
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
    /// Refloat v1.2.1 reads `mc_get_rpm()` in `src/motor_data.c:108`; the VESC
    /// ABI slot is declared at `vesc_pkg_lib/vesc_c_if.h:450`.
    fn electrical_speed(&self) -> ElectricalSpeed;
    /// Return firmware-calculated vehicle speed.
    ///
    /// Refloat v1.2.1 reads `mc_get_speed()` in `src/motor_data.c:118`; the
    /// VESC ABI slot is declared at `vesc_pkg_lib/vesc_c_if.h:470`.
    fn vehicle_speed(&self) -> VehicleSpeed;
    /// Return filtered total motor current.
    ///
    /// Refloat v1.2.1 reads `mc_get_tot_current_filtered()` in
    /// `src/motor_data.c:120`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:456`.
    fn motor_current(&self) -> TotalMotorCurrent;
    /// Return filtered motor current with the configured motor direction applied.
    fn directional_motor_current(&self) -> DirectionalMotorCurrent;
    /// Return the configured positive motor-current limit.
    ///
    /// Refloat v1.2.1 reads `CFG_PARAM_l_current_max` through `get_cfg_float`
    /// in `src/motor_data.c:91`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:243`.
    fn drive_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured braking-current magnitude.
    ///
    /// Refloat v1.2.1 stores `fabsf(CFG_PARAM_l_current_min)` in
    /// `src/motor_data.c:90`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:244`.
    fn brake_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured MOSFET temperature limit-start threshold.
    fn mosfet_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured motor temperature limit-start threshold.
    fn motor_temperature_limit_start(&self) -> TemperatureLimitStart;
    /// Return the configured maximum duty-cycle limit.
    ///
    /// Refloat v1.2.1 reads `CFG_PARAM_l_max_duty` through `get_cfg_float`
    /// in `src/motor_data.c:97`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:271` and has ABI value 22.
    fn duty_cycle_limit(&self) -> DutyCycleLimit;
    /// Return the configured battery cell count, when positive and representable.
    fn battery_cell_count(&self) -> Option<BatteryCellCount>;
    /// Return filtered input/battery current.
    ///
    /// Refloat v1.2.1 reads `mc_get_tot_current_in_filtered()` in
    /// `src/motor_data.c:140`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:460`.
    fn battery_current(&self) -> InputCurrent;
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
    /// Refloat v1.2.1 reads optional `foc_get_id` while encoding compact
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
    /// Return the filtered controller input voltage.
    fn input_voltage(&self) -> InputVoltage;
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

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        (**self).directional_motor_current()
    }

    fn drive_current_limit(&self) -> MotorCurrentLimit {
        (**self).drive_current_limit()
    }

    fn brake_current_limit(&self) -> MotorCurrentLimit {
        (**self).brake_current_limit()
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

    fn input_voltage(&self) -> InputVoltage {
        (**self).input_voltage()
    }
}

/// Motor-control operations backed by firmware slots.
#[cfg(not(test))]
pub trait MotorControlBindings {
    /// Reset the firmware motor-command safety timeout.
    ///
    /// Refloat v1.2.1 calls this before motor-control output at
    /// `third_party/refloat/src/motor_control.c:92-93`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:538`.
    fn timeout_reset(&self);
    /// Keep current control enabled after a current command.
    ///
    /// Refloat v1.2.1 sets `0.05f` seconds before sending requested current at
    /// `third_party/refloat/src/motor_control.c:96-99`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:476`.
    fn set_current_off_delay(&self, delay: CurrentOffDelay);
    /// Set motor current in amps.
    ///
    /// Refloat v1.2.1 sends the requested current at
    /// `third_party/refloat/src/motor_control.c:99`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:440`.
    fn set_current(&self, current: MotorCurrent);
    /// Set motor duty cycle.
    ///
    /// Refloat v1.2.1 sends parking-brake duty zero at
    /// `third_party/refloat/src/motor_control.c:112-114`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
    fn set_duty_cycle(&self, duty: DutyCycle);
    /// Set motor brake current in amps.
    ///
    /// Refloat v1.2.1 sends idle brake current at
    /// `third_party/refloat/src/motor_control.c:115-117`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
    fn set_brake_current(&self, current: BrakeCurrent);
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

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        DirectionalMotorCurrent::new(Current::from_amps(unsafe {
            crate::ffi::mc_get_tot_current_directional_filtered()
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

    fn input_voltage(&self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(unsafe {
            crate::ffi::mc_get_input_voltage_filtered()
        }))
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
    /// Return filtered motor current with the configured motor direction applied.
    fn directional_motor_current(&self) -> DirectionalMotorCurrent;
    /// Return the configured positive motor-current limit.
    fn drive_current_limit(&self) -> MotorCurrentLimit;
    /// Return the configured braking-current magnitude.
    fn brake_current_limit(&self) -> MotorCurrentLimit;
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
    /// Return the filtered controller input voltage.
    fn input_voltage(&self) -> InputVoltage;
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

    /// Return filtered motor current with the configured motor direction applied.
    pub fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        self.bindings.directional_motor_current()
    }

    /// Return the configured positive motor-current limit.
    pub fn drive_current_limit(&self) -> MotorCurrentLimit {
        self.bindings.drive_current_limit()
    }

    /// Return the configured braking-current magnitude.
    pub fn brake_current_limit(&self) -> MotorCurrentLimit {
        self.bindings.brake_current_limit()
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

    /// Return the filtered controller input voltage.
    pub fn input_voltage(&self) -> InputVoltage {
        self.bindings.input_voltage()
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

    fn directional_motor_current(&self) -> DirectionalMotorCurrent {
        self.directional_motor_current()
    }

    fn drive_current_limit(&self) -> MotorCurrentLimit {
        self.drive_current_limit()
    }

    fn brake_current_limit(&self) -> MotorCurrentLimit {
        self.brake_current_limit()
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

    fn input_voltage(&self) -> InputVoltage {
        self.input_voltage()
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
    /// Refloat uses this for parking brake duty zero at
    /// `third_party/refloat/src/motor_control.c:112-114`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
    pub fn set_duty_cycle(&self, duty: DutyCycle) {
        self.bindings.set_duty_cycle(duty);
    }

    /// Set motor brake current.
    ///
    /// Refloat uses this for idle brake current at
    /// `third_party/refloat/src/motor_control.c:115-117`; the VESC ABI slot is
    /// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
    pub fn set_brake_current(&self, current: BrakeCurrent) {
        self.bindings.set_brake_current(current);
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
}
