// C map: Float Out Boy averages ERPM deltas over this many samples at
// `third_party/float-out-boy/src/motor_data.h:26`.
const WINDOW: usize = 40;
#[cfg(test)]
const WINDOW_U8: u8 = 40;
pub(super) const ABS_ERPM_SMOOTHING: vescpkg_rs::prelude::Ratio =
    vescpkg_rs::prelude::Ratio::from_ratio_const(0.1);
pub(super) type MotorKinematicsTracker = vescpkg_rs::MotorKinematics<WINDOW>;

#[cfg(test)]
mod tests;
