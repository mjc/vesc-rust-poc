use std::vec::Vec;

use super::{
    FloatOutBoyBeeper, FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount, FloatOutBoyBeeperLevel,
};
use crate::config::FloatOutBoyConfigImage;

#[test]
fn idle_beeper_tick_stays_quiet() {
    let mut beeper = FloatOutBoyBeeper::new(true);

    assert_eq!(beeper.tick(), None);
}

#[test]
fn completed_alert_tick_stays_quiet() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
    for _ in 0..240 {
        let _ = beeper.tick();
    }

    assert_eq!(beeper.tick(), None);
}

#[test]
fn beeper_enable_decodes_exact_float_out_boy_generated_offset() {
    let mut config = FloatOutBoyConfigImage::defaults();
    assert!(!config.beeper_enabled());

    assert!(config.editor().set_beeper_enabled(true));

    assert!(config.beeper_enabled());
    assert_eq!(config.as_bytes()[242], 1);
}

#[test]
fn continuous_warning_flags_decode_exact_float_out_boy_generated_offsets() {
    let config = FloatOutBoyConfigImage::defaults();

    assert!(config.foot_beep_enabled());
    assert!(!config.duty_beep_enabled());
    assert_eq!(config.as_bytes()[28], 1);
    assert_eq!(config.as_bytes()[50], 0);
}

#[test]
fn three_short_alert_matches_float_out_boy_transition_sequence() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));

    let changes: Vec<_> = (1..=560)
        .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
        .collect();

    assert_eq!(
        changes,
        [
            (80, FloatOutBoyBeeperLevel::Low),
            (160, FloatOutBoyBeeperLevel::High),
            (240, FloatOutBoyBeeperLevel::Low),
            (320, FloatOutBoyBeeperLevel::High),
            (400, FloatOutBoyBeeperLevel::Low),
            (480, FloatOutBoyBeeperLevel::High),
            (560, FloatOutBoyBeeperLevel::Low),
        ]
    );
}

#[test]
fn three_long_alert_uses_float_out_boy_long_period() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::THREE));

    let changes: Vec<_> = (1..=2_100)
        .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
        .collect();

    assert_eq!(
        changes,
        [
            (300, FloatOutBoyBeeperLevel::Low),
            (600, FloatOutBoyBeeperLevel::High),
            (900, FloatOutBoyBeeperLevel::Low),
            (1_200, FloatOutBoyBeeperLevel::High),
            (1_500, FloatOutBoyBeeperLevel::Low),
            (1_800, FloatOutBoyBeeperLevel::High),
            (2_100, FloatOutBoyBeeperLevel::Low),
        ]
    );
}

#[test]
fn four_short_alert_uses_float_out_boy_transition_count() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::FOUR));

    let changes: Vec<_> = (1..=720)
        .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
        .collect();

    assert_eq!(changes.len(), 9);
    assert_eq!(changes.last(), Some(&(720, FloatOutBoyBeeperLevel::Low)));
}

#[test]
fn seven_long_alert_uses_float_out_boy_capped_transition_count() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::SEVEN));

    let changes: Vec<_> = (1..=4_500)
        .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
        .collect();

    assert_eq!(changes.len(), 15);
    assert_eq!(changes.last(), Some(&(4_500, FloatOutBoyBeeperLevel::Low)));
}

#[test]
fn continuous_beeper_respects_alert_guard_and_force_like_float_out_boy() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.alert(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));

    beeper.off();
    assert_eq!(beeper.take_level(), None);
    beeper.force_on();
    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::High));

    let changes: Vec<_> = (1..=240)
        .filter_map(|tick| beeper.tick().map(|level| (tick, level)))
        .collect();
    assert_eq!(
        changes,
        [
            (80, FloatOutBoyBeeperLevel::Low),
            (160, FloatOutBoyBeeperLevel::High),
            (240, FloatOutBoyBeeperLevel::Low),
        ]
    );

    beeper.on();
    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::High));
    beeper.off();
    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::Low));
}

#[test]
fn disabled_beeper_rejects_on_but_still_allows_forced_off_like_float_out_boy() {
    let mut beeper = FloatOutBoyBeeper::new(false);

    beeper.force_on();
    assert_eq!(beeper.take_level(), None);
    beeper.force_off();
    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::Low));
}

#[test]
fn disabling_an_active_beeper_avoids_refloats_stuck_high_bug() {
    let mut beeper = FloatOutBoyBeeper::new(true);
    beeper.force_on();
    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::High));

    beeper.set_enabled(false);

    assert_eq!(beeper.take_level(), Some(FloatOutBoyBeeperLevel::Low));
}
