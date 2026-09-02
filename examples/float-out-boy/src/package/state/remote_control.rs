use crate::domain::{FloatOutBoyAppDataCommand, FloatOutBoyRealtimeRemoteInput};
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
use crate::package::state::smooth_setpoint::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::Ratio;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, MotorCurrent, SampleRate, SignedRatio, Speed, TimestampTicks,
    VescSeconds,
};

#[cfg(any(test, target_arch = "arm"))]
const REMOTE_INPUT_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.5);
#[cfg(any(test, target_arch = "arm"))]
const REMOTE_MOVE_IDLE_TIMEOUT: VescSeconds = VescSeconds::from_seconds(1.0);
const REMOTE_COMMAND_MOVE_GRACE: VescSeconds = VescSeconds::from_seconds(2.0);
const REMOTE_COMMAND_DEFAULT_MOVE_SPEED: Speed = Speed::from_kilometers_per_hour(5.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct RemoteCommandByte(i8);

impl RemoteCommandByte {
    fn parse(byte: u8) -> Option<Self> {
        let value = i8::from_ne_bytes([byte]);
        (value != i8::MIN).then_some(Self(value))
    }

    fn input(self) -> FloatOutBoyRealtimeRemoteInput {
        FloatOutBoyRealtimeRemoteInput::new(SignedRatio::clamped(f32::from(self.0) / 127.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct RemoteMoveTarget(Speed);

impl RemoteMoveTarget {
    #[cfg(any(test, target_arch = "arm"))]
    const STOPPED: Self = Self(Speed::ZERO);

    fn from_input(input: FloatOutBoyRealtimeRemoteInput, maximum: Speed) -> Self {
        Self(Speed::from_kilometers_per_hour(
            input.ratio().as_ratio() * maximum.as_kilometers_per_hour(),
        ))
    }

    const fn speed(self) -> Speed {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct RemoteMoveIntegral(MotorTorque);

impl RemoteMoveIntegral {
    const ZERO: Self = Self(MotorTorque::ZERO);
    const LIMIT_NEWTON_METERS: f32 = 10.0;

    fn advance(&mut self, speed_error_kph: f32, elapsed: VescSeconds) {
        self.0 = MotorTorque::from_newton_meters(
            (self.0.as_newton_meters() + speed_error_kph * elapsed.as_seconds())
                .clamp(-Self::LIMIT_NEWTON_METERS, Self::LIMIT_NEWTON_METERS),
        );
    }

    const fn torque(self) -> MotorTorque {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct RemoteCommandEpoch(TimestampTicks);

impl RemoteCommandEpoch {
    #[cfg(any(test, target_arch = "arm"))]
    fn is_active(self, now: TimestampTicks) -> bool {
        !vescpkg_rs::timer_older(now, self.0, REMOTE_INPUT_TIMEOUT)
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PhysicalRemoteInput {
    pub(super) raw: Option<SignedRatio>,
    pub(super) now: TimestampTicks,
    pub(super) disengage_epoch: TimestampTicks,
    pub(super) deadband: Ratio,
    pub(super) inverted: bool,
    pub(super) maximum_move_speed: Speed,
    pub(super) move_grace: VescSeconds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RemoteControlState {
    input: FloatOutBoyRealtimeRemoteInput,
    tilt_setpoint: SmoothSetpoint,
    command_epoch: Option<RemoteCommandEpoch>,
    move_target: Option<RemoteMoveTarget>,
    move_integral: RemoteMoveIntegral,
    move_idle_epoch: Option<TimestampTicks>,
}

impl Default for RemoteControlState {
    fn default() -> Self {
        Self {
            input: FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0)),
            tilt_setpoint: SmoothSetpoint::default(),
            command_epoch: None,
            move_target: None,
            move_integral: RemoteMoveIntegral::ZERO,
            move_idle_epoch: None,
        }
    }
}

impl RemoteControlState {
    #[cfg(test)]
    pub(super) fn set_input(&mut self, input: FloatOutBoyRealtimeRemoteInput) {
        self.input = input;
    }

    pub(super) fn reset_runtime_vars(&mut self) {
        self.tilt_setpoint.reset();
        self.move_target = None;
        self.move_integral = RemoteMoveIntegral::ZERO;
        self.move_idle_epoch = None;
    }

    pub(super) fn handle_packet(
        &mut self,
        now: TimestampTicks,
        disengage_epoch: TimestampTicks,
        maximum_move_speed: Speed,
        bytes: &[u8],
    ) -> bool {
        let [package_id, command, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != crate::domain::FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID
            || *command != FloatOutBoyAppDataCommand::Remote.id()
        {
            return false;
        }
        let Some(byte) = payload.first() else {
            return false;
        };
        let Some(command) = RemoteCommandByte::parse(*byte) else {
            return true;
        };

        let input = command.input();
        self.input = input;
        if vescpkg_rs::timer_older(now, disengage_epoch, REMOTE_COMMAND_MOVE_GRACE) {
            let maximum = if maximum_move_speed > Speed::ZERO {
                maximum_move_speed
            } else {
                REMOTE_COMMAND_DEFAULT_MOVE_SPEED
            };
            self.move_target = Some(RemoteMoveTarget::from_input(input, maximum));
        }
        self.command_epoch = Some(RemoteCommandEpoch(now));
        true
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(super) fn refresh_physical_input(&mut self, sample: PhysicalRemoteInput) {
        let PhysicalRemoteInput {
            raw: raw_input,
            now,
            disengage_epoch,
            deadband,
            inverted,
            maximum_move_speed,
            move_grace,
        } = sample;
        if self
            .command_epoch
            .is_some_and(|command| command.is_active(now))
        {
            return;
        }
        self.command_epoch = None;

        let Some(raw_input) = raw_input else {
            self.input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::from_ratio_const(0.0));
            self.move_target = None;
            return;
        };
        let deadband = deadband.as_ratio();
        let raw = raw_input.as_ratio();
        let normalized = if raw.abs() < deadband {
            0.0
        } else {
            raw.signum() * (raw.abs() - deadband) / (1.0 - deadband)
        };
        let move_input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::clamped(normalized));
        if normalized == 0.0 {
            self.move_target = self
                .move_idle_epoch
                .filter(|epoch| !vescpkg_rs::timer_older(now, *epoch, REMOTE_MOVE_IDLE_TIMEOUT))
                .map(|_| RemoteMoveTarget::STOPPED);
        } else if maximum_move_speed > Speed::ZERO
            && vescpkg_rs::timer_older(now, disengage_epoch, move_grace)
        {
            self.move_target = Some(RemoteMoveTarget::from_input(move_input, maximum_move_speed));
            self.move_idle_epoch = Some(now);
        }
        self.input = FloatOutBoyRealtimeRemoteInput::new(SignedRatio::clamped(if inverted {
            -normalized
        } else {
            normalized
        }));
    }

    #[cfg(test)]
    pub(super) fn update_input_tilt(
        &mut self,
        angle_limit: AngleDegrees,
        filter_time_constant: VescSeconds,
        sample_rate: SampleRate,
        darkride: bool,
    ) -> AngleDegrees {
        sample_rate
            .sample_period()
            .map_or(self.tilt_setpoint.value(), |period| {
                self.update_input_tilt_elapsed(angle_limit, filter_time_constant, period, darkride)
            })
    }

    pub(super) fn update_input_tilt_elapsed(
        &mut self,
        angle_limit: AngleDegrees,
        filter_time_constant: VescSeconds,
        elapsed: VescSeconds,
        darkride: bool,
    ) -> AngleDegrees {
        let seconds = elapsed.as_seconds();
        if !seconds.is_finite() || seconds <= 0.0 {
            return self.tilt_setpoint.value();
        }
        let speed_time_constant =
            VescSeconds::from_seconds(filter_time_constant.as_seconds() * 0.25);
        self.tilt_setpoint.configure(
            SmoothSetpointConfig {
                time_constant: filter_time_constant,
                on_speed_time_constant: speed_time_constant,
                off_speed_time_constant: speed_time_constant,
                winddown_time_constant: VescSeconds::from_seconds(0.2),
                on_speed_up: AngularVelocity::from_degrees_per_second(100.0),
                off_speed_up: AngularVelocity::from_degrees_per_second(100.0),
                on_speed_down: AngularVelocity::from_degrees_per_second(100.0),
                off_speed_down: AngularVelocity::from_degrees_per_second(100.0),
            },
            SampleRate::from_hertz(1.0 / seconds),
        );
        let upright_target = angle_limit * self.input.ratio().as_ratio();
        let target = if darkride {
            -upright_target
        } else {
            upright_target
        };
        self.tilt_setpoint.update(
            target,
            SmoothSetpointDirection::Forward,
            SmoothSetpointMultiplier::ONE,
            elapsed,
        );
        self.tilt_setpoint.value()
    }

    pub(super) const fn input(self) -> FloatOutBoyRealtimeRemoteInput {
        self.input
    }

    #[cfg(test)]
    pub(super) fn move_target_for_test(self) -> Option<Speed> {
        self.move_target.map(RemoteMoveTarget::speed)
    }

    pub(super) fn request_ready_current(
        &mut self,
        vehicle_speed: Speed,
        elapsed: VescSeconds,
        torque_constant: MotorTorqueConstant,
    ) -> Option<MotorCurrent> {
        let Some(target) = self.move_target else {
            self.move_integral = RemoteMoveIntegral::ZERO;
            return None;
        };
        let error =
            target.speed().as_kilometers_per_hour() - vehicle_speed.as_kilometers_per_hour();
        self.move_integral.advance(error, elapsed);
        let torque = MotorTorque::from_newton_meters(
            (1.2 * error + self.move_integral.torque().as_newton_meters()).clamp(-10.0, 10.0),
        );
        Some(torque_constant.motor_current_from_torque(torque))
    }
}

#[cfg(test)]
#[path = "remote_control/tests.rs"]
mod tests;
