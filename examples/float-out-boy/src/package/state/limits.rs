use vescpkg_rs::prelude::{AngleDegrees, Rpm, SignedRatio, VescSeconds};

// These are named modules rather than one-instance structs: each value remains
// domain typed, while the namespace preserves which FOB behavior owns it.
pub(super) mod quick_stop {
    use super::{AngleDegrees, Rpm};

    // C map: parking-brake quickstop thresholds at `third_party/float-out-boy/src/main.c:419-421`.
    pub(in crate::package::state) const STOPPED_ERPM: Rpm = Rpm::from_revolutions_per_minute(200.0);
    pub(in crate::package::state) const PITCH: AngleDegrees = AngleDegrees::from_degrees(14.0);
}

// C map: pitch/quickstop remote-setpoint suppression at
// `third_party/float-out-boy/src/main.c:419-421,499-506`.
pub(super) const REMOTE_SETPOINT_FAULT_ANGLE: AngleDegrees = AngleDegrees::from_degrees(30.0);

// C map: moving switch-fault suppression roll limit at `third_party/float-out-boy/src/main.c:393-397`.
pub(super) const MOVING_FAULT_ROLL: AngleDegrees = AngleDegrees::from_degrees(40.0);

pub(super) mod darkride {
    use super::{AngleDegrees, Rpm, VescSeconds};

    // C map: darkride high-ERPM and roll faults at
    // `third_party/float-out-boy/src/main.c:361-390,484-489`.
    pub(in crate::package::state) const TIMED_HIGH_ERPM: Rpm =
        Rpm::from_revolutions_per_minute(1000.0);
    pub(in crate::package::state) const TIMED_HIGH_DELAY: VescSeconds =
        VescSeconds::from_seconds(0.1);
    pub(in crate::package::state) const HIGH_ERPM: Rpm = Rpm::from_revolutions_per_minute(2000.0);
    pub(in crate::package::state) const LOW_ERPM: Rpm = Rpm::from_revolutions_per_minute(300.0);
    pub(in crate::package::state) const LOW_DELAY: VescSeconds = VescSeconds::from_seconds(0.5);
    pub(in crate::package::state) const ROLL_LOWER: AngleDegrees =
        AngleDegrees::from_degrees(100.0);
    pub(in crate::package::state) const ROLL_UPPER: AngleDegrees =
        AngleDegrees::from_degrees(135.0);
}

pub(super) mod push_start {
    use super::{AngleDegrees, Rpm};

    // C map: push-start speed and angle thresholds at `third_party/float-out-boy/src/main.c:1055-1067`.
    pub(in crate::package::state) const ERPM_MIN: Rpm = Rpm::from_revolutions_per_minute(1000.0);
    pub(in crate::package::state) const ANGLE: AngleDegrees = AngleDegrees::from_degrees(45.0);
}

pub(super) mod traction_loss {
    use super::{Rpm, SignedRatio, VescSeconds};
    use vescpkg_rs::prelude::Ratio;

    // C map: wheelslip detection and traction-control clear thresholds at
    // `third_party/float-out-boy/src/main.c:551-575`.
    pub(in crate::package::state) const ACCELERATION_DETECT: Rpm =
        Rpm::from_revolutions_per_minute(10_000.0);
    pub(in crate::package::state) const ACCELERATION_CLEAR: Rpm =
        Rpm::from_revolutions_per_minute(7_000.0);
    pub(in crate::package::state) const DUTY: SignedRatio = SignedRatio::from_ratio_const(0.3);
    pub(in crate::package::state) const DUTY_MARGIN: Ratio = Ratio::from_ratio_const(0.05);
    pub(in crate::package::state) const CLEAR_DELAY: VescSeconds = VescSeconds::from_seconds(0.2);
    pub(in crate::package::state) const RAW_DUTY_CLEAR: Ratio = Ratio::from_ratio_const(0.85);
    pub(in crate::package::state) const ERPM: Rpm = Rpm::from_revolutions_per_minute(2000.0);
}

#[cfg(test)]
mod tests;
