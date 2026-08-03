use super::*;
use vescpkg_rs::prelude::SystemTicks;

pub(in crate::package::state) struct ActiveReverseStopFaultInput {
    pub(in crate::package::state) footpad: FloatOutBoyFootpadState,
    pub(in crate::package::state) darkride: FloatOutBoyDarkRideState,
    pub(in crate::package::state) pitch: AngleDegrees,
    pub(in crate::package::state) elapsed: SystemTicks,
    pub(in crate::package::state) total_erpm: Rpm,
}

impl ActiveReverseStopFaultInput {
    #[must_use]
    pub(in crate::package::state) fn stop_event(self) -> Option<FloatOutBoyStopEvent> {
        if !self.footpad.is_pressed() {
            return Some(FloatOutBoyStopEvent::ReverseStopNoFootpads);
        }
        if matches!(self.darkride, FloatOutBoyDarkRideState::Active) {
            return None;
        }
        if self.pitch > reverse_stop::PITCH {
            return Some(FloatOutBoyStopEvent::ReverseStopPitch);
        }
        let fast_timer_expired = self.pitch > reverse_stop::TIMER_FAST_PITCH
            && VescSeconds::from_seconds(1.0)
                .to_system_ticks_saturating()
                .is_some_and(|timeout| self.elapsed > timeout);
        let slow_timer_expired = self.pitch > reverse_stop::TIMER_SLOW_PITCH
            && VescSeconds::from_seconds(2.0)
                .to_system_ticks_saturating()
                .is_some_and(|timeout| self.elapsed > timeout);
        if fast_timer_expired || slow_timer_expired {
            return Some(FloatOutBoyStopEvent::ReverseStopTimer);
        }
        (self.total_erpm.abs() > reverse_stop::TOTAL_ERPM)
            .then_some(FloatOutBoyStopEvent::ReverseStopTotalErpm)
    }
}

#[must_use]
pub(in crate::package::state) fn reverse_stop_timer_inactive(pitch_abs: AngleDegrees) -> bool {
    pitch_abs <= reverse_stop::TIMER_SLOW_PITCH
}
