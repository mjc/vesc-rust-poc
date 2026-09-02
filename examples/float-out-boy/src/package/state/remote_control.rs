use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyAppDataCommand, FloatOutBoyRunState};
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
use crate::package::state::float_out_boy_command_payload;
use crate::package::state::smooth_setpoint::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};
#[cfg(test)]
use vescpkg_rs::prelude::Rpm;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, Current, MotorCurrent, Ratio, SampleRate, Speed, TimestampTicks,
    VescSeconds,
};
use vescpkg_rs::timer_older as float_out_boy_ticks_elapsed_seconds;

#[cfg(test)]
const REMOTE_CURRENT_FILTER: Ratio = Ratio::from_ratio_const(0.05);
const REMOTE_INPUT_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.5);
const REMOTE_MOVE_IDLE_TIMEOUT: VescSeconds = VescSeconds::from_seconds(1.0);
const REMOTE_COMMAND_MOVE_GRACE: VescSeconds = VescSeconds::from_seconds(2.0);
const REMOTE_COMMAND_DEFAULT_MOVE_SPEED: Speed = Speed::from_kilometers_per_hour(5.0);
const REMOTE_MOVE_TORQUE_LIMIT: MotorTorque = MotorTorque::from_newton_meters(10.0);

fn parse_remote_command(byte: u8) -> Option<vescpkg_rs::SignedRatio> {
    let value = i8::from_ne_bytes([byte]);
    (value != i8::MIN).then(|| vescpkg_rs::SignedRatio::clamped(f32::from(value) / 127.0))
}

fn remote_move_target(input: vescpkg_rs::SignedRatio, maximum: Speed) -> Speed {
    Speed::from_kilometers_per_hour(input.as_ratio() * maximum.as_kilometers_per_hour())
}

fn zero_motor_current() -> MotorCurrent {
    // C map: `reset_runtime_vars` and the RC-move idle branches clear current
    // by writing zero at `third_party/float-out-boy/src/main.c:239-252` and
    // `third_party/float-out-boy/src/main.c:291-298`.
    MotorCurrent::new(Current::ZERO)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct RemoteCurrentTarget(i16);

impl RemoteCurrentTarget {
    const ZERO: Self = Self(0);

    const fn new(deciamps: i16) -> Self {
        Self(deciamps)
    }

    #[cfg(test)]
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
        // C map: `cmd_rc_move` clamps positive targets above 8 A (80 packet
        // deciamps) before storing the 2 A fallback target.
        self.0 > 80
    }

    #[cfg(test)]
    const fn should_halve_mid_move(self) -> bool {
        // C map: `do_rc_move` halves targets above 2A after 500 steps.
        self.0 > 20
    }

    #[cfg(test)]
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
        let current = if u16::from(sum) == u16::from(time).saturating_add(u16::from(current)) {
            current
        } else {
            0
        };

        let target = match direction {
            0 => RemoteCurrentTarget::new(i16::from(current).saturating_neg()),
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

pub(super) fn handle_remote_packet(
    remote_control: &mut RemoteControlState,
    bytes: &[u8],
    now: TimestampTicks,
    disengage_ticks: TimestampTicks,
    maximum_move_speed: Speed,
) -> bool {
    match float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::Remote) {
        Some([byte, ..]) => {
            let Some(input) = parse_remote_command(*byte) else {
                return true;
            };
            remote_control.input = crate::domain::FloatOutBoyRealtimeRemoteInput::new(input);
            if float_out_boy_ticks_elapsed_seconds(now, disengage_ticks, REMOTE_COMMAND_MOVE_GRACE)
            {
                let maximum = if maximum_move_speed > Speed::ZERO {
                    maximum_move_speed
                } else {
                    REMOTE_COMMAND_DEFAULT_MOVE_SPEED
                };
                remote_control.move_target = Some(remote_move_target(input, maximum));
            }
            remote_control.command_ticks = Some(now);
            true
        }
        Some([]) | None => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RemoteControlState {
    input: crate::domain::FloatOutBoyRealtimeRemoteInput,
    input_tilt: SmoothSetpoint,
    current: MotorCurrent,
    steps: u16,
    counter: u16,
    target: RemoteCurrentTarget,
    command_ticks: Option<TimestampTicks>,
    move_target: Option<Speed>,
    move_integral: MotorTorque,
    move_idle_ticks: Option<TimestampTicks>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RemotePhysicalInputContext {
    pub(super) now: TimestampTicks,
    pub(super) disengage_ticks: TimestampTicks,
    pub(super) deadband: Ratio,
    pub(super) inverted: bool,
    pub(super) maximum_move_speed: Speed,
    pub(super) move_grace: VescSeconds,
}

impl Default for RemoteControlState {
    fn default() -> Self {
        // C map: Float Out Boy resets RC move state and current to zero at
        // `third_party/float-out-boy/src/main.c:239-252`.
        Self {
            input: crate::domain::FloatOutBoyRealtimeRemoteInput::new(
                vescpkg_rs::prelude::SignedRatio::from_ratio_const(0.0),
            ),
            input_tilt: SmoothSetpoint::default(),
            current: zero_motor_current(),
            steps: 0,
            counter: 0,
            target: RemoteCurrentTarget::ZERO,
            command_ticks: None,
            move_target: None,
            move_integral: MotorTorque::ZERO,
            move_idle_ticks: None,
        }
    }
}

impl RemoteControlState {
    #[cfg(test)]
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
        self.input_tilt.reset();
        self.command_ticks = None;
        self.move_target = None;
        self.move_integral = MotorTorque::ZERO;
        self.move_idle_ticks = None;
    }

    pub(super) fn refresh_physical_input(
        &mut self,
        raw: Option<vescpkg_rs::SignedRatio>,
        context: RemotePhysicalInputContext,
    ) {
        let RemotePhysicalInputContext {
            now,
            disengage_ticks,
            deadband,
            inverted,
            maximum_move_speed,
            move_grace,
        } = context;
        if self.command_ticks.is_some_and(|command| {
            !float_out_boy_ticks_elapsed_seconds(now, command, REMOTE_INPUT_TIMEOUT)
        }) {
            return;
        }
        self.command_ticks = None;

        let Some(raw) = raw else {
            self.input = crate::domain::FloatOutBoyRealtimeRemoteInput::new(
                vescpkg_rs::SignedRatio::from_ratio_const(0.0),
            );
            self.move_target = None;
            return;
        };
        let raw = raw.as_ratio();
        let deadband = deadband.as_ratio();
        let normalized = if raw.abs() < deadband {
            0.0
        } else {
            raw.signum() * (raw.abs() - deadband) / (1.0 - deadband)
        };
        let normalized = vescpkg_rs::SignedRatio::clamped(normalized);
        if normalized.as_ratio() == 0.0 {
            self.move_target = self
                .move_idle_ticks
                .filter(|idle| {
                    !float_out_boy_ticks_elapsed_seconds(now, *idle, REMOTE_MOVE_IDLE_TIMEOUT)
                })
                .map(|_| Speed::ZERO);
        } else if maximum_move_speed > Speed::ZERO
            && float_out_boy_ticks_elapsed_seconds(now, disengage_ticks, move_grace)
        {
            self.move_target = Some(remote_move_target(normalized, maximum_move_speed));
            self.move_idle_ticks = Some(now);
        }
        self.input = crate::domain::FloatOutBoyRealtimeRemoteInput::new(if inverted {
            vescpkg_rs::SignedRatio::clamped(-normalized.as_ratio())
        } else {
            normalized
        });
    }

    #[cfg(test)]
    pub(super) fn update_input_tilt(
        &mut self,
        angle_limit: AngleDegrees,
        sample_rate: SampleRate,
        darkride: bool,
    ) -> AngleDegrees {
        // C map: pinned-cutoff `remote_configure` configures SmoothSetpoint and
        // `remote_update` advances it from the remote target.
        sample_rate
            .sample_period()
            .map_or(self.input_tilt.value(), |elapsed| {
                self.update_input_tilt_elapsed(angle_limit, elapsed, darkride)
            })
    }

    #[cfg(test)]
    pub(super) fn update_input_tilt_elapsed(
        &mut self,
        angle_limit: AngleDegrees,
        elapsed: VescSeconds,
        darkride: bool,
    ) -> AngleDegrees {
        let frequency = smooth_setpoint_frequency(elapsed);
        self.update_input_tilt_elapsed_with_filter_rate(
            angle_limit,
            elapsed,
            darkride,
            VescSeconds::from_seconds(0.2),
            frequency.unwrap_or(SampleRate::from_hertz(500.0)),
        )
    }

    pub(super) fn update_input_tilt_elapsed_with_filter_rate(
        &mut self,
        angle_limit: AngleDegrees,
        elapsed: VescSeconds,
        darkride: bool,
        filter_time_constant: VescSeconds,
        filter_rate: SampleRate,
    ) -> AngleDegrees {
        let seconds = elapsed.as_seconds();
        if !seconds.is_finite() || seconds <= 0.0 {
            return self.input_tilt.value();
        }
        self.input_tilt.configure(
            SmoothSetpointConfig {
                time_constant: filter_time_constant,
                on_speed_time_constant: VescSeconds::from_seconds(
                    filter_time_constant.as_seconds() * 0.25,
                ),
                off_speed_time_constant: VescSeconds::from_seconds(
                    filter_time_constant.as_seconds() * 0.25,
                ),
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: AngularVelocity::from_degrees_per_second(100.0),
                off_speed_up: AngularVelocity::from_degrees_per_second(100.0),
                on_speed_down: AngularVelocity::from_degrees_per_second(100.0),
                off_speed_down: AngularVelocity::from_degrees_per_second(100.0),
            },
            filter_rate,
        );
        let upright_target = angle_limit * self.input.ratio().as_ratio();
        let target = if darkride {
            -upright_target
        } else {
            upright_target
        };
        self.input_tilt.update(
            target,
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
        self.input_tilt.value()
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

    #[cfg(test)]
    fn request_active_move_current(&mut self, motor_erpm: Rpm) -> Option<MotorCurrent> {
        if self.steps == 0 {
            return None;
        }

        // Upstream READY falls through to `do_rc_move(d)` at
        // `third_party/float-out-boy/src/main.c:1069`, where active RC move steps
        // filter/request `rc_current` at `third_party/float-out-boy/src/main.c:276-286`.
        self.filter_current(self.target.motor_current());
        if motor_erpm.abs() > Rpm::from_revolutions_per_minute(800.0) {
            self.current = zero_motor_current();
        }
        self.steps = self.steps.saturating_sub(1);
        self.counter = self.counter.saturating_add(1);
        if self.counter == 500 && self.target.should_halve_mid_move() {
            self.target.halve();
        }
        Some(self.current)
    }

    #[cfg(test)]
    fn filter_current(&mut self, target_current: MotorCurrent) -> MotorCurrent {
        // C map: `do_rc_move` blends the previous RC current with the target
        // using the same 95/5 smoothing factor at
        // `third_party/float-out-boy/src/main.c:275-286` and
        // `third_party/float-out-boy/src/main.c:291-298`.
        self.current = self.current * REMOTE_CURRENT_FILTER.complement().as_ratio()
            + target_current * REMOTE_CURRENT_FILTER.as_ratio();
        self.current
    }

    pub(super) fn request_remote_move_current(
        &mut self,
        vehicle_speed: Speed,
        elapsed: VescSeconds,
        torque_constant: MotorTorqueConstant,
    ) -> Option<MotorCurrent> {
        let Some(target) = self.move_target else {
            self.move_integral = MotorTorque::ZERO;
            return None;
        };
        let error = target.as_kilometers_per_hour() - vehicle_speed.as_kilometers_per_hour();
        self.move_integral = MotorTorque::from_newton_meters(
            (self.move_integral.as_newton_meters() + error * elapsed.as_seconds()).clamp(
                -REMOTE_MOVE_TORQUE_LIMIT.as_newton_meters(),
                REMOTE_MOVE_TORQUE_LIMIT.as_newton_meters(),
            ),
        );
        let torque = MotorTorque::from_newton_meters(
            (1.2 * error + self.move_integral.as_newton_meters()).clamp(
                -REMOTE_MOVE_TORQUE_LIMIT.as_newton_meters(),
                REMOTE_MOVE_TORQUE_LIMIT.as_newton_meters(),
            ),
        );
        Some(torque_constant.motor_current_from_torque(torque))
    }
}

#[cfg(test)]
fn smooth_setpoint_frequency(elapsed: VescSeconds) -> Option<SampleRate> {
    let seconds = elapsed.as_seconds();
    (seconds.is_finite() && seconds > 0.0).then(|| SampleRate::from_hertz(1.0 / seconds))
}

#[cfg(test)]
mod tests;
