use super::loop_io::{LoopConfig, LoopInput, LoopOutput, LoopState};
use crate::motor_torque::MotorTorqueConstant;
use vescpkg_rs::prelude::VescSeconds;

use super::{booster::Phase as BoosterPhase, current::softstart_increment, pid::Phase as PidPhase};

#[cfg(test)]
use super::{
    booster::{Branch, Proportional},
    loop_io::PidState,
    pid::PitchRate,
};

#[cfg(test)]
use crate::domain::{
    FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyRealtimeRuntimeSetpoint,
    FloatOutBoyTractionControlState,
};

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
    #[cfg(test)]
    pub(crate) fn advance_balance_loop_elapsed(
        self,
        config: LoopConfig,
        input: LoopInput,
        elapsed: VescSeconds,
    ) -> LoopOutput {
        self.advance_balance_loop_elapsed_with_torque(
            config,
            input,
            elapsed,
            MotorTorqueConstant::REFLOAT_COMPAT,
        )
    }

    pub(crate) fn advance_balance_loop_elapsed_with_torque(
        self,
        config: LoopConfig,
        input: LoopInput,
        elapsed: VescSeconds,
        motor_torque_constant: MotorTorqueConstant,
    ) -> LoopOutput {
        let (pid_torques, state) = PidPhase::from_step(config, input).update_state(self, elapsed);
        let booster_torque =
            BoosterPhase::from_step(config, input).filtered_torque(state.booster_torque, elapsed);
        let pitch_based = pid_torques.pitch_based_current(
            booster_torque,
            motor_torque_constant,
            state.softstart_pid_limit,
            input.motor_current_max,
            softstart_increment(elapsed),
        );
        let state = state.with_booster_torque_and_softstart_limit(
            booster_torque,
            pitch_based.softstart_pid_limit,
        );

        let balance_current = pid_torques
            .requested_with_pitch_based(pitch_based, motor_torque_constant)
            .clamped_to(input.current_limit())
            .adjusted_for_darkride(input.darkride)
            .filtered_from(state.balance_current, input.traction_control, elapsed);
        let state = state.with_balance_current(balance_current);

        LoopOutput {
            requested_current: state.balance_current,
            state,
        }
    }
}

#[cfg(test)]
mod tests;
