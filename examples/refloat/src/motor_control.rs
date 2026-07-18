//! Refloat motor output request/apply state.
//!
//! Source map: upstream owns this in `third_party/refloat/src/motor_control.c`
//! and `third_party/refloat/src/motor_control.h`.

use crate::config::RefloatParkingBrakeMode;
use crate::domain::{RefloatMotorCommand, RefloatRunState};
use vescpkg_rs::MotorOutput;
use vescpkg_rs::prelude::{
    BrakeCurrent, Current, CurrentOffDelay, DutyCycle, MotorCurrent, Rpm, SYSTEM_TICK_RATE_HZ,
    SignedRatio, TimestampTicks, VescSeconds,
};
const CURRENT_OFF_DELAY: CurrentOffDelay = CurrentOffDelay::new(VescSeconds::from_seconds(0.05));

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
    brake_timer_ticks: TimestampTicks,
}

impl RefloatMotorControl {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self {
            disabled: false,
            requested_current: None,
            parking_brake_active: false,
            brake_timer_ticks: TimestampTicks::from_ticks(0),
        }
    }

    #[inline(always)]
    pub(crate) fn request_current(&mut self, current: MotorCurrent) {
        // Upstream `motor_control_request_current` sets the request flag and
        // stores the requested current at `third_party/refloat/src/motor_control.c:44-47`.
        self.requested_current = Some(RefloatMotorCommand::new(current));
    }

    #[inline(always)]
    pub(crate) fn apply_requested_current(&mut self, motor: &impl MotorOutput) -> bool {
        self.requested_current.take().is_some_and(|command| {
            // Upstream keeps this sign unchanged: `motor_control_request_current`
            // stores it at `third_party/refloat/src/motor_control.c:44-47`, then
            // `motor_control_apply` passes it to `mc_set_current` at
            // `third_party/refloat/src/motor_control.c:93-99`.
            motor.keep_alive();
            motor.set_current_off_delay(CURRENT_OFF_DELAY);
            motor.set_current(command.requested_current());
            true
        })
    }

    #[inline(always)]
    pub(crate) fn apply(
        &mut self,
        motor: &impl MotorOutput,
        run_state: RefloatRunState,
        abs_erpm: Rpm,
        system_time_ticks: TimestampTicks,
        parking_brake_mode: RefloatParkingBrakeMode,
        brake_current: MotorCurrent,
    ) -> bool {
        if matches!(run_state, RefloatRunState::Disabled) {
            if !self.disabled {
                // C map: disabled mode sets 0A once, then stops touching motor output at
                // `third_party/refloat/src/motor_control.c:53-60`.
                motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
                self.disabled = true;
                return true;
            }
            return false;
        }

        self.disabled = false;
        // Upstream updates `parking_brake_active` before idle output at
        // `third_party/refloat/src/motor_control.c:66-70`; enum values come from
        // `third_party/refloat/src/conf/datatypes.h:31-33`.
        let parking_brake_was_active = self.parking_brake_active;
        if matches!(parking_brake_mode, RefloatParkingBrakeMode::Always)
            || matches!(parking_brake_mode, RefloatParkingBrakeMode::Idle)
                && !matches!(run_state, RefloatRunState::Running)
                && abs_erpm < Rpm::from_revolutions_per_minute(50.0)
        {
            self.parking_brake_active = true;
        } else if matches!(parking_brake_mode, RefloatParkingBrakeMode::Never)
            || matches!(run_state, RefloatRunState::Running)
        {
            self.parking_brake_active = false;
        }
        if self.parking_brake_active && !parking_brake_was_active {
            // C map: upstream initializes `brake_timer` against package-local
            // time at `third_party/refloat/src/motor_control.c:29`, so the
            // first idle apply does not immediately trip `timer_older` at
            // `third_party/refloat/src/motor_control.c:106`.
            self.brake_timer_ticks = system_time_ticks;
        }

        if self.apply_requested_current(motor) {
            return true;
        }

        motor.keep_alive();
        if abs_erpm > Rpm::from_revolutions_per_minute(200.0) {
            self.brake_timer_ticks = system_time_ticks;
        }
        if system_time_ticks
            .wrapping_duration_since(self.brake_timer_ticks)
            .as_ticks()
            > SYSTEM_TICK_RATE_HZ as u32
        {
            // Upstream releases idle motor output by setting 0A once
            // `timer_older(time, brake_timer, 1)` passes at
            // `third_party/refloat/src/motor_control.c:101-109`; `timer_older`
            // converts seconds at `third_party/refloat/src/time.h:46-51`.
            motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
        } else if self.parking_brake_active && abs_erpm < Rpm::from_revolutions_per_minute(2000.0) {
            // Upstream parking brake applies duty zero below 2000 ERPM at
            // `third_party/refloat/src/motor_control.c:112-114`.
            motor.set_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0)));
        } else {
            // Upstream idle fallback applies configured brake current at
            // `third_party/refloat/src/motor_control.c:115-117`.
            motor.set_brake_current(BrakeCurrent::new(brake_current.current()));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn motor_control_sets_zero_once_while_disabled_like_refloat() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = RefloatMotorControl::new();

        assert!(control.apply(
            bindings,
            RefloatRunState::Disabled,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            RefloatParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.current_command_count(), 1);
        assert_eq!(motor.commanded_current().current().as_amps(), 0.0);

        assert!(!control.apply(
            bindings,
            RefloatRunState::Disabled,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            RefloatParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.current_command_count(), 1);
    }

    #[test]
    fn motor_control_applies_ready_parking_brake_like_refloat() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = RefloatMotorControl::new();

        assert!(control.apply(
            bindings,
            RefloatRunState::Ready,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            RefloatParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));

        // Upstream `motor_control_apply` resets timeout at
        // `third_party/refloat/src/motor_control.c:92-93`, activates default
        // `PARKING_BRAKE_IDLE` at `third_party/refloat/src/motor_control.c:66-70`,
        // and applies duty zero while stopped at
        // `third_party/refloat/src/motor_control.c:112-114`.
        assert_eq!(motor.keep_alive_count(), 1);
        assert_eq!(motor.duty_command_count(), 1);
        assert_eq!(motor.commanded_duty().ratio().as_ratio(), 0.0);
        assert_eq!(motor.current_command_count(), 0);
        assert_eq!(motor.brake_current_command_count(), 0);
    }

    #[test]
    fn motor_control_seeds_idle_brake_timer_on_ready_entry_like_refloat() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = RefloatMotorControl::new();

        assert!(control.apply(
            bindings,
            RefloatRunState::Ready,
            Rpm::ZERO,
            TimestampTicks::from_ticks(20_000),
            RefloatParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));

        // C map: Refloat initializes `brake_timer` with package-local time at
        // `third_party/refloat/src/motor_control.c:29`, then lets stopped idle
        // output hold the parking brake until `timer_older(..., 1)` at
        // `third_party/refloat/src/motor_control.c:101-114`.
        assert_eq!(motor.keep_alive_count(), 1);
        assert_eq!(motor.duty_command_count(), 1);
        assert_eq!(motor.current_command_count(), 0);
        assert_eq!(motor.brake_current_command_count(), 0);
    }
}
