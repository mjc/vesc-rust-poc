use super::booster::Branch;
use super::loop_io::LoopInput;
use super::loop_io::LoopState;
use crate::domain::{FloatOutBoyDarkRideState, FloatOutBoyMode};
use vescpkg_rs::prelude::{Current, MotorCurrent, MotorCurrentLimit, SampleRate};

// C map: upstream chooses these scalar current limits and ramp values inside
// `third_party/float-out-boy/src/main.c:924-954`.
const HANDTEST_CURRENT_LIMIT_AMPS: f32 = 7.0;
const FLYWHEEL_CURRENT_LIMIT_AMPS: f32 = 40.0;
const SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND: f32 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct PitchBasedDemand(MotorCurrent);

impl PitchBasedDemand {
    #[inline(always)]
    fn from_terms(rate_p: MotorCurrent, booster: MotorCurrent) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:926-930` adds the rate-P and
        // booster terms before soft-start and current limiting.
        Self(rate_p + booster)
    }

    #[inline(always)]
    fn with_softstart(
        self,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrentLimit,
        hertz: SampleRate,
    ) -> PitchBasedCurrent {
        if softstart_pid_limit.current() < motor_current_max.current() {
            let pitch_based = self.0.current();
            PitchBasedCurrent {
                // C map: `third_party/float-out-boy/src/main.c:927-929` clamps only
                // magnitude; sign remains the requested direction.
                current: MotorCurrent::new(
                    pitch_based.abs().min(softstart_pid_limit.current()) * pitch_based.signum(),
                ),
                // C map: `third_party/float-out-boy/src/main.c:927-929` advances the
                // soft-start current limit at 100 A/s.
                softstart_pid_limit: softstart_pid_limit
                    + MotorCurrent::new(Current::from_amps(
                        SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND / hertz.as_hertz().max(1.0),
                    )),
            }
        } else {
            PitchBasedCurrent {
                current: self.0,
                softstart_pid_limit,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PitchBasedCurrent {
    pub(super) current: MotorCurrent,
    pub(super) softstart_pid_limit: MotorCurrent,
}

impl PitchBasedCurrent {
    /// Source map: upstream soft-start clamps pitch-based current at
    /// `third_party/float-out-boy/src/main.c:924-930`.
    #[inline(always)]
    pub(super) fn from_rate_and_booster(
        rate_p: MotorCurrent,
        booster: MotorCurrent,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrentLimit,
        hertz: SampleRate,
    ) -> Self {
        PitchBasedDemand::from_terms(rate_p, booster).with_softstart(
            softstart_pid_limit,
            motor_current_max,
            hertz,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(super) struct RequestedCurrent(pub(super) MotorCurrent);

impl RequestedCurrent {
    #[inline(always)]
    fn zero() -> MotorCurrent {
        // C map: traction-control filter zeroes the request in
        // `third_party/float-out-boy/src/main.c:949-954`.
        MotorCurrent::new(Current::ZERO)
    }

    #[inline(always)]
    pub(super) fn clamped_to(self, limit: MotorCurrentLimit) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:941-942` clamps the requested
        // balance current to the selected magnitude while preserving sign.
        Self(limit.clamp(self.0))
    }

    #[inline(always)]
    pub(super) fn adjusted_for_darkride(self, darkride: FloatOutBoyDarkRideState) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:944-946` flips the completed
        // RUNNING current request after limit selection and before smoothing.
        Self(match darkride {
            FloatOutBoyDarkRideState::Active => -self.0,
            FloatOutBoyDarkRideState::Upright => self.0,
        })
    }

    #[inline(always)]
    pub(super) fn filtered_from(
        self,
        previous: MotorCurrent,
        traction_control: bool,
    ) -> MotorCurrent {
        if traction_control {
            Self::zero()
        } else {
            // C map: `third_party/float-out-boy/src/main.c:949-954` filters RUNNING
            // output current with 20% of the new request.
            previous * 0.8 + self.0 * 0.2
        }
    }
}

impl LoopInput {
    #[inline(always)]
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
    #[inline(always)]
    pub(super) fn with_booster_current_and_softstart_limit(
        self,
        booster_current: MotorCurrent,
        softstart_pid_limit: MotorCurrent,
    ) -> Self {
        Self {
            booster_current,
            softstart_pid_limit,
            ..self
        }
    }

    #[inline(always)]
    pub(super) fn with_balance_current(self, balance_current: MotorCurrent) -> Self {
        Self {
            balance_current,
            ..self
        }
    }
}
