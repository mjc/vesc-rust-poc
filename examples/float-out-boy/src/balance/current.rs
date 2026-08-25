use super::booster::Branch;
use super::loop_io::LoopInput;
use super::loop_io::LoopState;
use crate::domain::{FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyTractionControlState};
use crate::ema::EmaAlpha;
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
use vescpkg_rs::prelude::{Current, Frequency, MotorCurrent, MotorCurrentLimit, VescSeconds};

// C map: upstream chooses these scalar current limits and ramp values inside
// `third_party/float-out-boy/src/main.c:924-954`.
const HANDTEST_CURRENT_LIMIT_AMPS: f32 = 7.0;
const FLYWHEEL_CURRENT_LIMIT_AMPS: f32 = 40.0;
const SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND: f32 = 100.0;
const BALANCE_CURRENT_FILTER_CUTOFF: Frequency = Frequency::from_hertz(25.0);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct PitchBasedDemand(MotorTorque);

impl PitchBasedDemand {
    #[inline]
    fn from_torques(rate_damping: MotorTorque, booster: MotorTorque) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:926-930` adds the rate-P and
        // booster terms before soft-start and current limiting.
        Self(rate_damping.add(booster))
    }

    #[inline]
    fn with_softstart(
        self,
        motor_torque_constant: MotorTorqueConstant,
        softstart_pid_limit: MotorCurrentLimit,
        motor_current_max: MotorCurrentLimit,
        softstart_increment: Current,
    ) -> PitchBasedCurrent {
        let current = motor_torque_constant.motor_current_from_torque(self.0);
        if softstart_pid_limit.current() < motor_current_max.current() {
            PitchBasedCurrent {
                // C map: `third_party/float-out-boy/src/main.c:927-929` clamps only
                // magnitude; sign remains the requested direction.
                current: softstart_pid_limit.clamp(current),
                // C map: `third_party/float-out-boy/src/main.c:927-929` advances the
                // soft-start current limit at 100 A/s.
                softstart_pid_limit: MotorCurrentLimit::new(
                    softstart_pid_limit.current() + softstart_increment,
                ),
            }
        } else {
            PitchBasedCurrent {
                current,
                softstart_pid_limit,
            }
        }
    }
}

fn valid_elapsed_seconds(elapsed: VescSeconds) -> f32 {
    let seconds = elapsed.as_seconds();
    if seconds.is_finite() && seconds > 0.0 {
        seconds
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PitchBasedCurrent {
    pub(super) current: MotorCurrent,
    pub(super) softstart_pid_limit: MotorCurrentLimit,
}

impl PitchBasedCurrent {
    /// Source map: upstream soft-start clamps pitch-based current at
    /// `third_party/float-out-boy/src/main.c:924-930`.
    #[inline]
    pub(super) fn from_torques(
        rate_damping: MotorTorque,
        booster: MotorTorque,
        motor_torque_constant: MotorTorqueConstant,
        softstart_pid_limit: MotorCurrentLimit,
        motor_current_max: MotorCurrentLimit,
        softstart_increment: Current,
    ) -> Self {
        PitchBasedDemand::from_torques(rate_damping, booster).with_softstart(
            motor_torque_constant,
            softstart_pid_limit,
            motor_current_max,
            softstart_increment,
        )
    }
}

pub(super) fn softstart_increment(elapsed: VescSeconds) -> Current {
    Current::from_amps(SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND * valid_elapsed_seconds(elapsed))
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(super) struct RequestedCurrent(pub(super) MotorCurrent);

impl RequestedCurrent {
    #[inline]
    pub(super) fn clamped_to(self, limit: MotorCurrentLimit) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:941-942` clamps the requested
        // balance current to the selected magnitude while preserving sign.
        Self(limit.clamp(self.0))
    }

    #[inline]
    pub(super) fn adjusted_for_darkride(self, darkride: FloatOutBoyDarkRideState) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:944-946` flips the completed
        // RUNNING current request after limit selection and before smoothing.
        Self(match darkride {
            FloatOutBoyDarkRideState::Active => -self.0,
            FloatOutBoyDarkRideState::Upright => self.0,
        })
    }

    #[inline]
    pub(super) fn filtered_from(
        self,
        previous: MotorCurrent,
        traction_control: FloatOutBoyTractionControlState,
        elapsed: VescSeconds,
    ) -> MotorCurrent {
        match traction_control {
            // C map: Refloat main at caff10a resets the EMA to zero while its
            // darkride traction recovery is freewheeling (`src/main.c:723-728`).
            FloatOutBoyTractionControlState::Freewheeling => MotorCurrent::new(Current::ZERO),
            FloatOutBoyTractionControlState::FilteringCurrent => {
                // C map: Refloat main at caff10a updates the EMA as
                // `previous += alpha * (target - previous)` in
                // `src/main.c:723-728` and `src/filters/ema.h:37-38`.
                let alpha = EmaAlpha::from_elapsed(BALANCE_CURRENT_FILTER_CUTOFF, elapsed);
                previous + (self.0 - previous) * alpha.factor()
            }
        }
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn current_limit(self) -> MotorCurrentLimit {
        let braking = Branch::from_motor_current(self.motor_current).is_braking();

        match self.mode {
            FloatOutBoyMode::HandTest => {
                MotorCurrentLimit::new(Current::from_amps(HANDTEST_CURRENT_LIMIT_AMPS))
            }
            FloatOutBoyMode::Flywheel => {
                MotorCurrentLimit::new(Current::from_amps(FLYWHEEL_CURRENT_LIMIT_AMPS))
            }
            FloatOutBoyMode::Normal if braking => self.motor_current_min,
            FloatOutBoyMode::Normal => self.motor_current_max,
        }
    }
}

impl LoopState {
    #[inline]
    pub(super) fn with_booster_torque_and_softstart_limit(
        self,
        booster_torque: MotorTorque,
        softstart_pid_limit: MotorCurrentLimit,
    ) -> Self {
        Self {
            booster_torque,
            softstart_pid_limit,
            ..self
        }
    }

    #[inline]
    pub(super) fn with_balance_current(self, balance_current: MotorCurrent) -> Self {
        Self {
            balance_current,
            ..self
        }
    }
}
