use super::*;

const SEQUENCE: &[FloatOutBoyFootpadState] = &[
    FloatOutBoyFootpadState::Left,
    FloatOutBoyFootpadState::None,
    FloatOutBoyFootpadState::Right,
];

#[test]
fn built_in_sequences_have_the_expected_element_counts() {
    assert_eq!(FloatOutBoyKonami::flywheel().sequence.len(), 8);
    assert_eq!(FloatOutBoyKonami::headlights_on().sequence.len(), 5);
    assert_eq!(FloatOutBoyKonami::headlights_off().sequence.len(), 5);
}

#[test]
fn empty_sequence_never_completes() {
    let mut konami = FloatOutBoyKonami::new(&[]);

    assert!(!konami.check(
        FloatOutBoyFootpadState::Both,
        TimestampTicks::from_ticks(u32::MAX)
    ));
    assert_eq!(konami.state, 0);
}

#[test]
fn sequence_requires_source_timing_and_completes_once() {
    let mut konami = FloatOutBoyKonami::new(SEQUENCE);
    assert!(!konami.check(FloatOutBoyFootpadState::Left, TimestampTicks::from_ticks(0)));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(1_501)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::None,
        TimestampTicks::from_ticks(3_002)
    ));
    assert!(konami.check(
        FloatOutBoyFootpadState::Right,
        TimestampTicks::from_ticks(4_503)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Right,
        TimestampTicks::from_ticks(6_004)
    ));
}

#[test]
fn wrong_state_resets_but_repeated_previous_state_is_held() {
    let mut konami = FloatOutBoyKonami::new(SEQUENCE);
    assert!(!konami.check(
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(1_501)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(2_000)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Right,
        TimestampTicks::from_ticks(3_501)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::None,
        TimestampTicks::from_ticks(5_002)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(6_503)
    ));
}

#[test]
fn incomplete_sequence_expires_after_half_second() {
    let mut konami = FloatOutBoyKonami::new(SEQUENCE);
    assert!(!konami.check(
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(1_501)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::None,
        TimestampTicks::from_ticks(7_502)
    ));
    assert!(!konami.check(
        FloatOutBoyFootpadState::Right,
        TimestampTicks::from_ticks(9_003)
    ));
}

#[test]
fn flywheel_pitch_gate_is_strict_at_both_float_out_boy_boundaries() {
    let mut konami = FloatOutBoyKonami::new(&[FloatOutBoyFootpadState::Left]);

    assert!(!konami.check_flywheel(
        ImuPitch::new(AngleRadians::from_degrees(75.0)),
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(1_501),
    ));
    assert!(konami.check_flywheel(
        ImuPitch::new(AngleRadians::from_degrees(75.1)),
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(3_002),
    ));
    assert!(!konami.check_flywheel(
        ImuPitch::new(AngleRadians::from_degrees(105.0)),
        FloatOutBoyFootpadState::Left,
        TimestampTicks::from_ticks(4_503),
    ));
}
