//! Capability-aware safe subsystem constructors.

use crate::ffi;
use crate::{BatteryCellCount, CanBus, FocAudio, Nvm, NvmCapacity, Uart};
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
        }
    }
}

/// An integer firmware setting exposed by the pinned VESC ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareIntSetting {
    /// Battery cell count (`CFG_PARAM_si_battery_cells`).
    BatteryCellCount,
}

impl FirmwareIntSetting {
    const fn raw(self) -> i32 {
        match self {
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
