//! Typed controller input and output-safety capabilities.

use crate::types::{JoystickX, JoystickY, PpmAge, PpmInput, RemoteAge, TimeoutDuration};
use crate::{SignedRatio, VescSeconds};
use core::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_INHIBIT_LIVE: AtomicBool = AtomicBool::new(false);

/// Failure returned when an input/safety capability is unavailable or rejects a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// The firmware table does not expose this optional capability.
    Unsupported,
    /// Firmware exposed the capability but rejected the operation.
    FirmwareRejected,
    /// Firmware returned a value outside the semantic input range.
    InvalidValue,
    /// Another package-owned shutdown inhibition guard is active.
    Busy,
}

impl core::fmt::Display for InputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "firmware does not expose this input capability",
            Self::FirmwareRejected => "firmware rejected the input capability operation",
            Self::InvalidValue => "firmware returned an invalid input value",
            Self::Busy => "another shutdown inhibition guard is active",
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

/// Snapshot of the firmware motor-command timeout state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeoutSnapshot {
    timed_out: bool,
    since_update: TimeoutDuration,
}

impl TimeoutSnapshot {
    /// Return whether the firmware timeout is active.
    pub const fn has_timed_out(self) -> bool {
        self.timed_out
    }

    /// Return the elapsed time since the last timeout refresh.
    pub const fn since_update(self) -> TimeoutDuration {
        self.since_update
    }
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

/// RAII guard that keeps firmware automatic shutdown inhibited.
pub struct ShutdownInhibit;

impl Drop for ShutdownInhibit {
    fn drop(&mut self) {
        // Keep the guard's ownership bit if firmware cannot restore shutdown.
        if unsafe { crate::ffi::shutdown_disable(false) }.is_some() {
            SHUTDOWN_INHIBIT_LIVE.store(false, Ordering::Release);
        }
    }
}

impl FirmwareInputs {
    /// Construct the firmware-backed input capability.
    pub const fn new() -> Self {
        Self
    }

    /// Copy the remote-control state before returning it to package code.
    pub fn remote(&self) -> Result<RemoteInputSnapshot, InputError> {
        let raw = unsafe { crate::ffi::remote_state() };
        Ok(RemoteInputSnapshot {
            joystick_x: JoystickX::new(decode_ratio(raw.js_x)?),
            joystick_y: JoystickY::new(decode_ratio(raw.js_y)?),
            bluetooth_connected: raw.bt_c,
            bluetooth_z: raw.bt_z,
            reverse: raw.is_rev,
            age: RemoteAge::new(VescSeconds::from_seconds(raw.age_s)),
        })
    }

    /// Read and copy decoded PPM input and its firmware-reported age.
    pub fn ppm(&self) -> Result<PpmSnapshot, InputError> {
        let value = unsafe { crate::ffi::get_ppm() }.ok_or(InputError::Unsupported)?;
        let age = unsafe { crate::ffi::get_ppm_age() }.ok_or(InputError::Unsupported)?;
        Ok(PpmSnapshot {
            value: PpmInput::new(decode_ratio(value)?),
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

    /// Read the firmware motor-command timeout state.
    pub fn timeout(&self) -> TimeoutSnapshot {
        TimeoutSnapshot {
            timed_out: unsafe { crate::ffi::timeout_has_timeout() },
            since_update: TimeoutDuration::new(VescSeconds::from_seconds(unsafe {
                crate::ffi::timeout_secs_since_update()
            })),
        }
    }

    /// Refresh the firmware motor-command timeout.
    pub fn reset_timeout(&self) {
        unsafe { crate::ffi::timeout_reset() }
    }

    /// Inhibit firmware automatic shutdown until the guard is dropped.
    pub fn inhibit_shutdown(&self) -> Result<ShutdownInhibit, InputError> {
        SHUTDOWN_INHIBIT_LIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| InputError::Busy)?;
        if unsafe { crate::ffi::shutdown_disable(true) }.is_none() {
            SHUTDOWN_INHIBIT_LIVE.store(false, Ordering::Release);
            return Err(InputError::Unsupported);
        }
        Ok(ShutdownInhibit)
    }
}

fn decode_ratio(value: f32) -> Result<SignedRatio, InputError> {
    SignedRatio::from_ratio(value).map_err(|_| InputError::InvalidValue)
}

#[cfg(test)]
mod tests {
    use super::{InputError, decode_ratio};

    #[test]
    fn ratio_decode_rejects_values_outside_the_input_domain() {
        assert_eq!(decode_ratio(2.0), Err(InputError::InvalidValue));
    }
}
