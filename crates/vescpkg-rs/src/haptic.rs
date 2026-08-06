//! Allocation-free haptic pulse sequencing shared by VESC packages.

use core::num::NonZeroU32;

use crate::{Ratio, Speed, TimestampTicks, WrappingTimer};

/// Nonzero duration of one haptic on/off beat in VESC system ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HapticBeatDuration(NonZeroU32);

impl HapticBeatDuration {
    /// Build a beat duration from a nonzero number of VESC system ticks.
    #[must_use]
    pub const fn from_ticks(ticks: u32) -> Option<Self> {
        match NonZeroU32::new(ticks) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    const fn as_ticks(self) -> u32 {
        self.0.get()
    }
}

/// Timing pattern for one haptic feedback kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticPulsePattern {
    /// Play without gaps until the requested feedback kind changes.
    Continuous,
    /// Alternate on/off beats in a repeating cycle.
    Repeating {
        /// Duration of each on or off beat.
        beat: HapticBeatDuration,
        /// Total number of beats in the cycle.
        beats: NonZeroU32,
    },
}

impl HapticPulsePattern {
    /// Build one repeating on/off pattern.
    #[must_use]
    pub const fn repeating(beat: HapticBeatDuration, beats: NonZeroU32) -> Self {
        Self::Repeating { beat, beats }
    }
}

/// Output requested by one haptic player update.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticPlayback<T> {
    /// No output transition is needed.
    Silent,
    /// Stop the output that was active on the preceding update.
    Stop,
    /// Play or continue playing this feedback kind.
    Play(T),
}

/// Fixed-state pulse sequencer for one package-defined haptic feedback kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HapticPulsePlayer<T> {
    active: Option<T>,
    timer: WrappingTimer,
    is_playing: bool,
    can_change_pattern: bool,
}

impl<T> Default for HapticPulsePlayer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HapticPulsePlayer<T> {
    /// Build an idle haptic pulse player.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active: None,
            timer: WrappingTimer::started_at(TimestampTicks::from_ticks(0)),
            is_playing: false,
            can_change_pattern: true,
        }
    }
}

impl<T: Copy + Eq> HapticPulsePlayer<T> {
    /// Advance the selected feedback kind and return its required output state.
    pub fn update(
        &mut self,
        requested: Option<T>,
        now: TimestampTicks,
        pattern_for: impl FnOnce(T) -> HapticPulsePattern,
    ) -> HapticPlayback<T> {
        if requested != self.active && self.can_change_pattern {
            self.active = requested;
            self.timer.restart(now);
        }

        let should_play = match self.active {
            None => {
                self.can_change_pattern = true;
                false
            }
            Some(active) => match pattern_for(active) {
                HapticPulsePattern::Continuous => {
                    self.can_change_pattern = true;
                    true
                }
                HapticPulsePattern::Repeating { beat, beats } => {
                    let beat_ticks = beat.as_ticks();
                    let cycle_ticks = beat_ticks.saturating_mul(beats.get());
                    let tone_ticks = self
                        .timer
                        .elapsed(now)
                        .as_ticks()
                        .checked_rem(cycle_ticks)
                        .unwrap_or_default();
                    let current_beat = tone_ticks / beat_ticks;
                    let omitted_off_beat = beats.get().saturating_sub(2);
                    self.can_change_pattern = !self.is_playing && current_beat == 0;
                    current_beat.is_multiple_of(2)
                        && (omitted_off_beat == 0 || current_beat != omitted_off_beat)
                }
            },
        };

        let playback = if self.is_playing && !should_play {
            HapticPlayback::Stop
        } else if let (true, Some(kind)) = (should_play, self.active) {
            HapticPlayback::Play(kind)
        } else {
            HapticPlayback::Silent
        };
        self.is_playing = should_play;
        playback
    }
}

/// Scale haptic strength quadratically from a configured minimum to full strength.
#[must_use]
pub fn haptic_strength_scale(
    speed: Speed,
    minimum: Ratio,
    maximum_speed: Speed,
    curvature: Ratio,
) -> Ratio {
    let configured_maximum = maximum_speed.as_kilometers_per_hour();
    let maximum = if configured_maximum > 0.0 {
        configured_maximum
    } else {
        1.0
    };
    let speed = speed.as_kilometers_per_hour().abs();
    let minimum = minimum.as_ratio();
    let linear = (1.0 - curvature.as_ratio()) * (1.0 - minimum) / maximum;
    let quadratic = (1.0 - minimum - linear * maximum) / (maximum * maximum);
    Ratio::clamped(minimum + linear * speed + quadratic * speed * speed)
}

#[cfg(test)]
mod tests {
    use super::{
        HapticBeatDuration, HapticPlayback, HapticPulsePattern, HapticPulsePlayer,
        haptic_strength_scale,
    };
    use crate::{Ratio, Speed, TimestampTicks};
    use core::num::NonZeroU32;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Tone {
        Warning,
        Error,
    }

    const BEAT: HapticBeatDuration = HapticBeatDuration::from_ticks(1_000).unwrap();
    const TWO_BEATS: HapticPulsePattern =
        HapticPulsePattern::repeating(BEAT, NonZeroU32::new(2).unwrap());

    #[test]
    fn pulse_player_preserves_repeating_continuous_and_stop_transitions() {
        let mut player = HapticPulsePlayer::new();

        assert_eq!(
            player.update(Some(Tone::Warning), TimestampTicks::from_ticks(0), |_| {
                TWO_BEATS
            }),
            HapticPlayback::Play(Tone::Warning)
        );
        assert_eq!(
            player.update(
                Some(Tone::Warning),
                TimestampTicks::from_ticks(1_000),
                |_| TWO_BEATS,
            ),
            HapticPlayback::Stop
        );
        assert_eq!(
            player.update(
                Some(Tone::Warning),
                TimestampTicks::from_ticks(2_000),
                |_| TWO_BEATS,
            ),
            HapticPlayback::Play(Tone::Warning)
        );
        assert_eq!(
            player.update(Some(Tone::Error), TimestampTicks::from_ticks(3_000), |_| {
                HapticPulsePattern::Continuous
            },),
            HapticPlayback::Play(Tone::Error)
        );
        assert_eq!(
            player.update(None, TimestampTicks::from_ticks(3_001), |_| TWO_BEATS),
            HapticPlayback::Stop
        );
        assert_eq!(
            player.update(None, TimestampTicks::from_ticks(3_002), |_| TWO_BEATS),
            HapticPlayback::Silent
        );
    }

    #[test]
    fn pulse_player_changes_pattern_only_at_the_source_compatible_boundary() {
        let mut player = HapticPulsePlayer::new();
        assert_eq!(
            player.update(Some(Tone::Warning), TimestampTicks::from_ticks(0), |_| {
                TWO_BEATS
            }),
            HapticPlayback::Play(Tone::Warning)
        );
        assert_eq!(
            player.update(
                Some(Tone::Warning),
                TimestampTicks::from_ticks(1_000),
                |_| TWO_BEATS,
            ),
            HapticPlayback::Stop
        );
        assert_eq!(
            player.update(
                Some(Tone::Error),
                TimestampTicks::from_ticks(2_000),
                |tone| match tone {
                    Tone::Warning => TWO_BEATS,
                    Tone::Error => HapticPulsePattern::Continuous,
                },
            ),
            HapticPlayback::Play(Tone::Warning)
        );
        assert_eq!(
            player.update(Some(Tone::Error), TimestampTicks::from_ticks(2_001), |_| {
                HapticPulsePattern::Continuous
            },),
            HapticPlayback::Play(Tone::Error)
        );
    }

    #[test]
    fn haptic_strength_curve_is_direction_independent_and_bounded() {
        let minimum = Ratio::from_ratio_const(0.2);
        let curvature = Ratio::from_ratio_const(0.6);
        let maximum_speed = Speed::from_kilometers_per_hour(30.0);
        let forward = haptic_strength_scale(
            Speed::from_kilometers_per_hour(18.0),
            minimum,
            maximum_speed,
            curvature,
        );
        let reverse = haptic_strength_scale(
            Speed::from_kilometers_per_hour(-18.0),
            minimum,
            maximum_speed,
            curvature,
        );

        assert_eq!(forward, reverse);
        assert_eq!(
            haptic_strength_scale(Speed::ZERO, minimum, maximum_speed, curvature),
            minimum
        );
        assert_eq!(
            haptic_strength_scale(
                Speed::from_kilometers_per_hour(60.0),
                minimum,
                maximum_speed,
                curvature,
            ),
            Ratio::FULL
        );
    }
}
