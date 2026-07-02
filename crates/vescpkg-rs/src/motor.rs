//! Motor telemetry helpers built on firmware motor-control table slots.

use crate::types::{
    AmpHoursCharged, AmpHoursDischarged, BatteryLevel, FirmwareFaultCode, InputVoltage,
    MosfetTemperature, MotorTemperature, TripDistance, WattHoursCharged, WattHoursDischarged,
};
use crate::units::OdometerMeters;
#[cfg(not(test))]
use crate::units::{Charge, Distance, Energy, Ratio, Temperature, Voltage};

/// Motor telemetry operations backed by firmware slots.
pub trait MotorTelemetryBindings {
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

#[cfg(not(test))]
/// Motor telemetry binding implementation that forwards to the live firmware ABI.
pub struct RealMotorTelemetryBindings;

#[cfg(not(test))]
impl MotorTelemetryBindings for RealMotorTelemetryBindings {
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

    fn input_voltage_filtered(&self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(unsafe {
            vescpkg_rs_sys::raw::mc_get_input_voltage_filtered()
        }))
    }
}

/// High-level motor telemetry API built on a binding implementation.
pub struct MotorTelemetryApi<B> {
    bindings: B,
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

#[cfg(any(test, feature = "test-support"))]
/// Motor telemetry fake binding helpers exported for tests.
pub mod test_support {
    use super::MotorTelemetryBindings;
    use crate::types::{
        AmpHoursCharged, AmpHoursDischarged, BatteryLevel, FirmwareFaultCode, InputVoltage,
        MosfetTemperature, MotorTemperature, TripDistance, WattHoursCharged, WattHoursDischarged,
    };
    use crate::units::{Charge, Distance, Energy, OdometerMeters, Ratio, Temperature, Voltage};
    use core::cell::Cell;

    /// Fake motor telemetry binding implementation used by package tests.
    pub struct FakeMotorTelemetryBindings {
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

        /// Creates fake motor telemetry bindings returning `input_voltage_filtered`.
        pub fn with_input_voltage_filtered(input_voltage_filtered: InputVoltage) -> Self {
            let bindings = Self::new();
            bindings.input_voltage_filtered.set(input_voltage_filtered);
            bindings
        }
    }

    impl MotorTelemetryBindings for FakeMotorTelemetryBindings {
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
}
