use vescpkg_rs::prelude::{AngleDegrees, Rpm, SignedRatio, VescSeconds};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct QuickStopLimits {
    pub(super) stopped_erpm: Rpm,
    pub(super) pitch: AngleDegrees,
}

impl QuickStopLimits {
    // C map: parking-brake quickstop thresholds at `third_party/refloat/src/main.c:419-421`.
    pub(super) const REFLOAT: Self = Self {
        stopped_erpm: Rpm::from_revolutions_per_minute(200.0),
        pitch: AngleDegrees::from_degrees(14.0),
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ReverseStopLimits {
    pub(super) entry_erpm: Rpm,
    pub(super) tolerance_erpm: Rpm,
    pub(super) total_erpm: Rpm,
    pub(super) pitch: AngleDegrees,
    pub(super) timer_fast_pitch: AngleDegrees,
    pub(super) timer_slow_pitch: AngleDegrees,
}

impl ReverseStopLimits {
    // C map: reverse-stop entry and fault thresholds at
    // `third_party/refloat/src/main.c:436-455` and
    // `third_party/refloat/src/main.c:538-552`.
    pub(super) const REFLOAT: Self = Self {
        entry_erpm: Rpm::from_revolutions_per_minute(200.0),
        tolerance_erpm: Rpm::from_revolutions_per_minute(20_000.0),
        total_erpm: Rpm::from_revolutions_per_minute(200_000.0),
        pitch: AngleDegrees::from_degrees(18.0),
        timer_fast_pitch: AngleDegrees::from_degrees(10.0),
        timer_slow_pitch: AngleDegrees::from_degrees(5.0),
    };

    pub(super) fn target_angle(self, reverse_total_erpm: Rpm) -> AngleDegrees {
        // C map: `REVSTOP_ERPM_INCR` and the target calculation at
        // `third_party/refloat/src/main.c:100,525-529`.
        AngleDegrees::from_degrees(
            (reverse_total_erpm.abs() - self.tolerance_erpm).as_revolutions_per_minute() * 0.000_08,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RemoteSetpointFaultLimit(AngleDegrees);

impl RemoteSetpointFaultLimit {
    // C map: pitch/quickstop remote-setpoint suppression at
    // `third_party/refloat/src/main.c:419-421` and `third_party/refloat/src/main.c:499-506`.
    pub(super) const REFLOAT: Self = Self(AngleDegrees::from_degrees(30.0));

    pub(super) const fn angle(self) -> AngleDegrees {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MovingFaultLimits {
    pub(super) roll: AngleDegrees,
}

impl MovingFaultLimits {
    // C map: moving switch-fault suppression roll limit at `third_party/refloat/src/main.c:393-397`.
    pub(super) const REFLOAT: Self = Self {
        roll: AngleDegrees::from_degrees(40.0),
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DarkrideLimits {
    pub(super) timed_high_erpm: Rpm,
    pub(super) timed_high_delay: VescSeconds,
    pub(super) high_erpm: Rpm,
    pub(super) low_erpm: Rpm,
    pub(super) low_delay: VescSeconds,
    pub(super) roll_lower: AngleDegrees,
    pub(super) roll_upper: AngleDegrees,
}

impl DarkrideLimits {
    // C map: darkride high-ERPM and roll faults at
    // `third_party/refloat/src/main.c:361-390` and `third_party/refloat/src/main.c:484-489`.
    pub(super) const REFLOAT: Self = Self {
        timed_high_erpm: Rpm::from_revolutions_per_minute(1000.0),
        timed_high_delay: VescSeconds::from_seconds(0.1),
        high_erpm: Rpm::from_revolutions_per_minute(2000.0),
        low_erpm: Rpm::from_revolutions_per_minute(300.0),
        low_delay: VescSeconds::from_seconds(0.5),
        roll_lower: AngleDegrees::from_degrees(100.0),
        roll_upper: AngleDegrees::from_degrees(135.0),
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PushStartLimits {
    pub(super) erpm_min: Rpm,
    pub(super) angle: AngleDegrees,
}

impl PushStartLimits {
    // C map: push-start speed and angle thresholds at `third_party/refloat/src/main.c:1055-1067`.
    pub(super) const REFLOAT: Self = Self {
        erpm_min: Rpm::from_revolutions_per_minute(1000.0),
        angle: AngleDegrees::from_degrees(45.0),
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TractionLossLimits {
    pub(super) acceleration_detect: Rpm,
    pub(super) acceleration_clear: Rpm,
    pub(super) duty: SignedRatio,
    pub(super) erpm: Rpm,
}

impl TractionLossLimits {
    // C map: wheelslip detection and traction-control clear thresholds at
    // `third_party/refloat/src/main.c:551-575`.
    pub(super) const REFLOAT: Self = Self {
        acceleration_detect: Rpm::from_revolutions_per_minute(15.0),
        acceleration_clear: Rpm::from_revolutions_per_minute(10.0),
        duty: SignedRatio::from_ratio_const(0.3),
        erpm: Rpm::from_revolutions_per_minute(2000.0),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refloat_limits_keep_source_backed_typed_values() {
        let quickstop = QuickStopLimits::REFLOAT;
        assert_eq!(
            quickstop.stopped_erpm,
            Rpm::from_revolutions_per_minute(200.0)
        );
        assert_eq!(quickstop.pitch, AngleDegrees::from_degrees(14.0));

        let reverse_stop = ReverseStopLimits::REFLOAT;
        assert_eq!(
            reverse_stop.total_erpm,
            Rpm::from_revolutions_per_minute(200_000.0)
        );
        assert_eq!(reverse_stop.pitch, AngleDegrees::from_degrees(18.0));
        assert_eq!(
            reverse_stop.timer_fast_pitch,
            AngleDegrees::from_degrees(10.0)
        );
        assert_eq!(
            reverse_stop.timer_slow_pitch,
            AngleDegrees::from_degrees(5.0)
        );

        assert_eq!(
            RemoteSetpointFaultLimit::REFLOAT.angle(),
            AngleDegrees::from_degrees(30.0)
        );
        assert_eq!(
            MovingFaultLimits::REFLOAT.roll,
            AngleDegrees::from_degrees(40.0)
        );

        let darkride = DarkrideLimits::REFLOAT;
        assert_eq!(darkride.high_erpm, Rpm::from_revolutions_per_minute(2000.0));
        assert_eq!(darkride.roll_lower, AngleDegrees::from_degrees(100.0));
        assert_eq!(darkride.roll_upper, AngleDegrees::from_degrees(135.0));

        let push_start = PushStartLimits::REFLOAT;
        assert_eq!(
            push_start.erpm_min,
            Rpm::from_revolutions_per_minute(1000.0)
        );
        assert_eq!(push_start.angle, AngleDegrees::from_degrees(45.0));

        let traction_loss = TractionLossLimits::REFLOAT;
        assert_eq!(
            traction_loss.acceleration_detect,
            Rpm::from_revolutions_per_minute(15.0)
        );
        assert_eq!(
            traction_loss.acceleration_clear,
            Rpm::from_revolutions_per_minute(10.0)
        );
        assert_eq!(traction_loss.duty, SignedRatio::from_ratio_const(0.3));
        assert_eq!(traction_loss.erpm, Rpm::from_revolutions_per_minute(2000.0));
    }
}
