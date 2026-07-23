use crate::config::FloatOutBoyRemoteThrottleConfig;
use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand, FloatOutBoyRunState};
use crate::package::state::float_out_boy_command_payload;
use crate::package::time::float_out_boy_ticks_elapsed_seconds;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, Current, MotorCurrent, Ratio, Rpm, SampleRate, TimestampTicks,
};

fn zero_motor_current() -> MotorCurrent {
    // C map: `reset_runtime_vars` and the RC-move idle branches clear current
    // by writing zero at `third_party/float-out-boy/src/main.c:239-252` and
    // `third_party/float-out-boy/src/main.c:291-298`.
    MotorCurrent::new(Current::ZERO)
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct CurrentSmoothing(Ratio);

impl CurrentSmoothing {
    // C map: `do_rc_move` filters remote current as old * 0.95 + target * 0.05
    // at `third_party/float-out-boy/src/main.c:275-286` and `third_party/float-out-boy/src/main.c:291-298`.
    const REMOTE_CURRENT_FILTER: Self = Self(Ratio::from_ratio_const(0.05));

    #[inline(always)]
    const fn retain_previous(self) -> Ratio {
        // C map: `do_rc_move` keeps the previous RC current with 95% weight.
        Ratio::from_ratio_const(1.0 - self.0.as_ratio())
    }

    #[inline(always)]
    const fn accept_target(self) -> Ratio {
        // C map: `do_rc_move` keeps the new RC target with 5% weight.
        self.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct RemoteCurrentTarget(i16);

impl RemoteCurrentTarget {
    const ZERO: Self = Self(0);

    const fn new(deciamps: i16) -> Self {
        Self(deciamps)
    }

    #[cfg(test)]
    const fn deciamps(self) -> i16 {
        self.0
    }

    fn motor_current(self) -> MotorCurrent {
        // C map: `cmd_rc_move` stores packet current as deciamps at
        // `third_party/float-out-boy/src/main.c:1747-1756`; `do_rc_move` requests amps.
        MotorCurrent::new(Current::from_amps(f32::from(self.0) * 0.1))
    }

    const fn is_zero(self) -> bool {
        // C map: `cmd_rc_move` treats zero target current as the idle step.
        self.0 == 0
    }

    const fn exceeds_packet_limit(self) -> bool {
        // C map: `cmd_rc_move` clamps packet targets above 20 deciamps.
        self.0 > 80
    }

    const fn should_halve_mid_move(self) -> bool {
        // C map: `do_rc_move` halves targets above 2A after 500 steps.
        self.0 > 20
    }

    fn halve(&mut self) {
        // C map: `do_rc_move` halves large RC moves after 500 steps at
        // `third_party/float-out-boy/src/main.c:281-284`.
        self.0 /= 2;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RemoteMove {
    target: RemoteCurrentTarget,
    duration_steps: u16,
}

impl RemoteMove {
    const ZERO_CURRENT_STEP: Self = Self {
        target: RemoteCurrentTarget::ZERO,
        duration_steps: 1,
    };

    pub(super) fn from_float_out_boy_command(
        direction: u8,
        current: u8,
        time: u8,
        sum: u8,
    ) -> Self {
        // C map: `cmd_rc_move` treats checksum failure as `current = 0`, then
        // stores READY-state RC move fields at `third_party/float-out-boy/src/main.c:1735-1758`.
        let current = if u16::from(sum) == u16::from(time) + u16::from(current) {
            current
        } else {
            0
        };

        let target = match direction {
            0 => RemoteCurrentTarget::new(-i16::from(current)),
            _ => RemoteCurrentTarget::new(i16::from(current)),
        };

        Self::new(target, time)
    }

    fn new(target: RemoteCurrentTarget, time: u8) -> Self {
        // C map: `cmd_rc_move` keeps zero requests idle, clamps oversized
        // targets, and stores duration as `time * 100` at
        // `third_party/float-out-boy/src/main.c:1735-1758`.
        match target {
            target if target.is_zero() => Self::ZERO_CURRENT_STEP,
            target if target.exceeds_packet_limit() => Self {
                // C map: oversized positive targets are clamped to 20 deciamps
                // at `third_party/float-out-boy/src/main.c:1753-1757`.
                target: RemoteCurrentTarget::new(20),
                duration_steps: u16::from(time) * 100,
            },
            target => Self {
                target,
                duration_steps: u16::from(time) * 100,
            },
        }
    }
}

pub(super) fn handle_packet(
    all_data_payloads: FloatOutBoyAllDataPayloads,
    remote_control: &mut RemoteControlState,
    bytes: &[u8],
) -> bool {
    // C map: `on_command_received` dispatches COMMAND_RC_MOVE only for
    // six-byte packets at `third_party/float-out-boy/src/main.c:2186-2192`; `cmd_rc_move`
    // mutates RC move state only while READY at `third_party/float-out-boy/src/main.c:1735-1758`.
    match float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::RcMove) {
        Some([direction, current, time, sum]) => {
            if all_data_payloads.base().status().ride_state().run_state()
                == FloatOutBoyRunState::Ready
            {
                remote_control.queue_move(RemoteMove::from_float_out_boy_command(
                    *direction, *current, *time, *sum,
                ));
            }
            true
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RemoteControlState {
    input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    tilt_ramped_step: AngleDegrees,
    tilt_setpoint: AngleDegrees,
    current: MotorCurrent,
    steps: u16,
    counter: u16,
    target: RemoteCurrentTarget,
}

impl Default for RemoteControlState {
    fn default() -> Self {
        // C map: Float Out Boy resets RC move state and current to zero at
        // `third_party/float-out-boy/src/main.c:239-252`.
        Self {
            input: crate::domain::FloatOutBoyRealtimeRemoteInput::new(
                vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
            ),
            tilt_ramped_step: AngleDegrees::ZERO,
            tilt_setpoint: AngleDegrees::ZERO,
            current: zero_motor_current(),
            steps: 0,
            counter: 0,
            target: RemoteCurrentTarget::ZERO,
        }
    }
}

impl RemoteControlState {
    #[cfg_attr(not(target_arch = "arm"), allow(dead_code))]
    pub(super) fn set_input(&mut self, input: crate::domain::FloatOutBoyRealtimeRemoteInput) {
        // C map: `remote_input` stores the connected, deadbanded, optionally
        // inverted input at `third_party/float-out-boy/src/remote.c:36-68`.
        self.input = input;
    }

    pub(super) fn reset_runtime_vars(&mut self) {
        // C map: `reset_runtime_vars` clears RC move state at
        // `third_party/float-out-boy/src/main.c:239-252`.
        self.current = zero_motor_current();
        self.steps = 0;
        self.tilt_ramped_step = AngleDegrees::ZERO;
        self.tilt_setpoint = AngleDegrees::ZERO;
    }

    pub(super) fn update_input_tilt(
        &mut self,
        angle_limit: AngleDegrees,
        speed: AngularVelocity,
        sample_rate: SampleRate,
        darkride: bool,
    ) -> AngleDegrees {
        // C map: `remote_configure` derives the per-loop step and
        // `remote_update` ramps the input target at
        // `third_party/float-out-boy/src/remote.c:30-34,70-94`.
        let Some(period) = sample_rate.sample_period() else {
            return self.tilt_setpoint;
        };
        let step = AngleDegrees::from(speed * period).as_degrees();
        let direction = if darkride { -1.0 } else { 1.0 };
        let target = self.input.ratio().as_ratio() * angle_limit.as_degrees() * direction;
        let setpoint = self.tilt_setpoint.as_degrees();
        let target_diff = target - setpoint;
        if target_diff.abs() < 2.0 {
            self.tilt_ramped_step = AngleDegrees::from_degrees(
                0.02 * step * target_diff / 2.0 + 0.98 * self.tilt_ramped_step.as_degrees(),
            );
            let centering_step = self
                .tilt_ramped_step
                .as_degrees()
                .abs()
                .min((target_diff / 2.0).abs() * step)
                * target_diff.signum();
            self.tilt_setpoint = if target_diff.abs() < centering_step.abs() {
                AngleDegrees::from_degrees(target)
            } else {
                AngleDegrees::from_degrees(setpoint + centering_step)
            };
        } else {
            self.tilt_ramped_step = AngleDegrees::from_degrees(
                0.02 * step * target_diff.signum() + 0.98 * self.tilt_ramped_step.as_degrees(),
            );
            self.tilt_setpoint = self.tilt_setpoint + self.tilt_ramped_step;
        }
        self.tilt_setpoint
    }

    pub(super) const fn input(self) -> crate::domain::FloatOutBoyRealtimeRemoteInput {
        self.input
    }

    pub(super) fn queue_move(&mut self, remote_move: RemoteMove) {
        // C map: RC move setup stores a deciamp target, zeroes the counter, and
        // converts packet time to 100 Hz steps before `do_rc_move(d)` consumes
        // it at `third_party/float-out-boy/src/main.c:1735-1758` and
        // `third_party/float-out-boy/src/main.c:275-286`.
        self.counter = 0;
        self.target = remote_move.target;
        self.steps = remote_move.duration_steps;

        if self.target.is_zero() {
            self.current = zero_motor_current();
        }
    }

    pub(super) fn request_ready_current(
        &mut self,
        motor_erpm: Rpm,
        remote_throttle: FloatOutBoyRemoteThrottleConfig<'_>,
        system_time_ticks: TimestampTicks,
        disengage_ticks: TimestampTicks,
    ) -> Option<MotorCurrent> {
        // C map: READY falls through to `do_rc_move(d)` after startup checks at
        // `third_party/float-out-boy/src/main.c:1033-1069`.
        self.request_active_move_current(motor_erpm).or_else(|| {
            self.request_remote_throttle_current(
                remote_throttle,
                system_time_ticks,
                disengage_ticks,
            )
        })
    }

    fn request_active_move_current(&mut self, motor_erpm: Rpm) -> Option<MotorCurrent> {
        if self.steps == 0 {
            return None;
        }

        // Upstream READY falls through to `do_rc_move(d)` at
        // `third_party/float-out-boy/src/main.c:1069`, where active RC move steps
        // filter/request `rc_current` at `third_party/float-out-boy/src/main.c:276-286`.
        self.filter_current(
            self.target.motor_current(),
            CurrentSmoothing::REMOTE_CURRENT_FILTER,
        );
        if motor_erpm.abs() > Rpm::from_revolutions_per_minute(800.0) {
            self.current = zero_motor_current();
        }
        self.steps -= 1;
        self.counter += 1;
        if self.counter == 500 && self.target.should_halve_mid_move() {
            self.target.halve();
        }
        Some(self.current)
    }

    fn request_remote_throttle_current(
        &mut self,
        remote_throttle: FloatOutBoyRemoteThrottleConfig<'_>,
        system_time_ticks: TimestampTicks,
        disengage_ticks: TimestampTicks,
    ) -> Option<MotorCurrent> {
        // C map: READY remote throttle stays idle until the max current,
        // grace period, and deadband checks all pass at
        // `third_party/float-out-boy/src/main.c:291-298`.
        let current_max = remote_throttle.current_max().current();
        let input = self.input.ratio().as_ratio();
        let grace_period = remote_throttle.grace_period();
        if current_max <= Current::ZERO
            || !float_out_boy_ticks_elapsed_seconds(
                system_time_ticks,
                disengage_ticks,
                grace_period,
            )
            || input.abs() <= 0.02
        {
            self.current = zero_motor_current();
            return None;
        }

        let servo = if remote_throttle.invert_throttle() {
            -input
        } else {
            input
        };
        let target_current = MotorCurrent::new(current_max * servo);
        // Upstream READY falls through to `do_rc_move(d)` at
        // `third_party/float-out-boy/src/main.c:1069`, where the remote-throttle idle
        // branch filters and requests `rc_current` at
        // `third_party/float-out-boy/src/main.c:291-298`.
        Some(self.filter_current(target_current, CurrentSmoothing::REMOTE_CURRENT_FILTER))
    }

    fn filter_current(
        &mut self,
        target_current: MotorCurrent,
        smoothing: CurrentSmoothing,
    ) -> MotorCurrent {
        // C map: `do_rc_move` blends the previous RC current with the target
        // using the same 95/5 smoothing factor at
        // `third_party/float-out-boy/src/main.c:275-286` and
        // `third_party/float-out-boy/src/main.c:291-298`.
        let retain_previous = smoothing.retain_previous().as_ratio();
        let accept_target = smoothing.accept_target().as_ratio();
        self.current = MotorCurrent::new(
            self.current.current() * retain_previous + target_current.current() * accept_target,
        );
        self.current
    }

    #[cfg(test)]
    pub(super) const fn target_deciamps_for_test(self) -> i16 {
        self.target.deciamps()
    }

    #[cfg(test)]
    pub(super) const fn remaining_steps_for_test(self) -> u16 {
        self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::{RemoteControlState, RemoteCurrentTarget, RemoteMove, handle_packet};
    use crate::domain::{
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
        FloatOutBoyRealtimeRemoteInput, FloatOutBoyRunState,
    };
    use crate::package::state::FloatOutBoyPackageState;
    use crate::package::test_support::{
        FloatOutBoyConfigTestBytes, editable_config_from_bytes,
        sample_all_data_payloads_with_ride_state,
    };
    use vescpkg_rs::prelude::{
        Current, MotorCurrent, Rpm, SignedRatio, TimestampTicks, VescSeconds,
    };

    #[test]
    fn remote_throttle_requests_idle_current_like_float_out_boy_do_rc_move() {
        let mut remote_control = RemoteControlState::default();
        remote_control.set_input(FloatOutBoyRealtimeRemoteInput::new(
            SignedRatio::from_ratio_const(0.5),
        ));
        let state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ));
        let mut config = *state.serialized_config();
        config.edit_float_out_boy_config(|config| {
            assert!(
                config
                    .set_remote_throttle_current_max(MotorCurrent::new(Current::from_amps(10.0,)))
            )
        });
        config.edit_float_out_boy_config(|config| {
            assert!(config.set_remote_throttle_grace_period(VescSeconds::ZERO))
        });
        let config = editable_config_from_bytes(&config);
        let remote_throttle = config.remote_throttle();

        let requested_current = remote_control
            .request_remote_throttle_current(
                remote_throttle,
                TimestampTicks::from_ticks(1),
                TimestampTicks::from_ticks(0),
            )
            .expect("remote throttle should request current");

        // Upstream `do_rc_move(d)` uses default inverted throttle and filters
        // `rc_current = old * 0.95 + target * 0.05` before requesting current
        // at `third_party/float-out-boy/src/main.c:291-298`; 10A max with 50% input requests -0.25A.
        assert_eq!(requested_current.current().as_amps(), -0.25);
    }

    #[test]
    fn rc_move_command_checksum_failure_becomes_zero_current_step_like_float_out_boy() {
        // C map: `cmd_rc_move` compares `sum != time + current` as ints, then
        // sets `current = 0` at `third_party/float-out-boy/src/main.c:1735-1741`.
        assert_eq!(
            RemoteMove::from_float_out_boy_command(1, 1, 255, 0),
            RemoteMove {
                target: RemoteCurrentTarget::ZERO,
                duration_steps: 1
            }
        );
    }

    #[test]
    fn rc_move_command_steps_idle_current_like_float_out_boy_do_rc_move() {
        let mut remote_control = RemoteControlState::default();
        remote_control.queue_move(RemoteMove::from_float_out_boy_command(1, 40, 2, 42));

        let requested_current = remote_control
            .request_active_move_current(Rpm::ZERO)
            .expect("active RC move should request current");

        // Upstream `cmd_rc_move` sets `rc_steps = time * 100` and target
        // current/10 at `third_party/float-out-boy/src/main.c:1747-1756`; `do_rc_move` filters the first
        // READY tick by 5% at `third_party/float-out-boy/src/main.c:276-286`.
        assert!((requested_current.current().as_amps() - 0.2).abs() < 0.0001);
    }

    #[test]
    fn rc_move_rejects_a_trailing_payload_byte_without_queueing_current() {
        let mut remote_control = RemoteControlState::default();
        let payloads = sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        );
        let packet = [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::RcMove.id(),
            1,
            40,
            2,
            42,
            0,
        ];

        assert!(!handle_packet(payloads, &mut remote_control, &packet));
        assert_eq!(remote_control.request_active_move_current(Rpm::ZERO), None);
    }

    #[test]
    fn rc_move_halves_large_target_after_500_steps_like_float_out_boy_do_rc_move() {
        let mut remote_control = RemoteControlState::default();
        remote_control.queue_move(RemoteMove::from_float_out_boy_command(1, 60, 6, 66));

        for _ in 0..500 {
            assert!(
                remote_control
                    .request_active_move_current(Rpm::ZERO)
                    .is_some()
            );
        }

        // Upstream `do_rc_move(d)` halves targets above 2A when `rc_counter`
        // reaches 500 at `third_party/float-out-boy/src/main.c:281-284`, after decrementing steps.
        assert_eq!(remote_control.target_deciamps_for_test(), 30);
        assert_eq!(remote_control.remaining_steps_for_test(), 100);
    }
}
