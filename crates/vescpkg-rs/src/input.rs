//! Typed controller-input access backed by VESC PPM and UART slots.

#![allow(deprecated)]

use crate::{JoystickY, PpmAge, PpmInput, RemoteAge, SignedRatio, VescSeconds};

/// Latest UART remote input used by package control loops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemoteInput {
    joystick_y: JoystickY,
    age: RemoteAge,
}

impl RemoteInput {
    /// Build a typed UART remote sample.
    #[must_use]
    pub const fn new(joystick_y: JoystickY, age: RemoteAge) -> Self {
        Self { joystick_y, age }
    }

    /// Return the vertical joystick axis.
    #[must_use]
    pub const fn joystick_y(self) -> JoystickY {
        self.joystick_y
    }

    /// Return the sample age.
    #[must_use]
    pub const fn age(self) -> RemoteAge {
        self.age
    }
}

/// Legacy firmware controller-input capability.
///
/// Use [`crate::FirmwareInputs`] for new code. This compatibility wrapper
/// predates the capability-aware API and intentionally preserves its fallback
/// return shapes for existing package ports.
#[deprecated(note = "use FirmwareInputs for capability-aware input access")]
#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerInput;

fn firmware_ratio(value: f32) -> SignedRatio {
    if value.is_finite() {
        SignedRatio::clamped(value)
    } else {
        SignedRatio::from_ratio_const(0.0)
    }
}

impl ControllerInput {
    #[cfg(not(test))]
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Return the latest decoded PPM input and its age.
    #[must_use]
    pub fn ppm(&self) -> (PpmInput, PpmAge) {
        // C map: Float Out Boy reads these VESC slots in
        // `third_party/float-out-boy/src/remote.c:39-42`.
        let input = call_vesc_ffi!(get_ppm()).zip(call_vesc_ffi!(get_ppm_age()));
        let (value, age) = input.unwrap_or((0.0, f32::INFINITY));
        (
            PpmInput::new(firmware_ratio(value)),
            PpmAge::new(VescSeconds::from_seconds(age.max(0.0))),
        )
    }

    /// Return the latest UART remote Y input and its age.
    #[must_use]
    pub fn remote(&self) -> RemoteInput {
        // C map: Float Out Boy reads the remote-state slot in
        // `third_party/float-out-boy/src/remote.c:43-48`.
        #[cfg(test)]
        let (joystick_y, age) = {
            let remote = call_vesc_ffi!(remote_state());
            (remote.js_y, remote.age_s)
        };
        #[cfg(not(test))]
        let (joystick_y, age) = call_vesc_ffi!(remote_state())
            .map_or((0.0, f32::INFINITY), |remote| (remote.js_y, remote.age_s));
        RemoteInput::new(
            JoystickY::new(firmware_ratio(joystick_y)),
            RemoteAge::new(VescSeconds::from_seconds(age.max(0.0))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::firmware_ratio;

    #[test]
    fn firmware_ratio_clamps_and_normalizes_non_finite_input() {
        assert_f32_eq!(firmware_ratio(2.0).as_ratio(), 1.0);
        assert_f32_eq!(firmware_ratio(-2.0).as_ratio(), -1.0);
        assert_f32_eq!(firmware_ratio(f32::NAN).as_ratio(), 0.0);
    }
}
