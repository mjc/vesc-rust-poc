//! Refloat motor output request/apply state.
//!
//! Source map: upstream owns this in `third_party/refloat/src/motor_control.c`
//! and `third_party/refloat/src/motor_control.h`.

use crate::domain::{RefloatMotorCommand, RefloatRunState};
use vescpkg_rs::prelude::{Current, MotorCurrent, SignedRatio};
use vescpkg_rs::{MotorControlApi, MotorControlBindings};

/// Refloat motor-control request state.
///
/// Upstream `MotorControl` stores `current_requested` and `requested_current`
/// at `third_party/refloat/src/motor_control.h:27-30`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatMotorControl {
    disabled: bool,
    requested_current: Option<RefloatMotorCommand>,
    // Refloat updates this flag before every idle motor output at
    // `third_party/refloat/src/motor_control.c:66-70`.
    parking_brake_active: bool,
    // Refloat uses `brake_timer` to release idle motor output after one second
    // at `third_party/refloat/src/motor_control.c:101-109`.
    brake_timer_ticks: u32,
}

impl RefloatMotorControl {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self {
            disabled: false,
            requested_current: None,
            parking_brake_active: false,
            brake_timer_ticks: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn request_current(&mut self, current: MotorCurrent) {
        // Upstream `motor_control_request_current` sets the request flag and
        // stores the requested current at `third_party/refloat/src/motor_control.c:44-47`.
        self.requested_current = Some(RefloatMotorCommand::new(current));
    }

    #[inline(always)]
    pub(crate) fn apply_requested_current<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
    ) -> bool {
        self.requested_current.take().is_some_and(|command| {
            // Upstream keeps this sign unchanged: `motor_control_request_current`
            // stores it at `third_party/refloat/src/motor_control.c:44-47`, then
            // `motor_control_apply` passes it to `mc_set_current` at
            // `third_party/refloat/src/motor_control.c:93-99`.
            motor.timeout_reset();
            motor.set_current_off_delay(0.05);
            motor.set_current(command.requested_current());
            true
        })
    }

    #[inline(always)]
    pub(crate) fn apply<B: MotorControlBindings>(
        &mut self,
        motor: &MotorControlApi<B>,
        run_state: RefloatRunState,
        abs_erpm: f32,
        system_time_ticks: u32,
        parking_brake_mode: u8,
        brake_current: MotorCurrent,
    ) -> bool {
        if matches!(run_state, RefloatRunState::Disabled) {
            if !self.disabled {
                motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
                self.disabled = true;
                return true;
            }
            return false;
        }

        self.disabled = false;
        // Upstream updates `parking_brake_active` before idle output at
        // `third_party/refloat/src/motor_control.c:66-70`; enum values come from
        // `third_party/refloat/src/conf/datatypes.h:29-33`.
        if parking_brake_mode == 0
            || parking_brake_mode == 1
                && !matches!(run_state, RefloatRunState::Running)
                && abs_erpm < 50.0
        {
            self.parking_brake_active = true;
        } else if parking_brake_mode == 2 || matches!(run_state, RefloatRunState::Running) {
            self.parking_brake_active = false;
        }

        if self.apply_requested_current(motor) {
            return true;
        }

        motor.timeout_reset();
        if abs_erpm > 200.0 {
            self.brake_timer_ticks = system_time_ticks;
        }
        if system_time_ticks.wrapping_sub(self.brake_timer_ticks) > 10_000 {
            // Upstream releases idle motor output by setting 0A once
            // `timer_older(time, brake_timer, 1)` passes at
            // `third_party/refloat/src/motor_control.c:101-109`; `timer_older`
            // converts seconds at `third_party/refloat/src/time.h:46-51`.
            motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
        } else if self.parking_brake_active && abs_erpm < 2000.0 {
            // Upstream parking brake applies duty zero below 2000 ERPM at
            // `third_party/refloat/src/motor_control.c:112-114`.
            motor.set_duty(SignedRatio::from_ratio_const(0.0));
        } else {
            // Upstream idle fallback applies configured brake current at
            // `third_party/refloat/src/motor_control.c:115-117`.
            motor.set_brake_current(brake_current);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::test_support::FakeMotorControlBindings;

    #[test]
    fn motor_control_sets_zero_once_while_disabled_like_refloat() {
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut control = RefloatMotorControl::new();

        assert!(control.apply(
            &motor,
            RefloatRunState::Disabled,
            0.0,
            0,
            1,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
        assert_eq!(motor.bindings().current().current().as_amps(), 0.0);

        assert!(!control.apply(
            &motor,
            RefloatRunState::Disabled,
            0.0,
            0,
            1,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.bindings().set_current_calls.get(), 1);
    }

    #[test]
    fn motor_control_applies_ready_parking_brake_like_refloat() {
        let motor = MotorControlApi::new(FakeMotorControlBindings::new());
        let mut control = RefloatMotorControl::new();

        assert!(control.apply(
            &motor,
            RefloatRunState::Ready,
            0.0,
            0,
            1,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));

        // Upstream `motor_control_apply` resets timeout at
        // `third_party/refloat/src/motor_control.c:92-93`, activates default
        // `PARKING_BRAKE_IDLE` at `third_party/refloat/src/motor_control.c:66-70`,
        // and applies duty zero while stopped at
        // `third_party/refloat/src/motor_control.c:112-114`.
        assert_eq!(motor.bindings().timeout_reset_calls.get(), 1);
        assert_eq!(motor.bindings().set_duty_calls.get(), 1);
        assert_eq!(motor.bindings().duty().as_ratio(), 0.0);
        assert_eq!(motor.bindings().set_current_calls.get(), 0);
        assert_eq!(motor.bindings().set_brake_current_calls.get(), 0);
    }
}
