//! Typed controller input and output-safety capabilities.

use crate::types::{JoystickX, JoystickY, PpmAge, PpmInput, RemoteAge};
use crate::{SignedRatio, VescSeconds};

/// Failure returned when an input/safety capability is unavailable or rejects a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// The firmware table does not expose this optional capability.
    Unsupported,
    /// Firmware exposed the capability but rejected the operation.
    FirmwareRejected,
}

impl core::fmt::Display for InputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "firmware does not expose this input capability",
            Self::FirmwareRejected => "firmware rejected the input capability operation",
        })
    }
}

impl core::error::Error for InputError {}

/// Owned snapshot of the firmware remote-control state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemoteInputSnapshot {
    joystick_x: JoystickX,
    joystick_y: JoystickY,
    bluetooth_connected: bool,
    bluetooth_z: bool,
    reverse: bool,
    age: RemoteAge,
}

impl RemoteInputSnapshot {
    /// Return the joystick X ratio.
    pub const fn joystick_x(self) -> JoystickX {
        self.joystick_x
    }

    /// Return the joystick Y ratio.
    pub const fn joystick_y(self) -> JoystickY {
        self.joystick_y
    }

    /// Return whether Bluetooth input is connected.
    pub const fn bluetooth_connected(self) -> bool {
        self.bluetooth_connected
    }

    /// Return the Bluetooth Z-button state.
    pub const fn bluetooth_z(self) -> bool {
        self.bluetooth_z
    }

    /// Return whether reverse is selected.
    pub const fn reverse(self) -> bool {
        self.reverse
    }

    /// Return the age of this remote sample.
    pub const fn age(self) -> RemoteAge {
        self.age
    }
}

/// Owned snapshot of decoded PPM input and its sample age.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PpmSnapshot {
    value: PpmInput,
    age: PpmAge,
}

impl PpmSnapshot {
    /// Return the decoded PPM ratio.
    pub const fn value(self) -> PpmInput {
        self.value
    }

    /// Return the age of the latest PPM sample.
    pub const fn age(self) -> PpmAge {
        self.age
    }
}

/// Firmware controller-input and output-safety capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct FirmwareInputs;

impl FirmwareInputs {
    /// Construct the firmware-backed input capability.
    pub const fn new() -> Self {
        Self
    }

    /// Copy the remote-control state before returning it to package code.
    pub fn remote(&self) -> RemoteInputSnapshot {
        let raw = unsafe { crate::ffi::remote_state() };
        RemoteInputSnapshot {
            joystick_x: JoystickX::new(
                SignedRatio::from_ratio(raw.js_x).unwrap_or(SignedRatio::from_ratio_const(0.0)),
            ),
            joystick_y: JoystickY::new(
                SignedRatio::from_ratio(raw.js_y).unwrap_or(SignedRatio::from_ratio_const(0.0)),
            ),
            bluetooth_connected: raw.bt_c,
            bluetooth_z: raw.bt_z,
            reverse: raw.is_rev,
            age: RemoteAge::new(VescSeconds::from_seconds(raw.age_s)),
        }
    }

    /// Read and copy decoded PPM input and its firmware-reported age.
    pub fn ppm(&self) -> Result<PpmSnapshot, InputError> {
        let value = unsafe { crate::ffi::get_ppm() }.ok_or(InputError::Unsupported)?;
        let age = unsafe { crate::ffi::get_ppm_age() }.ok_or(InputError::Unsupported)?;
        Ok(PpmSnapshot {
            value: PpmInput::new(
                SignedRatio::from_ratio(value).unwrap_or(SignedRatio::from_ratio_const(0.0)),
            ),
            age: PpmAge::new(VescSeconds::from_seconds(age)),
        })
    }

    /// Return whether firmware currently asks applications to disable output.
    pub fn output_disabled(&self) -> Result<bool, InputError> {
        unsafe { crate::ffi::app_is_output_disabled() }.ok_or(InputError::Unsupported)
    }

    /// Persist firmware backup data explicitly.
    pub fn store_backup(&self) -> Result<(), InputError> {
        match unsafe { crate::ffi::store_backup_data() } {
            None => Err(InputError::Unsupported),
            Some(true) => Ok(()),
            Some(false) => Err(InputError::FirmwareRejected),
        }
    }
}
