//! Float Out Boy motor output request/apply state.
//!
//! Source map: upstream owns this in `third_party/float-out-boy/src/motor_control.c`
//! and `third_party/float-out-boy/src/motor_control.h`.

use crate::config::FloatOutBoyParkingBrakeMode;
use crate::domain::{FloatOutBoyMotorCommand, FloatOutBoyRunState};
use vescpkg_rs::MotorOutput;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{AudioFrequency, SampleRate};
use vescpkg_rs::prelude::{
    BrakeCurrent, Current, CurrentOffDelay, DutyCycle, MotorCurrent, Rpm, SYSTEM_TICK_RATE_HZ,
    SignedRatio, TimestampTicks, VescSeconds,
};
const CURRENT_OFF_DELAY: CurrentOffDelay = CurrentOffDelay::new(VescSeconds::from_seconds(0.05));

/// Float Out Boy motor-control request state.
///
/// Upstream `MotorControl` stores `current_requested` and `requested_current`
/// at `third_party/float-out-boy/src/motor_control.h:27-30`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FloatOutBoyMotorControl {
    disabled: bool,
    requested_current: Option<FloatOutBoyMotorCommand>,
    // Float Out Boy updates this flag before every idle motor output at
    // `third_party/float-out-boy/src/motor_control.c:66-70`.
    parking_brake_active: bool,
    // Float Out Boy uses `brake_timer` to release idle motor output after one second
    // at `third_party/float-out-boy/src/motor_control.c:101-109`.
    brake_timer_ticks: TimestampTicks,
    tone_ticks: u8,
    tone_counter: u8,
    tone_high: bool,
    tone_intensity: MotorCurrent,
}

impl FloatOutBoyMotorControl {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self {
            disabled: false,
            requested_current: None,
            parking_brake_active: false,
            brake_timer_ticks: TimestampTicks::from_ticks(0),
            tone_ticks: 0,
            tone_counter: 0,
            tone_high: false,
            tone_intensity: MotorCurrent::new(Current::ZERO),
        }
    }

    #[inline(always)]
    pub(crate) fn request_current(&mut self, current: MotorCurrent) {
        // Upstream `motor_control_request_current` sets the request flag and
        // stores the requested current at `third_party/float-out-boy/src/motor_control.c:44-47`.
        self.requested_current = Some(FloatOutBoyMotorCommand::new(current));
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn play_tone(
        &mut self,
        frequency: AudioFrequency,
        intensity: MotorCurrent,
        sample_rate: SampleRate,
    ) {
        let ticks = (sample_rate.as_hertz() / 2.0 / frequency.frequency().as_hertz())
            .max(1.0)
            .min(f32::from(u8::MAX)) as u8;
        if ticks != self.tone_ticks {
            self.tone_ticks = ticks;
            self.tone_counter = ticks;
        }
        self.tone_intensity = intensity;
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn stop_tone(&mut self) {
        self.tone_ticks = 0;
        self.tone_counter = 0;
        self.tone_high = false;
    }

    #[inline(always)]
    pub(crate) fn apply_requested_current(&mut self, motor: &impl MotorOutput) -> bool {
        self.requested_current.take().is_some_and(|command| {
            // Upstream keeps this sign unchanged: `motor_control_request_current`
            // stores it at `third_party/float-out-boy/src/motor_control.c:44-47`, then
            // `motor_control_apply` passes it to `mc_set_current` at
            // `third_party/float-out-boy/src/motor_control.c:93-99`.
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
        run_state: FloatOutBoyRunState,
        abs_erpm: Rpm,
        system_time_ticks: TimestampTicks,
        parking_brake_mode: FloatOutBoyParkingBrakeMode,
        brake_current: MotorCurrent,
    ) -> bool {
        if matches!(run_state, FloatOutBoyRunState::Disabled) {
            if !self.disabled {
                // C map: disabled mode sets 0A once, then stops touching motor output at
                // `third_party/float-out-boy/src/motor_control.c:53-60`.
                motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
                self.disabled = true;
                return true;
            }
            return false;
        }

        self.disabled = false;
        // Upstream updates `parking_brake_active` before idle output at
        // `third_party/float-out-boy/src/motor_control.c:66-70`; enum values come from
        // `third_party/float-out-boy/src/conf/datatypes.h:31-33`.
        let parking_brake_was_active = self.parking_brake_active;
        if matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Always)
            || matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Idle)
                && !matches!(run_state, FloatOutBoyRunState::Running)
                && abs_erpm < Rpm::from_revolutions_per_minute(50.0)
        {
            self.parking_brake_active = true;
        } else if matches!(parking_brake_mode, FloatOutBoyParkingBrakeMode::Never)
            || matches!(run_state, FloatOutBoyRunState::Running)
        {
            self.parking_brake_active = false;
        }
        if self.parking_brake_active && !parking_brake_was_active {
            // C map: upstream initializes `brake_timer` against package-local
            // time at `third_party/float-out-boy/src/motor_control.c:29`, so the
            // first idle apply does not immediately trip `timer_older` at
            // `third_party/float-out-boy/src/motor_control.c:106`.
            self.brake_timer_ticks = system_time_ticks;
        }

        if self.tone_ticks > 0 {
            self.tone_counter -= 1;
            if self.tone_counter == 0 {
                self.tone_counter = self.tone_ticks;
                self.tone_high = !self.tone_high;
            }
            let requested = self.requested_current.map_or(Current::ZERO, |command| {
                command.requested_current().current()
            });
            let tone = self.tone_intensity.current();
            self.request_current(MotorCurrent::new(if self.tone_high {
                requested + tone
            } else {
                requested - tone
            }));
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
            // `third_party/float-out-boy/src/motor_control.c:101-109`; `timer_older`
            // converts seconds at `third_party/float-out-boy/src/time.h:46-51`.
            motor.set_current(MotorCurrent::new(Current::from_amps(0.0)));
        } else if self.parking_brake_active && abs_erpm < Rpm::from_revolutions_per_minute(2000.0) {
            // Upstream parking brake applies duty zero below 2000 ERPM at
            // `third_party/float-out-boy/src/motor_control.c:112-114`.
            motor.set_duty_cycle(DutyCycle::new(SignedRatio::from_ratio_const(0.0)));
        } else {
            // Upstream idle fallback applies configured brake current at
            // `third_party/float-out-boy/src/motor_control.c:115-117`.
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
    fn motor_control_sets_zero_once_while_disabled_like_float_out_boy() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = FloatOutBoyMotorControl::new();

        assert!(control.apply(
            bindings,
            FloatOutBoyRunState::Disabled,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.current_command_count(), 1);
        assert_eq!(motor.commanded_current().current().as_amps(), 0.0);

        assert!(!control.apply(
            bindings,
            FloatOutBoyRunState::Disabled,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.current_command_count(), 1);
    }

    #[test]
    fn motor_control_applies_ready_parking_brake_like_float_out_boy() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = FloatOutBoyMotorControl::new();

        assert!(control.apply(
            bindings,
            FloatOutBoyRunState::Ready,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));

        // Upstream `motor_control_apply` resets timeout at
        // `third_party/float-out-boy/src/motor_control.c:92-93`, activates default
        // `PARKING_BRAKE_IDLE` at `third_party/float-out-boy/src/motor_control.c:66-70`,
        // and applies duty zero while stopped at
        // `third_party/float-out-boy/src/motor_control.c:112-114`.
        assert_eq!(motor.keep_alive_count(), 1);
        assert_eq!(motor.duty_command_count(), 1);
        assert_eq!(motor.commanded_duty().ratio().as_ratio(), 0.0);
        assert_eq!(motor.current_command_count(), 0);
        assert_eq!(motor.brake_current_command_count(), 0);
    }

    #[test]
    fn motor_control_seeds_idle_brake_timer_on_ready_entry_like_float_out_boy() {
        let motor = FirmwareTest::new();
        let bindings = motor.motor();
        let mut control = FloatOutBoyMotorControl::new();

        assert!(control.apply(
            bindings,
            FloatOutBoyRunState::Ready,
            Rpm::ZERO,
            TimestampTicks::from_ticks(20_000),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));

        // C map: Float Out Boy initializes `brake_timer` with package-local time at
        // `third_party/float-out-boy/src/motor_control.c:29`, then lets stopped idle
        // output hold the parking brake until `timer_older(..., 1)` at
        // `third_party/float-out-boy/src/motor_control.c:101-114`.
        assert_eq!(motor.keep_alive_count(), 1);
        assert_eq!(motor.duty_command_count(), 1);
        assert_eq!(motor.current_command_count(), 0);
        assert_eq!(motor.brake_current_command_count(), 0);
    }

    #[test]
    fn motor_control_modulates_requested_current_for_vibration_like_float_out_boy() {
        let motor = FirmwareTest::new();
        let mut control = FloatOutBoyMotorControl::new();
        control.play_tone(
            AudioFrequency::new(vescpkg_rs::Frequency::from_hertz(70.0)),
            MotorCurrent::new(Current::from_amps(2.0)),
            SampleRate::from_hertz(832.0),
        );

        for _ in 0..4 {
            control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
            assert!(control.apply(
                motor.motor(),
                FloatOutBoyRunState::Running,
                Rpm::ZERO,
                TimestampTicks::from_ticks(0),
                FloatOutBoyParkingBrakeMode::Idle,
                MotorCurrent::new(Current::from_amps(50.0)),
            ));
            assert_eq!(motor.commanded_current().current().as_amps(), 3.0);
        }

        control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
        assert!(control.apply(
            motor.motor(),
            FloatOutBoyRunState::Running,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.commanded_current().current().as_amps(), 7.0);

        control.stop_tone();
        control.request_current(MotorCurrent::new(Current::from_amps(5.0)));
        assert!(control.apply(
            motor.motor(),
            FloatOutBoyRunState::Running,
            Rpm::ZERO,
            TimestampTicks::from_ticks(0),
            FloatOutBoyParkingBrakeMode::Idle,
            MotorCurrent::new(Current::from_amps(50.0)),
        ));
        assert_eq!(motor.commanded_current().current().as_amps(), 5.0);
    }
}
