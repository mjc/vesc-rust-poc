use super::loop_io::{LoopConfig, LoopInput, LoopOutput, LoopState};

#[cfg(test)]
use super::{booster::Branch, loop_io::PidState, pid::PitchRate};

#[cfg(test)]
use crate::domain::{
    FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyRealtimeRuntimeSetpoint,
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
    pub(crate) fn advance_balance_loop(self, config: LoopConfig, input: LoopInput) -> LoopOutput {
        let (pid_currents, state) = self.update_pid(config, input);
        let booster_current = input.filtered_booster_current(config, state.booster_current);
        let pitch_based = super::current::PitchBasedCurrent::from_rate_and_booster(
            pid_currents.rate_damping,
            booster_current,
            state.softstart_pid_limit,
            input.motor_current_max,
            config.hertz,
        );

        let balance_current = super::current::RequestedCurrent(
            pid_currents.angle_proportional + pid_currents.integral + pitch_based.current,
        )
        .clamped_to(input.current_limit())
        .adjusted_for_darkride(input.darkride)
        .filtered_from(state.balance_current, input.traction_control);
        let state = LoopState {
            balance_current,
            booster_current,
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
