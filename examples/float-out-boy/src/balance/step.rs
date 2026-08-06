use super::loop_io::{LoopConfig, LoopInput, LoopOutput, LoopState};
use vescpkg_rs::prelude::VescSeconds;

#[cfg(test)]
use super::{booster::Branch, loop_io::PidState, pid::PitchRate};

#[cfg(test)]
use crate::domain::{
    FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyRealtimeRuntimeSetpoint,
};
#[cfg(test)]
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};

#[cfg(test)]
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, ImuRoll,
    IntegralCurrentGain, MotorCurrent, MotorCurrentLimit, PidScale, RateCurrentGain, SampleRate,
};

impl LoopState {
    /// Advance one upstream Float Out Boy RUNNING balance-current step.
    ///
    /// Source map: upstream calls `pid_update`, `booster_update`, soft-start,
    /// current limiting, darkride inversion, traction freewheel, and motor-current
    /// request in `third_party/float-out-boy/src/main.c:918-956`; the subroutines are
    /// `third_party/float-out-boy/src/pid.c:37-73`,
    /// `third_party/float-out-boy/src/booster.c:32-75`, and
    /// `third_party/float-out-boy/src/imu.c:43-53`.
    #[inline]
    #[cfg(test)]
    pub(crate) fn advance_balance_loop(self, config: LoopConfig, input: LoopInput) -> LoopOutput {
        let elapsed = config
            .hertz
            .sample_period()
            .unwrap_or_else(|| VescSeconds::from_seconds(0.0));
        self.advance_balance_loop_elapsed(config, input, elapsed)
    }

    #[inline]
    pub(crate) fn advance_balance_loop_elapsed(
        self,
        config: LoopConfig,
        input: LoopInput,
        elapsed: VescSeconds,
    ) -> LoopOutput {
        let (pid_torques, state) = self.update_pid(config, input, elapsed);
        let booster_torque = input.filtered_booster_torque(config, state.booster_torque, elapsed);
        let pitch_based = super::current::PitchBasedCurrent::from_rate_and_booster(
            pid_torques.rate_damping,
            booster_torque,
            input.motor_torque_constant,
            state.softstart_pid_limit,
            input.motor_current_limits.positive(),
            elapsed,
        );

        let balance_current = super::current::RequestedCurrent(
            input.motor_torque_constant.motor_current_from_torque(
                pid_torques.angle_proportional.plus(pid_torques.integral),
            ) + pitch_based.current,
        )
        .clamped_to(input.current_limit())
        .adjusted_for_darkride(input.darkride)
        .filtered_from(state.balance_current, input.traction_control, elapsed);
        let state = LoopState {
            balance_current,
            booster_torque,
            softstart_pid_limit: pitch_based.softstart_pid_limit,
            ..state
        };

        LoopOutput {
            requested_current: state.balance_current,
            state,
        }
    }
}

#[cfg(test)]
mod tests;
