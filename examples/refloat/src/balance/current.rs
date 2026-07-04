use super::booster::Branch;
use super::loop_io::LoopInput;
use super::loop_io::LoopState;
use crate::domain::{RefloatDarkRideState, RefloatMode};
use vescpkg_rs::prelude::{Current, MotorCurrent, SampleRate};

// C map: upstream chooses these scalar current limits and ramp values inside
// `third_party/refloat/src/main.c:924-954`.
const HANDTEST_CURRENT_LIMIT_AMPS: f32 = 7.0;
const FLYWHEEL_CURRENT_LIMIT_AMPS: f32 = 40.0;
const SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND: f32 = 100.0;

/// Positive magnitude used to clamp Refloat RUNNING balance current.
///
/// Source map: upstream chooses a scalar `current_limit` at
/// `third_party/refloat/src/main.c:932-940`, then clamps with
/// `fabsf(new_current) > current_limit` at `third_party/refloat/src/main.c:941-942`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct LimitMagnitude(Current);

impl LimitMagnitude {
    #[inline(always)]
    pub(super) fn from_motor_current(limit: MotorCurrent) -> Self {
        // C map: `third_party/refloat/src/main.c:934-940` selects the active
        // motor limit as a signed value and we keep only its magnitude here.
        Self(limit.current().abs())
    }

    #[inline(always)]
    fn from_amps(amps: f32) -> Self {
        // C map: upstream hardcodes 7A and 40A mode limits at
        // `third_party/refloat/src/main.c:934-940`.
        Self(Current::from_amps(amps))
    }

    #[inline(always)]
    fn is_enabled(self) -> bool {
        self.0.is_positive()
    }

    #[inline(always)]
    pub(super) fn clamp(self, current: MotorCurrent) -> MotorCurrent {
        let requested = current.current();
        // C map: `third_party/refloat/src/main.c:941-942` treats the limit as
        // a positive magnitude and preserves command sign.
        if self.is_enabled() && requested.abs() > self.0 {
            MotorCurrent::new(self.0 * requested.signum())
        } else {
            current
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct PitchBasedDemand(MotorCurrent);

impl PitchBasedDemand {
    #[inline(always)]
    fn from_terms(rate_p: MotorCurrent, booster: MotorCurrent) -> Self {
        // C map: `third_party/refloat/src/main.c:926-930` adds the rate-P and
        // booster terms before soft-start and current limiting.
        Self(rate_p + booster)
    }

    #[inline(always)]
    fn with_softstart(
        self,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrent,
        hertz: SampleRate,
    ) -> PitchBasedCurrent {
        if softstart_pid_limit < motor_current_max {
            let pitch_based = self.0.current();
            PitchBasedCurrent {
                // C map: `third_party/refloat/src/main.c:927-929` clamps only
                // magnitude; sign remains the requested direction.
                current: MotorCurrent::new(
                    pitch_based.abs().min(softstart_pid_limit.current()) * pitch_based.signum(),
                ),
                // C map: `third_party/refloat/src/main.c:927-929` advances the
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
    /// `third_party/refloat/src/main.c:924-930`.
    #[inline(always)]
    pub(super) fn from_rate_and_booster(
        rate_p: MotorCurrent,
        booster: MotorCurrent,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrent,
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
        // `third_party/refloat/src/main.c:949-954`.
        MotorCurrent::new(Current::ZERO)
    }

    #[inline(always)]
    pub(super) fn clamped_to(self, limit: LimitMagnitude) -> Self {
        // C map: `third_party/refloat/src/main.c:941-942` clamps the requested
        // balance current to the selected magnitude while preserving sign.
        Self(limit.clamp(self.0))
    }

    #[inline(always)]
    pub(super) fn adjusted_for_darkride(self, darkride: RefloatDarkRideState) -> Self {
        // C map: `third_party/refloat/src/main.c:944-946` flips the completed
        // RUNNING current request after limit selection and before smoothing.
        Self(match darkride {
            RefloatDarkRideState::Active => -self.0,
            RefloatDarkRideState::Upright => self.0,
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
            // C map: `third_party/refloat/src/main.c:949-954` filters RUNNING
            // output current with 20% of the new request.
            previous * 0.8 + self.0 * 0.2
        }
    }
}

impl LoopInput {
    #[inline(always)]
    pub(super) fn current_limit(self) -> LimitMagnitude {
        let braking = Branch::from_motor_current(self.motor_current).is_braking();

        match self.mode {
            RefloatMode::HandTest => LimitMagnitude::from_amps(HANDTEST_CURRENT_LIMIT_AMPS),
            RefloatMode::Flywheel => LimitMagnitude::from_amps(FLYWHEEL_CURRENT_LIMIT_AMPS),
            RefloatMode::Normal if braking => {
                LimitMagnitude::from_motor_current(self.motor_current_min)
            }
            RefloatMode::Normal => LimitMagnitude::from_motor_current(self.motor_current_max),
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
