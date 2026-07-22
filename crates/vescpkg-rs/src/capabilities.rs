//! Capability-aware safe subsystem constructors.

use crate::ffi;
use crate::{
    BatteryCellCount, CanBus, Current, DutyCycleLimit, ElectricalSpeed, FocAudio,
    FocMotorFluxLinkage, FocMotorInductance, FocMotorResistance, GearRatio, InputCurrent,
    InputVoltage, MotorCurrentLimit, Nvm, NvmCapacity, Ratio, TemperatureLimitEnd,
    TemperatureLimitStart, Uart, Voltage, WheelDiameter,
};
use core::fmt;
use vescpkg_rs_sys::{AbiError, Stm32AbiRevision, VescIfCapabilities, VescIfPresence};

/// Observed firmware capabilities used to construct safe subsystem handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareCapabilities {
    inner: VescIfCapabilities,
}

/// A floating-point firmware setting exposed by the pinned VESC ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareFloatSetting {
    /// Maximum motor current (`CFG_PARAM_l_current_max`).
    MotorCurrentMax,
    /// Minimum motor current (`CFG_PARAM_l_current_min`).
    MotorCurrentMin,
    /// Maximum battery/input current (`CFG_PARAM_l_in_current_max`).
    InputCurrentMax,
    /// Minimum battery/input current (`CFG_PARAM_l_in_current_min`).
    InputCurrentMin,
    /// Absolute motor current ceiling (`CFG_PARAM_l_abs_current_max`).
    AbsoluteCurrentMax,
    /// Minimum electrical speed (`CFG_PARAM_l_min_erpm`).
    MinimumElectricalSpeed,
    /// Maximum electrical speed (`CFG_PARAM_l_max_erpm`).
    MaximumElectricalSpeed,
    /// Electrical-speed ramp start (`CFG_PARAM_l_erpm_start`).
    ElectricalSpeedRampStart,
    /// Maximum electrical speed during braking (`CFG_PARAM_l_max_erpm_fbrake`).
    MaximumElectricalSpeedBrake,
    /// Maximum electrical speed during braking-current control (`CFG_PARAM_l_max_erpm_fbrake_cc`).
    MaximumElectricalSpeedBrakeCurrent,
    /// Minimum input voltage (`CFG_PARAM_l_min_vin`).
    MinimumInputVoltage,
    /// Maximum input voltage (`CFG_PARAM_l_max_vin`).
    MaximumInputVoltage,
    /// Battery cut-start voltage (`CFG_PARAM_l_battery_cut_start`).
    BatteryCutStartVoltage,
    /// Battery cut-end voltage (`CFG_PARAM_l_battery_cut_end`).
    BatteryCutEndVoltage,
    /// MOSFET temperature limit-start threshold.
    MosfetTemperatureStart,
    /// MOSFET temperature limit-end threshold.
    MosfetTemperatureEnd,
    /// Motor temperature limit-start threshold.
    MotorTemperatureStart,
    /// Motor temperature limit-end threshold.
    MotorTemperatureEnd,
    /// Temperature-based acceleration/deceleration threshold.
    TemperatureAccelerationDecrease,
    /// Minimum duty-cycle limit.
    MinDuty,
    /// Maximum duty-cycle limit.
    MaxDuty,
    /// IMU accelerometer confidence decay (`CFG_PARAM_IMU_accel_confidence_decay`).
    ImuAccelerationConfidenceDecay,
    /// Mahony proportional gain (`CFG_PARAM_IMU_mahony_kp`).
    ImuMahonyKp,
    /// Mahony integral gain (`CFG_PARAM_IMU_mahony_ki`).
    ImuMahonyKi,
    /// Madgwick beta gain (`CFG_PARAM_IMU_madgwick_beta`).
    ImuMadgwickBeta,
    /// IMU roll mounting rotation (`CFG_PARAM_IMU_rot_roll`).
    ImuRotationRoll,
    /// IMU pitch mounting rotation (`CFG_PARAM_IMU_rot_pitch`).
    ImuRotationPitch,
    /// IMU yaw mounting rotation (`CFG_PARAM_IMU_rot_yaw`).
    ImuRotationYaw,
    /// IMU sample rate (`CFG_PARAM_IMU_sample_rate`).
    ImuSampleRate,
    /// IMU accelerometer X offset (`CFG_PARAM_IMU_accel_offset_x`).
    ImuAccelerationOffsetX,
    /// IMU accelerometer Y offset (`CFG_PARAM_IMU_accel_offset_y`).
    ImuAccelerationOffsetY,
    /// IMU accelerometer Z offset (`CFG_PARAM_IMU_accel_offset_z`).
    ImuAccelerationOffsetZ,
    /// IMU gyro X offset (`CFG_PARAM_IMU_gyro_offset_x`).
    ImuGyroOffsetX,
    /// IMU gyro Y offset (`CFG_PARAM_IMU_gyro_offset_y`).
    ImuGyroOffsetY,
    /// IMU gyro Z offset (`CFG_PARAM_IMU_gyro_offset_z`).
    ImuGyroOffsetZ,
    /// Gear ratio (`CFG_PARAM_si_gear_ratio`).
    GearRatio,
    /// Wheel diameter (`CFG_PARAM_si_wheel_diameter`).
    WheelDiameter,
    /// Battery capacity in amp-hours (`CFG_PARAM_si_battery_ah`).
    BatteryCapacity,
    /// Motor no-load current (`CFG_PARAM_si_motor_nl_current`).
    MotorNoLoadCurrent,
    /// FOC motor resistance (`CFG_PARAM_foc_motor_r`).
    FocMotorResistance,
    /// FOC motor inductance (`CFG_PARAM_foc_motor_l`).
    FocMotorInductance,
    /// FOC motor flux linkage (`CFG_PARAM_foc_motor_flux_linkage`).
    FocMotorFluxLinkage,
}

impl FirmwareFloatSetting {
    const fn raw(self) -> i32 {
        match self {
            Self::MotorCurrentMax => 0,
            Self::MotorCurrentMin => 1,
            Self::InputCurrentMax => 2,
            Self::InputCurrentMin => 3,
            Self::AbsoluteCurrentMax => 4,
            Self::MinimumElectricalSpeed => 5,
            Self::MaximumElectricalSpeed => 6,
            Self::ElectricalSpeedRampStart => 7,
            Self::MaximumElectricalSpeedBrake => 8,
            Self::MaximumElectricalSpeedBrakeCurrent => 9,
            Self::MinimumInputVoltage => 10,
            Self::MaximumInputVoltage => 11,
            Self::BatteryCutStartVoltage => 12,
            Self::BatteryCutEndVoltage => 13,
            Self::MosfetTemperatureStart => 16,
            Self::MosfetTemperatureEnd => 17,
            Self::MotorTemperatureStart => 18,
            Self::MotorTemperatureEnd => 19,
            Self::TemperatureAccelerationDecrease => 20,
            Self::MinDuty => 21,
            Self::MaxDuty => 22,
            Self::ImuAccelerationConfidenceDecay => 23,
            Self::ImuMahonyKp => 24,
            Self::ImuMahonyKi => 25,
            Self::ImuMadgwickBeta => 26,
            Self::ImuRotationRoll => 27,
            Self::ImuRotationPitch => 28,
            Self::ImuRotationYaw => 29,
            Self::ImuSampleRate => 31,
            Self::ImuAccelerationOffsetX => 32,
            Self::ImuAccelerationOffsetY => 33,
            Self::ImuAccelerationOffsetZ => 34,
            Self::ImuGyroOffsetX => 35,
            Self::ImuGyroOffsetY => 36,
            Self::ImuGyroOffsetZ => 37,
            Self::GearRatio => 40,
            Self::WheelDiameter => 41,
            Self::BatteryCapacity => 44,
            Self::MotorNoLoadCurrent => 45,
            Self::FocMotorResistance => 46,
            Self::FocMotorInductance => 47,
            Self::FocMotorFluxLinkage => 48,
        }
    }
}

/// An integer firmware setting exposed by the pinned VESC ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareIntSetting {
    /// CAN application mode (`CFG_PARAM_app_can_mode`).
    AppCanMode,
    /// CAN bus baud-rate selector (`CFG_PARAM_app_can_baud_rate`).
    AppCanBaudRate,
    /// IMU AHRS algorithm selector (`CFG_PARAM_IMU_ahrs_mode`).
    ImuAhrsMode,
    /// Application shutdown mode (`CFG_PARAM_app_shutdown_mode`).
    AppShutdownMode,
    /// Motor pole count (`CFG_PARAM_si_motor_poles`).
    MotorPoleCount,
    /// Battery chemistry selector (`CFG_PARAM_si_battery_type`).
    BatteryType,
    /// Battery cell count (`CFG_PARAM_si_battery_cells`).
    BatteryCellCount,
}

impl FirmwareIntSetting {
    const fn raw(self) -> i32 {
        match self {
            Self::AppCanMode => 14,
            Self::AppCanBaudRate => 15,
            Self::ImuAhrsMode => 30,
            Self::AppShutdownMode => 38,
            Self::MotorPoleCount => 39,
            Self::BatteryType => 42,
            Self::BatteryCellCount => 43,
        }
    }
}

/// Error returned when firmware rejects a settings write or persistence request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    /// The firmware rejected the requested operation.
    Rejected {
        /// Operation rejected by firmware.
        operation: &'static str,
    },
    /// The requested value cannot be represented as a live setting.
    InvalidValue,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { operation } => write!(formatter, "firmware rejected {operation}"),
            Self::InvalidValue => formatter.write_str("setting value must be finite"),
        }
    }
}

impl core::error::Error for SettingsError {}

/// Checked settings capability backed by the live VESC configuration slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareSettings;

impl FirmwareSettings {
    /// Read a floating-point setting from live firmware state.
    pub fn get_float(self, setting: FirmwareFloatSetting) -> f32 {
        unsafe { ffi::get_cfg_float(setting.raw()) }
    }

    /// Write a floating-point setting to live firmware state.
    pub fn set_float(self, setting: FirmwareFloatSetting, value: f32) -> Result<(), SettingsError> {
        if !value.is_finite() {
            return Err(SettingsError::InvalidValue);
        }
        unsafe { ffi::set_cfg_float(setting.raw(), value) }
            .then_some(())
            .ok_or(SettingsError::Rejected {
                operation: "float setting",
            })
    }

    /// Read the positive configured motor-current ceiling without erasing its domain.
    pub fn motor_current_max(self) -> MotorCurrentLimit {
        MotorCurrentLimit::new(Current::from_amps(
            self.get_float(FirmwareFloatSetting::MotorCurrentMax),
        ))
    }

    /// Read the positive configured motor-current floor magnitude.
    pub fn motor_current_min(self) -> MotorCurrentLimit {
        MotorCurrentLimit::new(Current::from_amps(
            self.get_float(FirmwareFloatSetting::MotorCurrentMin),
        ))
    }

    /// Update the live motor-current ceiling; persistence still requires [`Self::store`].
    pub fn set_motor_current_max(self, limit: MotorCurrentLimit) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MotorCurrentMax,
            limit.current().as_amps(),
        )
    }

    /// Update the live motor-current floor magnitude; persistence still requires [`Self::store`].
    pub fn set_motor_current_min(self, limit: MotorCurrentLimit) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MotorCurrentMin,
            -limit.current().as_amps(),
        )
    }

    /// Read the configured battery/input current ceiling.
    pub fn input_current_max(self) -> InputCurrent {
        InputCurrent::new(Current::from_amps(
            self.get_float(FirmwareFloatSetting::InputCurrentMax),
        ))
    }

    /// Read the configured minimum battery/input current.
    pub fn input_current_min(self) -> InputCurrent {
        InputCurrent::new(Current::from_amps(
            self.get_float(FirmwareFloatSetting::InputCurrentMin),
        ))
    }

    /// Read the configured absolute motor-current ceiling.
    pub fn absolute_current_max(self) -> MotorCurrentLimit {
        MotorCurrentLimit::new(Current::from_amps(
            self.get_float(FirmwareFloatSetting::AbsoluteCurrentMax),
        ))
    }

    /// Read the configured minimum electrical speed.
    pub fn minimum_electrical_speed(self) -> ElectricalSpeed {
        ElectricalSpeed::new(crate::Rpm::from_revolutions_per_minute(
            self.get_float(FirmwareFloatSetting::MinimumElectricalSpeed),
        ))
    }

    /// Read the configured maximum electrical speed.
    pub fn maximum_electrical_speed(self) -> ElectricalSpeed {
        ElectricalSpeed::new(crate::Rpm::from_revolutions_per_minute(
            self.get_float(FirmwareFloatSetting::MaximumElectricalSpeed),
        ))
    }

    /// Read the configured positive gear ratio, rejecting malformed firmware state.
    pub fn gear_ratio(self) -> Result<GearRatio, SettingsError> {
        GearRatio::try_new(self.get_float(FirmwareFloatSetting::GearRatio))
            .map_err(|_| SettingsError::InvalidValue)
    }

    /// Read the configured wheel diameter.
    pub fn wheel_diameter(self) -> WheelDiameter {
        WheelDiameter::new(crate::Distance::from_meters(
            self.get_float(FirmwareFloatSetting::WheelDiameter),
        ))
    }

    /// Read the configured FOC motor resistance.
    pub fn foc_motor_resistance(self) -> FocMotorResistance {
        FocMotorResistance::new(crate::Resistance::from_ohms(
            self.get_float(FirmwareFloatSetting::FocMotorResistance),
        ))
    }

    /// Read the configured FOC motor inductance.
    pub fn foc_motor_inductance(self) -> FocMotorInductance {
        FocMotorInductance::new(crate::Inductance::from_henries(
            self.get_float(FirmwareFloatSetting::FocMotorInductance),
        ))
    }

    /// Read the configured FOC motor flux linkage.
    pub fn foc_motor_flux_linkage(self) -> FocMotorFluxLinkage {
        FocMotorFluxLinkage::new(crate::FluxLinkage::from_webers(
            self.get_float(FirmwareFloatSetting::FocMotorFluxLinkage),
        ))
    }

    /// Update the live battery/input current ceiling; persistence still requires [`Self::store`].
    pub fn set_input_current_max(self, current: InputCurrent) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::InputCurrentMax,
            current.current().as_amps(),
        )
    }

    /// Update the live minimum battery/input current; persistence still requires [`Self::store`].
    pub fn set_input_current_min(self, current: InputCurrent) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::InputCurrentMin,
            current.current().as_amps(),
        )
    }

    /// Update the live absolute motor-current ceiling; persistence still requires [`Self::store`].
    pub fn set_absolute_current_max(self, limit: MotorCurrentLimit) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::AbsoluteCurrentMax,
            limit.current().as_amps(),
        )
    }

    /// Update the live minimum electrical speed; persistence still requires [`Self::store`].
    pub fn set_minimum_electrical_speed(self, speed: ElectricalSpeed) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MinimumElectricalSpeed,
            speed.rpm().as_revolutions_per_minute(),
        )
    }

    /// Update the live maximum electrical speed; persistence still requires [`Self::store`].
    pub fn set_maximum_electrical_speed(self, speed: ElectricalSpeed) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MaximumElectricalSpeed,
            speed.rpm().as_revolutions_per_minute(),
        )
    }

    /// Update the live gear ratio; persistence still requires [`Self::store`].
    pub fn set_gear_ratio(self, ratio: GearRatio) -> Result<(), SettingsError> {
        self.set_float(FirmwareFloatSetting::GearRatio, ratio.as_f32())
    }

    /// Update the live wheel diameter; persistence still requires [`Self::store`].
    pub fn set_wheel_diameter(self, diameter: WheelDiameter) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::WheelDiameter,
            diameter.distance().as_meters(),
        )
    }

    /// Update the live FOC motor resistance; persistence still requires [`Self::store`].
    pub fn set_foc_motor_resistance(
        self,
        resistance: FocMotorResistance,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::FocMotorResistance,
            resistance.resistance().as_ohms(),
        )
    }

    /// Update the live FOC motor inductance; persistence still requires [`Self::store`].
    pub fn set_foc_motor_inductance(
        self,
        inductance: FocMotorInductance,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::FocMotorInductance,
            inductance.inductance().as_henries(),
        )
    }

    /// Update the live FOC motor flux linkage; persistence still requires [`Self::store`].
    pub fn set_foc_motor_flux_linkage(
        self,
        flux_linkage: FocMotorFluxLinkage,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::FocMotorFluxLinkage,
            flux_linkage.flux_linkage().as_webers(),
        )
    }

    /// Read the configured minimum input voltage.
    pub fn input_voltage_min(self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(
            self.get_float(FirmwareFloatSetting::MinimumInputVoltage),
        ))
    }

    /// Read the configured maximum input voltage.
    pub fn input_voltage_max(self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(
            self.get_float(FirmwareFloatSetting::MaximumInputVoltage),
        ))
    }

    /// Read the configured battery cut-start voltage.
    pub fn battery_cut_start_voltage(self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(
            self.get_float(FirmwareFloatSetting::BatteryCutStartVoltage),
        ))
    }

    /// Read the configured battery cut-end voltage.
    pub fn battery_cut_end_voltage(self) -> InputVoltage {
        InputVoltage::new(Voltage::from_volts(
            self.get_float(FirmwareFloatSetting::BatteryCutEndVoltage),
        ))
    }

    /// Update the live minimum input-voltage cut threshold; persistence still requires [`Self::store`].
    pub fn set_input_voltage_min(self, voltage: InputVoltage) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MinimumInputVoltage,
            voltage.voltage().as_volts(),
        )
    }

    /// Update the live maximum input-voltage cut threshold; persistence still requires [`Self::store`].
    pub fn set_input_voltage_max(self, voltage: InputVoltage) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MaximumInputVoltage,
            voltage.voltage().as_volts(),
        )
    }

    /// Update the live battery cut-start voltage; persistence still requires [`Self::store`].
    pub fn set_battery_cut_start_voltage(self, voltage: InputVoltage) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::BatteryCutStartVoltage,
            voltage.voltage().as_volts(),
        )
    }

    /// Update the live battery cut-end voltage; persistence still requires [`Self::store`].
    pub fn set_battery_cut_end_voltage(self, voltage: InputVoltage) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::BatteryCutEndVoltage,
            voltage.voltage().as_volts(),
        )
    }

    /// Read the MOSFET temperature limit-start threshold.
    pub fn mosfet_temperature_start(self) -> TemperatureLimitStart {
        TemperatureLimitStart::new(crate::Temperature::from_degrees_celsius(
            self.get_float(FirmwareFloatSetting::MosfetTemperatureStart),
        ))
    }

    /// Read the MOSFET temperature limit-end threshold.
    pub fn mosfet_temperature_end(self) -> TemperatureLimitEnd {
        TemperatureLimitEnd::new(crate::Temperature::from_degrees_celsius(
            self.get_float(FirmwareFloatSetting::MosfetTemperatureEnd),
        ))
    }

    /// Read the motor temperature limit-start threshold.
    pub fn motor_temperature_start(self) -> TemperatureLimitStart {
        TemperatureLimitStart::new(crate::Temperature::from_degrees_celsius(
            self.get_float(FirmwareFloatSetting::MotorTemperatureStart),
        ))
    }

    /// Read the motor temperature limit-end threshold.
    pub fn motor_temperature_end(self) -> TemperatureLimitEnd {
        TemperatureLimitEnd::new(crate::Temperature::from_degrees_celsius(
            self.get_float(FirmwareFloatSetting::MotorTemperatureEnd),
        ))
    }

    /// Update the live MOSFET temperature limit-start threshold; persistence still requires [`Self::store`].
    pub fn set_mosfet_temperature_start(
        self,
        temperature: TemperatureLimitStart,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MosfetTemperatureStart,
            temperature.temperature().as_degrees_celsius(),
        )
    }

    /// Update the live MOSFET temperature limit-end threshold; persistence still requires [`Self::store`].
    pub fn set_mosfet_temperature_end(
        self,
        temperature: TemperatureLimitEnd,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MosfetTemperatureEnd,
            temperature.temperature().as_degrees_celsius(),
        )
    }

    /// Update the live motor temperature limit-start threshold; persistence still requires [`Self::store`].
    pub fn set_motor_temperature_start(
        self,
        temperature: TemperatureLimitStart,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MotorTemperatureStart,
            temperature.temperature().as_degrees_celsius(),
        )
    }

    /// Update the live motor temperature limit-end threshold; persistence still requires [`Self::store`].
    pub fn set_motor_temperature_end(
        self,
        temperature: TemperatureLimitEnd,
    ) -> Result<(), SettingsError> {
        self.set_float(
            FirmwareFloatSetting::MotorTemperatureEnd,
            temperature.temperature().as_degrees_celsius(),
        )
    }

    /// Read the configured maximum duty-cycle ratio, clamping malformed firmware output.
    pub fn duty_cycle_limit(self) -> DutyCycleLimit {
        DutyCycleLimit::new(Ratio::clamped(
            self.get_float(FirmwareFloatSetting::MaxDuty),
        ))
    }

    /// Update the live duty-cycle limit; persistence still requires [`Self::store`].
    pub fn set_duty_cycle_limit(self, limit: DutyCycleLimit) -> Result<(), SettingsError> {
        self.set_float(FirmwareFloatSetting::MaxDuty, limit.ratio().as_ratio())
    }

    /// Read an integer setting from live firmware state.
    pub fn get_int(self, setting: FirmwareIntSetting) -> i32 {
        unsafe { ffi::get_cfg_int(setting.raw()) }
    }

    /// Read the configured battery-cell count with semantic validation.
    pub fn battery_cell_count(self) -> Result<BatteryCellCount, SettingsError> {
        let raw = self.get_int(FirmwareIntSetting::BatteryCellCount);
        u16::try_from(raw)
            .ok()
            .and_then(|value| BatteryCellCount::try_new(value).ok())
            .ok_or(SettingsError::InvalidValue)
    }

    /// Write an integer setting to live firmware state.
    pub fn set_int(self, setting: FirmwareIntSetting, value: i32) -> Result<(), SettingsError> {
        unsafe { ffi::set_cfg_int(setting.raw(), value) }
            .then_some(())
            .ok_or(SettingsError::Rejected {
                operation: "integer setting",
            })
    }

    /// Write a checked battery-cell count to live firmware state.
    pub fn set_battery_cell_count(self, count: BatteryCellCount) -> Result<(), SettingsError> {
        self.set_int(
            FirmwareIntSetting::BatteryCellCount,
            i32::from(count.as_u16()),
        )
    }

    /// Persist all accepted setting writes in firmware storage.
    pub fn store(self) -> Result<(), SettingsError> {
        unsafe { ffi::store_cfg() }
            .then_some(())
            .ok_or(SettingsError::Rejected {
                operation: "settings persistence",
            })
    }
}

impl FirmwareCapabilities {
    /// Construct capabilities from one bounded table-presence snapshot.
    pub const fn new(presence: VescIfPresence) -> Self {
        Self {
            inner: VescIfCapabilities::new(presence),
        }
    }

    /// Return the observed slot presence used by this value.
    pub const fn presence(self) -> VescIfPresence {
        self.inner.presence()
    }

    /// Return the descriptive revision inferred from observed pointers.
    pub fn revision(self) -> Stm32AbiRevision {
        self.inner.revision()
    }

    /// Construct a CAN handle only when its observed transmit entry exists.
    pub fn can_bus(self) -> Result<CanBus, AbiError> {
        self.inner.can().map(|_| CanBus::new())
    }

    /// Construct an NVM handle only when its observed read entry exists.
    pub fn nvm(self) -> Result<Nvm, AbiError> {
        self.inner.nvm().map(|_| Nvm::new())
    }

    /// Construct NVM with a separately discovered byte capacity.
    pub fn nvm_with_capacity(self, capacity: NvmCapacity) -> Result<Nvm, AbiError> {
        self.inner.nvm().map(|_| Nvm::with_capacity(capacity))
    }

    /// Construct an FOC audio handle only when its observed beep entry exists.
    pub fn audio(self) -> Result<FocAudio, AbiError> {
        self.inner.audio().map(|_| FocAudio::new())
    }

    /// Construct a UART handle only when its observed start entry exists.
    pub fn uart(self) -> Result<Uart, AbiError> {
        self.inner.uart().map(|_| Uart::new())
    }

    /// Construct a settings marker only when its observed getter exists.
    pub fn settings(self) -> Result<FirmwareSettings, AbiError> {
        self.inner.settings().map(|_| FirmwareSettings)
    }

    /// Require CAN for a constructor that cannot operate without it.
    pub fn require_can(self) -> Result<CanBus, AbiError> {
        self.inner.require_can().map(|_| CanBus::new())
    }

    /// Require settings for a constructor that cannot operate without it.
    pub fn require_settings(self) -> Result<FirmwareSettings, AbiError> {
        self.inner.require_settings().map(|_| FirmwareSettings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs_sys::VescIfAbi;

    #[test]
    fn setting_ids_match_the_pinned_cfg_param_enum() {
        let floats = [
            (FirmwareFloatSetting::MotorCurrentMax, 0),
            (FirmwareFloatSetting::MotorCurrentMin, 1),
            (FirmwareFloatSetting::InputCurrentMax, 2),
            (FirmwareFloatSetting::InputCurrentMin, 3),
            (FirmwareFloatSetting::AbsoluteCurrentMax, 4),
            (FirmwareFloatSetting::MinimumElectricalSpeed, 5),
            (FirmwareFloatSetting::MaximumElectricalSpeed, 6),
            (FirmwareFloatSetting::ElectricalSpeedRampStart, 7),
            (FirmwareFloatSetting::MaximumElectricalSpeedBrake, 8),
            (FirmwareFloatSetting::MaximumElectricalSpeedBrakeCurrent, 9),
            (FirmwareFloatSetting::MinimumInputVoltage, 10),
            (FirmwareFloatSetting::MaximumInputVoltage, 11),
            (FirmwareFloatSetting::BatteryCutStartVoltage, 12),
            (FirmwareFloatSetting::BatteryCutEndVoltage, 13),
            (FirmwareFloatSetting::MosfetTemperatureStart, 16),
            (FirmwareFloatSetting::MosfetTemperatureEnd, 17),
            (FirmwareFloatSetting::MotorTemperatureStart, 18),
            (FirmwareFloatSetting::MotorTemperatureEnd, 19),
            (FirmwareFloatSetting::TemperatureAccelerationDecrease, 20),
            (FirmwareFloatSetting::MinDuty, 21),
            (FirmwareFloatSetting::MaxDuty, 22),
            (FirmwareFloatSetting::ImuAccelerationConfidenceDecay, 23),
            (FirmwareFloatSetting::ImuMahonyKp, 24),
            (FirmwareFloatSetting::ImuMahonyKi, 25),
            (FirmwareFloatSetting::ImuMadgwickBeta, 26),
            (FirmwareFloatSetting::ImuRotationRoll, 27),
            (FirmwareFloatSetting::ImuRotationPitch, 28),
            (FirmwareFloatSetting::ImuRotationYaw, 29),
            (FirmwareFloatSetting::ImuSampleRate, 31),
            (FirmwareFloatSetting::ImuAccelerationOffsetX, 32),
            (FirmwareFloatSetting::ImuAccelerationOffsetY, 33),
            (FirmwareFloatSetting::ImuAccelerationOffsetZ, 34),
            (FirmwareFloatSetting::ImuGyroOffsetX, 35),
            (FirmwareFloatSetting::ImuGyroOffsetY, 36),
            (FirmwareFloatSetting::ImuGyroOffsetZ, 37),
            (FirmwareFloatSetting::GearRatio, 40),
            (FirmwareFloatSetting::WheelDiameter, 41),
            (FirmwareFloatSetting::BatteryCapacity, 44),
            (FirmwareFloatSetting::MotorNoLoadCurrent, 45),
            (FirmwareFloatSetting::FocMotorResistance, 46),
            (FirmwareFloatSetting::FocMotorInductance, 47),
            (FirmwareFloatSetting::FocMotorFluxLinkage, 48),
        ];
        assert!(
            floats
                .into_iter()
                .all(|(setting, raw)| setting.raw() == raw)
        );

        let integers = [
            (FirmwareIntSetting::AppCanMode, 14),
            (FirmwareIntSetting::AppCanBaudRate, 15),
            (FirmwareIntSetting::ImuAhrsMode, 30),
            (FirmwareIntSetting::AppShutdownMode, 38),
            (FirmwareIntSetting::MotorPoleCount, 39),
            (FirmwareIntSetting::BatteryType, 42),
            (FirmwareIntSetting::BatteryCellCount, 43),
        ];
        assert!(
            integers
                .into_iter()
                .all(|(setting, raw)| setting.raw() == raw)
        );
    }

    #[test]
    fn safe_capability_constructors_follow_observed_presence() {
        let mut words = [0_usize; VescIfAbi::FIELD_COUNT];
        words[VescIfAbi::CAN_TRANSMIT_SID.slot_index()] = 1;
        words[VescIfAbi::READ_NVM.slot_index()] = 1;
        let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

        assert!(capabilities.can_bus().is_ok());
        assert!(capabilities.nvm().is_ok());
        assert_eq!(
            capabilities
                .nvm_with_capacity(NvmCapacity::new(32).unwrap())
                .unwrap()
                .capacity()
                .unwrap()
                .get(),
            32
        );
        assert_eq!(capabilities.audio().unwrap_err().capability(), "FOC audio");
        assert_eq!(capabilities.uart().unwrap_err().capability(), "UART");
        assert_eq!(
            capabilities.settings().unwrap_err().capability(),
            "settings"
        );
    }

    #[test]
    fn safe_required_constructor_preserves_missing_slot_diagnostics() {
        let capabilities = FirmwareCapabilities::new(VescIfPresence::empty());

        let Err(error) = capabilities.require_can() else {
            panic!("empty presence must reject required CAN")
        };
        assert_eq!(error.capability(), "CAN");
        assert_eq!(error.slot(), VescIfAbi::CAN_TRANSMIT_SID);
        assert_eq!(capabilities.revision(), Stm32AbiRevision::UnknownCompatible);
        assert_eq!(
            capabilities.require_settings().unwrap_err().capability(),
            "settings"
        );
    }
}
