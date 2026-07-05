//! Motor telemetry helpers built on firmware motor-control table slots.

use crate::types::{
    AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, DutyCycle, ElectricalSpeed,
    FirmwareFaultCode, InputVoltage, MosfetTemperature, MotorCurrent, MotorTemperature,
    TripDistance, VehicleSpeed, WattHoursCharged, WattHoursDischarged,
};
#[cfg(not(test))]
use crate::units::{Charge, Current, Distance, Energy, Ratio, Rpm, Speed, Temperature, Voltage};
use crate::units::{OdometerMeters, SignedRatio};

#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MAX: core::ffi::c_int = 0;
#[cfg(not(test))]
const CFG_PARAM_L_CURRENT_MIN: core::ffi::c_int = 1;

/// Motor telemetry operations backed by firmware slots.
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
    fn motor_current(&self) -> MotorCurrent;
    /// Return the configured positive motor-current limit.
    ///
    /// Refloat v1.2.1 reads `CFG_PARAM_l_current_max` through `get_cfg_float`
    /// in `src/motor_data.c:91`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:243`.
    fn motor_current_max(&self) -> MotorCurrent;
    /// Return the configured braking-current magnitude.
    ///
    /// Refloat v1.2.1 stores `fabsf(CFG_PARAM_l_current_min)` in
    /// `src/motor_data.c:90`; the VESC config id is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:244`.
    fn motor_current_min(&self) -> MotorCurrent;
    /// Return filtered input/battery current.
    ///
    /// Refloat v1.2.1 reads `mc_get_tot_current_in_filtered()` in
    /// `src/motor_data.c:140`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:460`.
    fn battery_current(&self) -> BatteryCurrent;
    /// Return the current duty-cycle magnitude.
    ///
    /// The value is the absolute value of firmware `mc_get_duty_cycle_now()`,
    /// clamped to the signed ratio range and therefore always non-negative.
    ///
    /// Refloat v1.2.1 stores `fabsf(mc_get_duty_cycle_now())` as
    /// `duty_raw` in `src/motor_data.c:124`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:448`.
    fn duty_cycle_now(&self) -> DutyCycle;
    /// Return optional FOC d-axis Id current.
    ///
    /// Refloat v1.2.1 reads optional `foc_get_id` while encoding compact
    /// all-data at `src/main.c:1364-1368`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:616`.
    fn foc_id_current(&self) -> Option<MotorCurrent>;
    /// Return the absolute distance travelled by the motor/vehicle.
    fn distance_abs(&self) -> TripDistance;
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
    fn input_voltage_filtered(&self) -> InputVoltage;
}

/// Motor-control operations backed by firmware slots.
pub trait MotorControlBindings {
    /// Reset the firmware motor-command safety timeout.
    ///
    /// Refloat v1.2.1 calls this before motor-control output at
    /// `src/motor_control.c:92-93`.
    fn timeout_reset(&self);
    /// Keep current control enabled after a current command.
    ///
    /// Refloat v1.2.1 sets `0.05f` seconds before sending requested current at
    /// `src/motor_control.c:96-99`.
    fn set_current_off_delay(&self, seconds: f32);
    /// Set motor current in amps.
    ///
    /// Refloat v1.2.1 sends the requested current at `src/motor_control.c:99`.
    fn set_current(&self, current: MotorCurrent);
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
            vescpkg_rs_sys::raw::mc_get_rpm()
        }))
    }

    fn vehicle_speed(&self) -> VehicleSpeed {
        VehicleSpeed::new(Speed::from_meters_per_second(unsafe {
            vescpkg_rs_sys::raw::mc_get_speed()
        }))
    }

    fn motor_current(&self) -> MotorCurrent {
        MotorCurrent::new(Current::from_amps(unsafe {
            vescpkg_rs_sys::raw::mc_get_tot_current_filtered()
        }))
    }

    fn motor_current_max(&self) -> MotorCurrent {
        MotorCurrent::new(Current::from_amps(unsafe {
            vescpkg_rs_sys::raw::get_cfg_float(CFG_PARAM_L_CURRENT_MAX)
        }))
    }

    fn motor_current_min(&self) -> MotorCurrent {
        MotorCurrent::new(Current::from_amps(unsafe {
            vescpkg_rs_sys::raw::get_cfg_float(CFG_PARAM_L_CURRENT_MIN).abs()
        }))
    }

    fn battery_current(&self) -> BatteryCurrent {
        BatteryCurrent::new(Current::from_amps(unsafe {
            vescpkg_rs_sys::raw::mc_get_tot_current_in_filtered()
        }))
    }

    fn duty_cycle_now(&self) -> DutyCycle {
        duty_cycle_magnitude(unsafe { vescpkg_rs_sys::raw::mc_get_duty_cycle_now() })
    }

    fn foc_id_current(&self) -> Option<MotorCurrent> {
        unsafe { vescpkg_rs_sys::raw::foc_get_id() }
            .map(|amps| MotorCurrent::new(Current::from_amps(amps)))
    }

    fn distance_abs(&self) -> TripDistance {
        TripDistance::new(Distance::from_meters(unsafe {
            vescpkg_rs_sys::raw::mc_get_distance_abs()
        }))
    }

    fn mosfet_temperature(&self) -> MosfetTemperature {
        MosfetTemperature::new(Temperature::from_degrees_celsius(unsafe {
            vescpkg_rs_sys::raw::mc_temp_fet_filtered()
        }))
    }

    fn motor_temperature(&self) -> MotorTemperature {
        MotorTemperature::new(Temperature::from_degrees_celsius(unsafe {
            vescpkg_rs_sys::raw::mc_temp_motor_filtered()
        }))
    }

    fn odometer(&self) -> OdometerMeters {
        OdometerMeters::from_meters(unsafe { vescpkg_rs_sys::raw::mc_get_odometer() })
    }

    fn amp_hours_discharged(&self) -> AmpHoursDischarged {
        AmpHoursDischarged::new(Charge::from_amp_hours(unsafe {
            vescpkg_rs_sys::raw::mc_get_amp_hours(false)
        }))
    }

    fn amp_hours_charged(&self) -> AmpHoursCharged {
        AmpHoursCharged::new(Charge::from_amp_hours(unsafe {
            vescpkg_rs_sys::raw::mc_get_amp_hours_charged(false)
        }))
    }

    fn watt_hours_discharged(&self) -> WattHoursDischarged {
        WattHoursDischarged::new(Energy::from_watt_hours(unsafe {
            vescpkg_rs_sys::raw::mc_get_watt_hours(false)
        }))
    }

    fn watt_hours_charged(&self) -> WattHoursCharged {
        WattHoursCharged::new(Energy::from_watt_hours(unsafe {
            vescpkg_rs_sys::raw::mc_get_watt_hours_charged(false)
        }))
    }

    fn battery_level(&self) -> BatteryLevel {
        BatteryLevel::new(Ratio::clamped(unsafe {
            vescpkg_rs_sys::raw::mc_get_battery_level(core::ptr::null_mut())
        }))
    }

    fn firmware_fault(&self) -> FirmwareFaultCode {
        FirmwareFaultCode::from_raw_code(unsafe { vescpkg_rs_sys::raw::mc_get_fault() })
    }

    fn input_voltage_filtered(&self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(unsafe {
            vescpkg_rs_sys::raw::mc_get_input_voltage_filtered()
        }))
    }
}

#[cfg(not(test))]
impl MotorControlBindings for RealMotorControlBindings {
    fn timeout_reset(&self) {
        unsafe { vescpkg_rs_sys::raw::timeout_reset() };
    }

    fn set_current_off_delay(&self, seconds: f32) {
        unsafe { vescpkg_rs_sys::raw::mc_set_current_off_delay(seconds) };
    }

    fn set_current(&self, current: MotorCurrent) {
        unsafe { vescpkg_rs_sys::raw::mc_set_current(current.current().as_amps()) };
    }
}

fn duty_cycle_magnitude(raw_duty: f32) -> DutyCycle {
    let magnitude = if raw_duty.is_nan() {
        0.0
    } else {
        raw_duty.abs()
    };
    DutyCycle::new(SignedRatio::clamped(magnitude))
}

/// High-level motor telemetry API built on a binding implementation.
pub struct MotorTelemetryApi<B> {
    bindings: B,
}

/// High-level motor-control API built on a binding implementation.
pub struct MotorControlApi<B> {
    bindings: B,
}

#[cfg(test)]
mod tests {
    use super::duty_cycle_magnitude;

    #[test]
    fn duty_cycle_magnitude_never_reports_negative_nan() {
        assert_eq!(duty_cycle_magnitude(f32::NAN).ratio().as_ratio(), 0.0);
        assert_eq!(duty_cycle_magnitude(-0.42).ratio().as_ratio(), 0.42);
        assert_eq!(duty_cycle_magnitude(2.0).ratio().as_ratio(), 1.0);
    }
}

impl<B: MotorTelemetryBindings> MotorTelemetryApi<B> {
    /// Construct a new motor telemetry API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the wrapped motor telemetry bindings.
    pub fn bindings(&self) -> &B {
        &self.bindings
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
    pub fn motor_current(&self) -> MotorCurrent {
        self.bindings.motor_current()
    }

    /// Return the configured positive motor-current limit.
    pub fn motor_current_max(&self) -> MotorCurrent {
        self.bindings.motor_current_max()
    }

    /// Return the configured braking-current magnitude.
    pub fn motor_current_min(&self) -> MotorCurrent {
        self.bindings.motor_current_min()
    }

    /// Return filtered input/battery current.
    pub fn battery_current(&self) -> BatteryCurrent {
        self.bindings.battery_current()
    }

    /// Return the current duty-cycle magnitude.
    pub fn duty_cycle_now(&self) -> DutyCycle {
        self.bindings.duty_cycle_now()
    }

    /// Return optional FOC d-axis Id current.
    pub fn foc_id_current(&self) -> Option<MotorCurrent> {
        self.bindings.foc_id_current()
    }

    /// Return the absolute distance travelled by the motor/vehicle.
    pub fn distance_abs(&self) -> TripDistance {
        self.bindings.distance_abs()
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
    pub fn input_voltage_filtered(&self) -> InputVoltage {
        self.bindings.input_voltage_filtered()
    }
}

impl<B: MotorControlBindings> MotorControlApi<B> {
    /// Construct a new motor-control API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the wrapped motor-control bindings.
    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// Reset the firmware motor-command safety timeout.
    pub fn timeout_reset(&self) {
        self.bindings.timeout_reset();
    }

    /// Keep current control enabled after a current command.
    pub fn set_current_off_delay(&self, seconds: f32) {
        self.bindings.set_current_off_delay(seconds);
    }

    /// Set motor current.
    pub fn set_current(&self, current: MotorCurrent) {
        self.bindings.set_current(current);
    }
}

#[cfg(any(test, feature = "test-support"))]
/// Motor telemetry fake binding helpers exported for tests.
pub mod test_support {
    use super::{MotorControlBindings, MotorTelemetryBindings};
    use crate::types::{
        AmpHoursCharged, AmpHoursDischarged, BatteryCurrent, BatteryLevel, DutyCycle,
        ElectricalSpeed, FirmwareFaultCode, InputVoltage, MosfetTemperature, MotorCurrent,
        MotorTemperature, TripDistance, VehicleSpeed, WattHoursCharged, WattHoursDischarged,
    };
    use crate::units::{
        Charge, Current, Distance, Energy, OdometerMeters, Ratio, Rpm, SignedRatio, Speed,
        Temperature, Voltage,
    };
    use core::cell::Cell;

    /// Fake motor telemetry binding implementation used by package tests.
    pub struct FakeMotorTelemetryBindings {
        /// Number of electrical-speed calls observed.
        pub electrical_speed_calls: Cell<usize>,
        /// Number of vehicle-speed calls observed.
        pub vehicle_speed_calls: Cell<usize>,
        /// Number of motor-current calls observed.
        pub motor_current_calls: Cell<usize>,
        /// Number of motor-current max calls observed.
        pub motor_current_max_calls: Cell<usize>,
        /// Number of motor-current min calls observed.
        pub motor_current_min_calls: Cell<usize>,
        /// Number of battery-current calls observed.
        pub battery_current_calls: Cell<usize>,
        /// Number of duty-cycle calls observed.
        pub duty_cycle_now_calls: Cell<usize>,
        /// Number of optional FOC Id current calls observed.
        pub foc_id_current_calls: Cell<usize>,
        /// Number of absolute-distance calls observed.
        pub distance_abs_calls: Cell<usize>,
        /// Number of MOSFET temperature calls observed.
        pub mosfet_temperature_calls: Cell<usize>,
        /// Number of motor temperature calls observed.
        pub motor_temperature_calls: Cell<usize>,
        /// Number of odometer calls observed.
        pub odometer_calls: Cell<usize>,
        /// Number of discharged amp-hours calls observed.
        pub amp_hours_discharged_calls: Cell<usize>,
        /// Number of charged amp-hours calls observed.
        pub amp_hours_charged_calls: Cell<usize>,
        /// Number of discharged watt-hours calls observed.
        pub watt_hours_discharged_calls: Cell<usize>,
        /// Number of charged watt-hours calls observed.
        pub watt_hours_charged_calls: Cell<usize>,
        /// Number of battery-level calls observed.
        pub battery_level_calls: Cell<usize>,
        /// Number of firmware fault-code calls observed.
        pub firmware_fault_calls: Cell<usize>,
        /// Number of filtered input-voltage calls observed.
        pub input_voltage_filtered_calls: Cell<usize>,
        electrical_speed: Cell<ElectricalSpeed>,
        vehicle_speed: Cell<VehicleSpeed>,
        motor_current: Cell<MotorCurrent>,
        motor_current_max: Cell<MotorCurrent>,
        motor_current_min: Cell<MotorCurrent>,
        battery_current: Cell<BatteryCurrent>,
        duty_cycle_now: Cell<DutyCycle>,
        foc_id_current: Cell<Option<MotorCurrent>>,
        distance_abs: Cell<TripDistance>,
        mosfet_temperature: Cell<MosfetTemperature>,
        motor_temperature: Cell<MotorTemperature>,
        odometer: Cell<OdometerMeters>,
        amp_hours_discharged: Cell<AmpHoursDischarged>,
        amp_hours_charged: Cell<AmpHoursCharged>,
        watt_hours_discharged: Cell<WattHoursDischarged>,
        watt_hours_charged: Cell<WattHoursCharged>,
        battery_level: Cell<BatteryLevel>,
        firmware_fault: Cell<FirmwareFaultCode>,
        input_voltage_filtered: Cell<InputVoltage>,
    }

    impl Default for FakeMotorTelemetryBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeMotorTelemetryBindings {
        /// Creates fake motor telemetry bindings with zero distance.
        pub fn new() -> Self {
            Self {
                electrical_speed_calls: Cell::new(0),
                vehicle_speed_calls: Cell::new(0),
                motor_current_calls: Cell::new(0),
                motor_current_max_calls: Cell::new(0),
                motor_current_min_calls: Cell::new(0),
                battery_current_calls: Cell::new(0),
                duty_cycle_now_calls: Cell::new(0),
                foc_id_current_calls: Cell::new(0),
                distance_abs_calls: Cell::new(0),
                mosfet_temperature_calls: Cell::new(0),
                motor_temperature_calls: Cell::new(0),
                odometer_calls: Cell::new(0),
                amp_hours_discharged_calls: Cell::new(0),
                amp_hours_charged_calls: Cell::new(0),
                watt_hours_discharged_calls: Cell::new(0),
                watt_hours_charged_calls: Cell::new(0),
                battery_level_calls: Cell::new(0),
                firmware_fault_calls: Cell::new(0),
                input_voltage_filtered_calls: Cell::new(0),
                electrical_speed: Cell::new(ElectricalSpeed::new(
                    Rpm::from_revolutions_per_minute(0.0),
                )),
                vehicle_speed: Cell::new(VehicleSpeed::new(Speed::from_meters_per_second(0.0))),
                motor_current: Cell::new(MotorCurrent::new(Current::from_amps(0.0))),
                motor_current_max: Cell::new(MotorCurrent::new(Current::from_amps(100.0))),
                motor_current_min: Cell::new(MotorCurrent::new(Current::from_amps(100.0))),
                battery_current: Cell::new(BatteryCurrent::new(Current::from_amps(0.0))),
                duty_cycle_now: Cell::new(DutyCycle::new(SignedRatio::from_ratio_const(0.0))),
                foc_id_current: Cell::new(None),
                distance_abs: Cell::new(TripDistance::new(Distance::from_meters(0.0))),
                mosfet_temperature: Cell::new(MosfetTemperature::new(
                    Temperature::from_degrees_celsius(0.0),
                )),
                motor_temperature: Cell::new(MotorTemperature::new(
                    Temperature::from_degrees_celsius(0.0),
                )),
                odometer: Cell::new(OdometerMeters::from_meters(0)),
                amp_hours_discharged: Cell::new(AmpHoursDischarged::new(Charge::from_amp_hours(
                    0.0,
                ))),
                amp_hours_charged: Cell::new(AmpHoursCharged::new(Charge::from_amp_hours(0.0))),
                watt_hours_discharged: Cell::new(WattHoursDischarged::new(
                    Energy::from_watt_hours(0.0),
                )),
                watt_hours_charged: Cell::new(WattHoursCharged::new(Energy::from_watt_hours(0.0))),
                battery_level: Cell::new(BatteryLevel::new(Ratio::from_ratio_const(0.0))),
                firmware_fault: Cell::new(FirmwareFaultCode::from_compat_code(0)),
                input_voltage_filtered: Cell::new(InputVoltage::new(Voltage::from_volts(0.0))),
            }
        }

        /// Return fake motor telemetry bindings returning source-backed runtime motor fields.
        pub fn with_runtime_motor(
            self,
            electrical_speed: ElectricalSpeed,
            vehicle_speed: VehicleSpeed,
            motor_current: MotorCurrent,
            battery_current: BatteryCurrent,
            duty_cycle_now: DutyCycle,
        ) -> Self {
            self.electrical_speed.set(electrical_speed);
            self.vehicle_speed.set(vehicle_speed);
            self.motor_current.set(motor_current);
            self.battery_current.set(battery_current);
            self.duty_cycle_now.set(duty_cycle_now);
            self
        }

        /// Return fake motor telemetry bindings returning configured current limits.
        pub fn with_motor_current_limits(
            self,
            motor_current_max: MotorCurrent,
            motor_current_min: MotorCurrent,
        ) -> Self {
            self.motor_current_max.set(motor_current_max);
            self.motor_current_min.set(motor_current_min);
            self
        }

        /// Return fake motor telemetry bindings returning `distance_abs`.
        pub fn with_distance_abs(self, distance_abs: TripDistance) -> Self {
            self.distance_abs.set(distance_abs);
            self
        }

        /// Return fake motor telemetry bindings returning the supplied temperatures.
        pub fn with_temperatures(
            self,
            mosfet_temperature: MosfetTemperature,
            motor_temperature: MotorTemperature,
        ) -> Self {
            self.mosfet_temperature.set(mosfet_temperature);
            self.motor_temperature.set(motor_temperature);
            self
        }

        /// Return fake motor telemetry bindings returning accumulated ride totals.
        pub fn with_ride_totals(
            self,
            odometer: OdometerMeters,
            amp_hours_discharged: AmpHoursDischarged,
            amp_hours_charged: AmpHoursCharged,
            watt_hours_discharged: WattHoursDischarged,
            watt_hours_charged: WattHoursCharged,
            battery_level: BatteryLevel,
        ) -> Self {
            self.odometer.set(odometer);
            self.amp_hours_discharged.set(amp_hours_discharged);
            self.amp_hours_charged.set(amp_hours_charged);
            self.watt_hours_discharged.set(watt_hours_discharged);
            self.watt_hours_charged.set(watt_hours_charged);
            self.battery_level.set(battery_level);
            self
        }

        /// Return fake motor telemetry bindings returning `firmware_fault`.
        pub fn with_firmware_fault(self, firmware_fault: FirmwareFaultCode) -> Self {
            self.firmware_fault.set(firmware_fault);
            self
        }

        /// Return fake motor telemetry bindings returning `input_voltage_filtered`.
        pub fn with_input_voltage_filtered(self, input_voltage_filtered: InputVoltage) -> Self {
            self.input_voltage_filtered.set(input_voltage_filtered);
            self
        }

        /// Return fake motor telemetry bindings returning optional FOC Id current.
        pub fn with_foc_id_current(self, foc_id_current: Option<MotorCurrent>) -> Self {
            self.foc_id_current.set(foc_id_current);
            self
        }

        /// Creates fake motor telemetry bindings returning voltage and temperatures.
        pub fn with_input_voltage_and_temperatures(
            input_voltage_filtered: InputVoltage,
            mosfet_temperature: MosfetTemperature,
            motor_temperature: MotorTemperature,
        ) -> Self {
            let bindings = Self::new().with_temperatures(mosfet_temperature, motor_temperature);
            bindings.input_voltage_filtered.set(input_voltage_filtered);
            bindings
        }
    }

    impl MotorTelemetryBindings for FakeMotorTelemetryBindings {
        fn electrical_speed(&self) -> ElectricalSpeed {
            self.electrical_speed_calls
                .set(self.electrical_speed_calls.get() + 1);
            self.electrical_speed.get()
        }

        fn vehicle_speed(&self) -> VehicleSpeed {
            self.vehicle_speed_calls
                .set(self.vehicle_speed_calls.get() + 1);
            self.vehicle_speed.get()
        }

        fn motor_current(&self) -> MotorCurrent {
            self.motor_current_calls
                .set(self.motor_current_calls.get() + 1);
            self.motor_current.get()
        }

        fn motor_current_max(&self) -> MotorCurrent {
            self.motor_current_max_calls
                .set(self.motor_current_max_calls.get() + 1);
            self.motor_current_max.get()
        }

        fn motor_current_min(&self) -> MotorCurrent {
            self.motor_current_min_calls
                .set(self.motor_current_min_calls.get() + 1);
            self.motor_current_min.get()
        }

        fn battery_current(&self) -> BatteryCurrent {
            self.battery_current_calls
                .set(self.battery_current_calls.get() + 1);
            self.battery_current.get()
        }

        fn duty_cycle_now(&self) -> DutyCycle {
            self.duty_cycle_now_calls
                .set(self.duty_cycle_now_calls.get() + 1);
            self.duty_cycle_now.get()
        }

        fn foc_id_current(&self) -> Option<MotorCurrent> {
            self.foc_id_current_calls
                .set(self.foc_id_current_calls.get() + 1);
            self.foc_id_current.get()
        }

        fn distance_abs(&self) -> TripDistance {
            self.distance_abs_calls
                .set(self.distance_abs_calls.get() + 1);
            self.distance_abs.get()
        }

        fn mosfet_temperature(&self) -> MosfetTemperature {
            self.mosfet_temperature_calls
                .set(self.mosfet_temperature_calls.get() + 1);
            self.mosfet_temperature.get()
        }

        fn motor_temperature(&self) -> MotorTemperature {
            self.motor_temperature_calls
                .set(self.motor_temperature_calls.get() + 1);
            self.motor_temperature.get()
        }

        fn odometer(&self) -> OdometerMeters {
            self.odometer_calls.set(self.odometer_calls.get() + 1);
            self.odometer.get()
        }

        fn amp_hours_discharged(&self) -> AmpHoursDischarged {
            self.amp_hours_discharged_calls
                .set(self.amp_hours_discharged_calls.get() + 1);
            self.amp_hours_discharged.get()
        }

        fn amp_hours_charged(&self) -> AmpHoursCharged {
            self.amp_hours_charged_calls
                .set(self.amp_hours_charged_calls.get() + 1);
            self.amp_hours_charged.get()
        }

        fn watt_hours_discharged(&self) -> WattHoursDischarged {
            self.watt_hours_discharged_calls
                .set(self.watt_hours_discharged_calls.get() + 1);
            self.watt_hours_discharged.get()
        }

        fn watt_hours_charged(&self) -> WattHoursCharged {
            self.watt_hours_charged_calls
                .set(self.watt_hours_charged_calls.get() + 1);
            self.watt_hours_charged.get()
        }

        fn battery_level(&self) -> BatteryLevel {
            self.battery_level_calls
                .set(self.battery_level_calls.get() + 1);
            self.battery_level.get()
        }

        fn firmware_fault(&self) -> FirmwareFaultCode {
            self.firmware_fault_calls
                .set(self.firmware_fault_calls.get() + 1);
            self.firmware_fault.get()
        }

        fn input_voltage_filtered(&self) -> InputVoltage {
            self.input_voltage_filtered_calls
                .set(self.input_voltage_filtered_calls.get() + 1);
            self.input_voltage_filtered.get()
        }
    }

    /// Fake motor-control bindings used by package tests.
    pub struct FakeMotorControlBindings {
        /// Number of timeout-reset calls observed.
        pub timeout_reset_calls: Cell<usize>,
        /// Number of current-off-delay calls observed.
        pub set_current_off_delay_calls: Cell<usize>,
        /// Number of current command calls observed.
        pub set_current_calls: Cell<usize>,
        current_off_delay_seconds: Cell<f32>,
        current: Cell<MotorCurrent>,
    }

    impl Default for FakeMotorControlBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeMotorControlBindings {
        /// Creates fake motor-control bindings with zeroed captured values.
        pub fn new() -> Self {
            Self {
                timeout_reset_calls: Cell::new(0),
                set_current_off_delay_calls: Cell::new(0),
                set_current_calls: Cell::new(0),
                current_off_delay_seconds: Cell::new(0.0),
                current: Cell::new(MotorCurrent::new(Current::from_amps(0.0))),
            }
        }

        /// Return the most recent current-off-delay seconds.
        pub fn current_off_delay_seconds(&self) -> f32 {
            self.current_off_delay_seconds.get()
        }

        /// Return the most recent current command.
        pub fn current(&self) -> MotorCurrent {
            self.current.get()
        }
    }

    impl MotorControlBindings for FakeMotorControlBindings {
        fn timeout_reset(&self) {
            self.timeout_reset_calls
                .set(self.timeout_reset_calls.get() + 1);
        }

        fn set_current_off_delay(&self, seconds: f32) {
            self.set_current_off_delay_calls
                .set(self.set_current_off_delay_calls.get() + 1);
            self.current_off_delay_seconds.set(seconds);
        }

        fn set_current(&self, current: MotorCurrent) {
            self.set_current_calls.set(self.set_current_calls.get() + 1);
            self.current.set(current);
        }
    }
}
