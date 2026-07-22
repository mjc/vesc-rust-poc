use vescpkg_rs::prelude::{AngleDegrees, Ratio, Rpm, SignedRatio, VescSeconds};

#[derive(Debug, Clone, Copy, PartialEq)]
struct ReverseStopRate {
    angle: AngleDegrees,
    erpm: Rpm,
}

impl ReverseStopRate {
    const FLOAT_OUT_BOY: Self = Self {
        angle: AngleDegrees::from_degrees(0.08),
        erpm: Rpm::from_revolutions_per_minute(1_000.0),
    };

    #[must_use]
    fn angle_for(self, erpm: Rpm) -> AngleDegrees {
        self.angle * (erpm / self.erpm)
    }

    #[must_use]
    fn erpm_for(self, angle: AngleDegrees) -> Rpm {
        self.erpm * (angle / self.angle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct QuickStopLimits {
    pub(super) stopped_erpm: Rpm,
    pub(super) pitch: AngleDegrees,
}

impl QuickStopLimits {
    // C map: parking-brake quickstop thresholds at `third_party/float-out-boy/src/main.c:419-421`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
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
    rate: ReverseStopRate,
}

impl ReverseStopLimits {
    // C map: reverse-stop entry and fault thresholds at
    // `third_party/float-out-boy/src/main.c:436-455` and
    // `third_party/float-out-boy/src/main.c:538-552`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
        entry_erpm: Rpm::from_revolutions_per_minute(200.0),
        tolerance_erpm: Rpm::from_revolutions_per_minute(20_000.0),
        total_erpm: Rpm::from_revolutions_per_minute(200_000.0),
        pitch: AngleDegrees::from_degrees(18.0),
        timer_fast_pitch: AngleDegrees::from_degrees(10.0),
        timer_slow_pitch: AngleDegrees::from_degrees(5.0),
        rate: ReverseStopRate::FLOAT_OUT_BOY,
    };

    #[must_use]
    pub(super) fn carryover_total_erpm(self, interpolated_target: AngleDegrees) -> Rpm {
        // C map: preserve an error-pushback target when entering reverse-stop
        // at `third_party/float-out-boy/src/main.c:541-546`.
        -(self.tolerance_erpm + self.rate.erpm_for(interpolated_target))
    }

    #[must_use]
    pub(super) fn target_angle(self, reverse_total_erpm: Rpm) -> AngleDegrees {
        // C map: `REVSTOP_ERPM_INCR` and the target calculation at
        // `third_party/float-out-boy/src/main.c:100,525-529`.
        self.rate
            .angle_for(reverse_total_erpm.abs() - self.tolerance_erpm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RemoteSetpointFaultLimit(AngleDegrees);

impl RemoteSetpointFaultLimit {
    // C map: pitch/quickstop remote-setpoint suppression at
    // `third_party/float-out-boy/src/main.c:419-421` and `third_party/float-out-boy/src/main.c:499-506`.
    pub(super) const FLOAT_OUT_BOY: Self = Self(AngleDegrees::from_degrees(30.0));

    pub(super) const fn angle(self) -> AngleDegrees {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MovingFaultLimits {
    pub(super) roll: AngleDegrees,
}

impl MovingFaultLimits {
    // C map: moving switch-fault suppression roll limit at `third_party/float-out-boy/src/main.c:393-397`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
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
    // `third_party/float-out-boy/src/main.c:361-390` and `third_party/float-out-boy/src/main.c:484-489`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
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
    // C map: push-start speed and angle thresholds at `third_party/float-out-boy/src/main.c:1055-1067`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
        erpm_min: Rpm::from_revolutions_per_minute(1000.0),
        angle: AngleDegrees::from_degrees(45.0),
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TractionLossLimits {
    pub(super) acceleration_detect: Rpm,
    pub(super) acceleration_clear: Rpm,
    pub(super) duty: SignedRatio,
    pub(super) duty_margin: Ratio,
    pub(super) clear_delay: VescSeconds,
    pub(super) raw_duty_clear: Ratio,
    pub(super) erpm: Rpm,
}

impl TractionLossLimits {
    // C map: wheelslip detection and traction-control clear thresholds at
    // `third_party/float-out-boy/src/main.c:551-575`.
    pub(super) const FLOAT_OUT_BOY: Self = Self {
        acceleration_detect: Rpm::from_revolutions_per_minute(15.0),
        acceleration_clear: Rpm::from_revolutions_per_minute(10.0),
        duty: SignedRatio::from_ratio_const(0.3),
        duty_margin: Ratio::from_ratio_const(0.05),
        clear_delay: VescSeconds::from_seconds(0.2),
        raw_duty_clear: Ratio::from_ratio_const(0.85),
        erpm: Rpm::from_revolutions_per_minute(2000.0),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_out_boy_limits_keep_source_backed_typed_values() {
        let quickstop = QuickStopLimits::FLOAT_OUT_BOY;
        assert_eq!(
            quickstop.stopped_erpm,
            Rpm::from_revolutions_per_minute(200.0)
        );
        assert_eq!(quickstop.pitch, AngleDegrees::from_degrees(14.0));

        let reverse_stop = ReverseStopLimits::FLOAT_OUT_BOY;
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
            RemoteSetpointFaultLimit::FLOAT_OUT_BOY.angle(),
            AngleDegrees::from_degrees(30.0)
        );
        assert_eq!(
            MovingFaultLimits::FLOAT_OUT_BOY.roll,
            AngleDegrees::from_degrees(40.0)
        );

        let darkride = DarkrideLimits::FLOAT_OUT_BOY;
        assert_eq!(darkride.high_erpm, Rpm::from_revolutions_per_minute(2000.0));
        assert_eq!(darkride.roll_lower, AngleDegrees::from_degrees(100.0));
        assert_eq!(darkride.roll_upper, AngleDegrees::from_degrees(135.0));

        let push_start = PushStartLimits::FLOAT_OUT_BOY;
        assert_eq!(
            push_start.erpm_min,
            Rpm::from_revolutions_per_minute(1000.0)
        );
        assert_eq!(push_start.angle, AngleDegrees::from_degrees(45.0));

        let traction_loss = TractionLossLimits::FLOAT_OUT_BOY;
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
        assert_eq!(
            traction_loss.duty_margin,
            vescpkg_rs::prelude::Ratio::from_ratio_const(0.05)
        );
        assert_eq!(traction_loss.clear_delay, VescSeconds::from_seconds(0.2));
        assert_eq!(
            traction_loss.raw_duty_clear,
            vescpkg_rs::prelude::Ratio::from_ratio_const(0.85)
        );
    }
}
