use super::loop_io::LoopInput;
use crate::domain::{FloatOutBoyDarkRideState, FloatOutBoyMode};
use crate::motor_torque::{MotorTorque, MotorTorqueConstant};
use vescpkg_rs::prelude::{Current, MotorCurrent, MotorCurrentLimit, VescSeconds};

// C map: upstream chooses these scalar current limits and ramp values inside
// `third_party/float-out-boy/src/main.c:924-954`.
const HANDTEST_CURRENT_LIMIT_AMPS: f32 = 7.0;
const FLYWHEEL_CURRENT_LIMIT_AMPS: f32 = 40.0;
const SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND: f32 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PitchBasedCurrent {
    pub(super) current: MotorCurrent,
    pub(super) softstart_pid_limit: MotorCurrent,
}

impl PitchBasedCurrent {
    /// Source map: upstream soft-start clamps pitch-based current at
    /// `third_party/float-out-boy/src/main.c:924-930`.
    #[inline]
    pub(super) fn from_rate_and_booster(
        rate_p: MotorTorque,
        booster: MotorTorque,
        torque_constant: MotorTorqueConstant,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrentLimit,
        elapsed: VescSeconds,
    ) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:926-930` adds the rate-P and
        // booster terms before soft-start and current limiting.
        let demand = torque_constant.motor_current_from_torque(rate_p.plus(booster));
        if softstart_pid_limit.current() < motor_current_max.current() {
            Self {
                // C map: `third_party/float-out-boy/src/main.c:927-929` clamps only
                // magnitude; sign remains the requested direction.
                current: MotorCurrentLimit::new(softstart_pid_limit.current()).clamp(demand),
                // C map: `third_party/float-out-boy/src/main.c:927-929` advances the
                // soft-start current limit at 100 A/s.
                softstart_pid_limit: softstart_pid_limit
                    + MotorCurrent::new(Current::from_amps(
                        SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND * valid_elapsed_seconds(elapsed),
                    )),
            }
        } else {
            Self {
                current: demand,
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
#[repr(transparent)]
pub(super) struct RequestedCurrent(pub(super) MotorCurrent);

impl RequestedCurrent {
    #[inline]
    fn zero() -> MotorCurrent {
        // C map: traction-control filter zeroes the request in
        // `third_party/float-out-boy/src/main.c:949-954`.
        MotorCurrent::new(Current::ZERO)
    }

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
        traction_control: bool,
        elapsed: VescSeconds,
    ) -> MotorCurrent {
        if traction_control {
            Self::zero()
        } else {
            // C map: Refloat filters RUNNING output current with a 25 Hz EMA.
            let alpha = super::ema_alpha(25.0, elapsed);
            previous + (self.0 - previous) * alpha
        }
    }
}

impl LoopInput {
    #[inline]
    pub(super) fn current_limit(self) -> MotorCurrentLimit {
        match self.mode {
            FloatOutBoyMode::HandTest => {
                MotorCurrentLimit::new(Current::from_amps(HANDTEST_CURRENT_LIMIT_AMPS))
            }
            FloatOutBoyMode::Flywheel => {
                MotorCurrentLimit::new(Current::from_amps(FLYWHEEL_CURRENT_LIMIT_AMPS))
            }
            FloatOutBoyMode::Normal => self
                .motor_current_limits
                .for_current(self.motor_current.current()),
        }
    }
}
