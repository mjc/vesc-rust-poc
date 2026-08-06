use crate::domain::FloatOutBoyRealtimeRemoteInput;
#[cfg(test)]
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::Ratio;
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, MotorCurrent, SampleRate, SignedRatio, Speed, TimestampTicks,
    VescSeconds,
};
use vescpkg_rs::{
    SmoothSetpoint, SmoothSetpointConfig, SmoothSetpointDirection, SmoothSetpointMultiplier,
};

#[cfg(any(test, target_arch = "arm"))]
const REMOTE_INPUT_TIMEOUT: VescSeconds = VescSeconds::from_seconds(0.5);
#[cfg(any(test, target_arch = "arm"))]
const REMOTE_MOVE_IDLE_TIMEOUT: VescSeconds = VescSeconds::from_seconds(1.0);
const REMOTE_COMMAND_MOVE_GRACE: VescSeconds = VescSeconds::from_seconds(2.0);
const REMOTE_COMMAND_DEFAULT_MOVE_SPEED: Speed = Speed::from_kilometers_per_hour(5.0);

fn parse_remote_command(byte: u8) -> Option<SignedRatio> {
    let value = i8::from_ne_bytes([byte]);
    (value != i8::MIN).then(|| SignedRatio::clamped(f32::from(value) / 127.0))
}

fn remote_move_target(input: SignedRatio, maximum: Speed) -> Speed {
    Speed::from_kilometers_per_hour(input.as_ratio() * maximum.as_kilometers_per_hour())
}

const REMOTE_MOVE_INTEGRAL_LIMIT_NEWTON_METERS: f32 = 10.0;

fn advance_remote_move_integral(
    integral: &mut MotorTorque,
    speed_error_kph: f32,
    elapsed: VescSeconds,
) {
    *integral = MotorTorque::from_newton_meters(
        (integral.as_newton_meters() + speed_error_kph * elapsed.as_seconds()).clamp(
            -REMOTE_MOVE_INTEGRAL_LIMIT_NEWTON_METERS,
            REMOTE_MOVE_INTEGRAL_LIMIT_NEWTON_METERS,
        ),
    );
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct RemoteControlState {
    input: SignedRatio,
    tilt_setpoint: SmoothSetpoint,
    command_epoch: Option<TimestampTicks>,
    move_target: Option<Speed>,
    move_integral: MotorTorque,
    move_idle_epoch: Option<TimestampTicks>,
}

impl RemoteControlState {
    #[cfg(test)]
    pub(super) fn set_input(&mut self, input: FloatOutBoyRealtimeRemoteInput) {
        self.input = input.ratio();
    }

    pub(super) fn reset_runtime_vars(&mut self) {
        self.tilt_setpoint.reset();
        self.move_target = None;
        self.move_integral = MotorTorque::ZERO;
        self.move_idle_epoch = None;
    }

    pub(super) fn handle_command(
        &mut self,
        now: TimestampTicks,
        disengage_epoch: TimestampTicks,
        maximum_move_speed: Speed,
        payload: &[u8],
    ) -> bool {
        let Some(input) = payload.first().copied().and_then(parse_remote_command) else {
            return !payload.is_empty();
        };

        self.input = input;
        if vescpkg_rs::timer_older(now, disengage_epoch, REMOTE_COMMAND_MOVE_GRACE) {
            let maximum = if maximum_move_speed > Speed::ZERO {
                maximum_move_speed
            } else {
                REMOTE_COMMAND_DEFAULT_MOVE_SPEED
            };
            self.move_target = Some(remote_move_target(input, maximum));
        }
        self.command_epoch = Some(now);
        true
    }

    #[cfg(test)]
    pub(super) fn handle_packet(
        &mut self,
        now: TimestampTicks,
        disengage_epoch: TimestampTicks,
        maximum_move_speed: Speed,
        bytes: &[u8],
    ) -> bool {
        vescpkg_rs::protocol_app_data::parse_app_data_command::<FloatOutBoyAppDataCommand>(
            bytes,
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        )
        .filter(|(command, _)| *command == FloatOutBoyAppDataCommand::Remote)
        .is_some_and(|(_, payload)| {
            self.handle_command(now, disengage_epoch, maximum_move_speed, payload)
        })
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
            .is_some_and(|epoch| !vescpkg_rs::timer_older(now, epoch, REMOTE_INPUT_TIMEOUT))
        {
            return;
        }
        self.command_epoch = None;

        let Some(raw_input) = raw_input else {
            self.input = SignedRatio::from_ratio_const(0.0);
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
        let move_input = SignedRatio::clamped(normalized);
        if normalized == 0.0 {
            self.move_target = self
                .move_idle_epoch
                .filter(|epoch| !vescpkg_rs::timer_older(now, *epoch, REMOTE_MOVE_IDLE_TIMEOUT))
                .map(|_| Speed::ZERO);
        } else if maximum_move_speed > Speed::ZERO
            && vescpkg_rs::timer_older(now, disengage_epoch, move_grace)
        {
            self.move_target = Some(remote_move_target(move_input, maximum_move_speed));
            self.move_idle_epoch = Some(now);
        }
        self.input = SignedRatio::clamped(if inverted { -normalized } else { normalized });
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
            SmoothSetpointConfig::symmetric(
                filter_time_constant,
                speed_time_constant,
                speed_time_constant,
                VescSeconds::from_seconds(0.2),
                AngularVelocity::from_degrees_per_second(100.0),
                AngularVelocity::from_degrees_per_second(100.0),
            ),
            SampleRate::from_hertz(1.0 / seconds),
        );
        let upright_target = angle_limit * self.input.as_ratio();
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
        FloatOutBoyRealtimeRemoteInput::new(self.input)
    }

    #[cfg(test)]
    pub(super) fn move_target_for_test(self) -> Option<Speed> {
        self.move_target
    }

    pub(super) fn request_ready_current(
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
        advance_remote_move_integral(&mut self.move_integral, error, elapsed);
        Some(
            torque_constant.motor_current_from_torque(MotorTorque::from_newton_meters(
                (1.2 * error + self.move_integral.as_newton_meters()).clamp(-10.0, 10.0),
            )),
        )
    }
}

#[cfg(test)]
#[path = "remote_control/tests.rs"]
mod tests;
