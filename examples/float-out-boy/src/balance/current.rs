use super::booster::Branch;
use super::loop_io::LoopInput;
use super::loop_io::LoopState;
use crate::domain::{FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyTractionControlState};
use vescpkg_rs::prelude::{Current, Frequency, MotorCurrent, MotorCurrentLimit, SampleRate};

// C map: upstream chooses these scalar current limits and ramp values inside
// `third_party/float-out-boy/src/main.c:924-954`.
const HANDTEST_CURRENT_LIMIT_AMPS: f32 = 7.0;
const FLYWHEEL_CURRENT_LIMIT_AMPS: f32 = 40.0;
const SOFTSTART_CURRENT_RAMP_AMPS_PER_SECOND: f32 = 100.0;
const BALANCE_CURRENT_FILTER_CUTOFF: Frequency = Frequency::from_hertz(25.0);

#[inline]
fn balance_current_filter_alpha(sample_rate: SampleRate) -> f32 {
    // C map: Refloat main at caff10a configures this 25 Hz filter in
    // `src/main.c:168-175` and uses the bounded second-order approximation of
    // `1 - e^-omega` from `src/filters/ema.c:24-30`.
    let omega = (2.0 * core::f32::consts::PI * BALANCE_CURRENT_FILTER_CUTOFF.as_hertz()
        / sample_rate.as_hertz())
    .min(0.5);
    omega - 0.5 * omega * omega
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct PitchBasedDemand(MotorCurrent);

impl PitchBasedDemand {
    #[inline]
    fn from_terms(rate_p: MotorCurrent, booster: MotorCurrent) -> Self {
        // C map: `third_party/float-out-boy/src/main.c:926-930` adds the rate-P and
        // booster terms before soft-start and current limiting.
        Self(rate_p + booster)
    }

    #[inline]
    fn with_softstart(
        self,
        softstart_pid_limit: MotorCurrent,
        motor_current_max: MotorCurrentLimit,
        hertz: SampleRate,
    ) -> PitchBasedCurrent {
        if softstart_pid_limit.current() < motor_current_max.current() {
            PitchBasedCurrent {
                // C map: `third_party/float-out-boy/src/main.c:927-929` clamps only
                // magnitude; sign remains the requested direction.
                current: MotorCurrentLimit::new(softstart_pid_limit.current()).clamp(self.0),
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
    #[inline]
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
        sample_rate: SampleRate,
        traction_control: FloatOutBoyTractionControlState,
    ) -> MotorCurrent {
        match traction_control {
            FloatOutBoyTractionControlState::Active => Self::zero(),
            FloatOutBoyTractionControlState::Inactive => {
                // C map: Refloat main at caff10a updates the EMA as
                // `previous += alpha * (target - previous)` in
                // `src/main.c:723-728` and `src/filters/ema.h:37-38`.
                previous + (self.0 - previous) * balance_current_filter_alpha(sample_rate)
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

    #[inline]
    pub(super) fn with_balance_current(self, balance_current: MotorCurrent) -> Self {
        Self {
            balance_current,
            ..self
        }
    }
}
