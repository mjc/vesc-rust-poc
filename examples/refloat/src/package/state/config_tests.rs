use super::RefloatPackageState;
use crate::config::RefloatConfigImage;
use crate::domain::{RefloatAllDataPayloads, RefloatMode, RefloatRunState};
use crate::package::test_support::{
    RefloatConfigTestBytes, default_refloat_config_bytes, editable_config_from_bytes,
    editable_config_from_state, sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::{MahonyPitchGain, MahonyRollGain};

#[test]
fn configured_loop_time_uses_refloat_hertz_config() {
    let mut incoming = default_refloat_config_bytes();
    let mut state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    assert_eq!(state.configured_loop_time_us(), 1201);

    incoming.edit_refloat_config(|config| {
        assert!(config.set_hertz(vescpkg_rs::SampleRate::from_hertz(500.0)));
    });
    assert!(state.store_serialized_config(&incoming));

    // Upstream generated serialization places `hertz` after the first
    // seven float16 config fields; `configure(d)` then uses it as
    // `1e6 / d->float_conf.hertz` at `third_party/refloat/src/main.c:190-191`.
    assert_eq!(state.configured_loop_time_us(), 2000);
}

#[test]
fn default_config_decodes_pid_scales_like_refloat_settings() {
    let state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());

    // Refloat generated settings serialize `kp` with scale 10 at
    // `third_party/refloat/src/conf/settings.xml:28-54`, `kp2` with scale
    // 100 at `third_party/refloat/src/conf/settings.xml:55-84`, and
    // `kp2_brake` with scale 100 at
    // `third_party/refloat/src/conf/settings.xml:199-222`.
    let balance = state.balance_config_for_test();
    assert_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_eq!(balance.kp2_brake().value(), 1.0);
}

#[test]
fn default_scaled_config_fields_decode_to_semantic_values() {
    let config = editable_config_from_bytes(&default_refloat_config_bytes());
    let balance = config.balance();
    let startup = config.startup();

    // C map: generated float16 fields use the offsets and scales in
    // `third_party/refloat/src/conf/settings.xml:28-222,3916-3984` and are
    // decoded by `third_party/refloat/src/conf/buffer.c:208-210`.
    assert_eq!(config.filter().mahony_kp(), MahonyPitchGain::new(2.0));
    assert_eq!(config.filter().mahony_kp_roll(), MahonyRollGain::new(1.4));
    assert_eq!(
        config.motor_control().brake_current().current().as_amps(),
        6.0
    );
    assert_eq!(startup.pitch_tolerance().as_degrees(), 4.0);
    assert_eq!(startup.roll_tolerance().as_degrees(), 45.0);
    assert_eq!(startup.startup_speed().as_degrees_per_second(), 30.0);
    assert_eq!(config.low_voltage_pushback_angle().as_degrees(), 10.0);
    assert_eq!(balance.kp().as_amps_per_degree(), 20.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.6);
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.005);
    assert_eq!(balance.kp_brake().value(), 1.0);
    assert_eq!(balance.kp2_brake().value(), 1.0);
    assert_eq!(balance.ki_limit().current().as_amps(), 30.0);
    assert_eq!(balance.booster_angle().as_degrees(), 8.0);
    assert_eq!(balance.booster_ramp().as_degrees(), 4.0);
    assert_eq!(balance.booster_current().current().as_amps(), 0.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 8.0);
    assert_eq!(balance.brake_booster_ramp().as_degrees(), 4.0);
    assert_eq!(balance.brake_booster_current().current().as_amps(), 0.0);
    assert_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        0.0
    );
    assert_eq!(config.remote_throttle().grace_period().as_seconds(), 10.0);
}

#[test]
fn semantic_config_writes_round_trip_through_generated_storage() {
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
        assert!(config.set_startup_pitch_tolerance(vescpkg_rs::AngleDegrees::from_degrees(3.5)));
        assert!(config.set_startup_roll_tolerance(vescpkg_rs::AngleDegrees::from_degrees(42.0)));
        assert!(
            config.set_startup_speed(vescpkg_rs::AngularVelocity::from_degrees_per_second(25.0,))
        );
        assert!(
            config.set_remote_throttle_current_max(vescpkg_rs::MotorCurrent::new(
                vescpkg_rs::Current::from_amps(12.0),
            ))
        );
        assert!(
            config.set_remote_throttle_grace_period(vescpkg_rs::VescSeconds::from_seconds(1.5,))
        );
        assert!(config.set_kp(vescpkg_rs::AngleCurrentGain::new(15.0)));
        assert!(config.set_kp2(vescpkg_rs::RateCurrentGain::new(0.75)));
        assert!(config.set_ki(vescpkg_rs::IntegralCurrentGain::new(0.004)));
        assert!(config.set_kp_brake(vescpkg_rs::PidScale::new(0.8)));
        assert!(config.set_booster_angle(vescpkg_rs::AngleDegrees::from_degrees(7.0)));
        assert!(config.set_booster_ramp(vescpkg_rs::AngleDegrees::from_degrees(2.5)));
        assert!(config.set_booster_current(vescpkg_rs::MotorCurrent::new(
            vescpkg_rs::Current::from_amps(4.0),
        )));
        assert!(config.set_brake_booster_angle(vescpkg_rs::AngleDegrees::from_degrees(6.0)));
        assert!(config.set_brake_booster_ramp(vescpkg_rs::AngleDegrees::from_degrees(2.0)));
        assert!(
            config.set_brake_booster_current(vescpkg_rs::MotorCurrent::new(
                vescpkg_rs::Current::from_amps(3.0),
            ))
        );
    });

    let config = editable_config_from_bytes(&bytes);
    let balance = config.balance();
    assert_eq!(config.startup().pitch_tolerance().as_degrees(), 3.5);
    assert_eq!(config.startup().roll_tolerance().as_degrees(), 42.0);
    assert_eq!(
        config.startup().startup_speed().as_degrees_per_second(),
        25.0
    );
    assert_eq!(
        config.remote_throttle().current_max().current().as_amps(),
        12.0
    );
    assert_eq!(config.remote_throttle().grace_period().as_seconds(), 1.5);
    assert_eq!(balance.kp().as_amps_per_degree(), 15.0);
    assert_eq!(balance.kp2().as_amps_per_degree_per_second(), 0.75);
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.004);
    assert_eq!(balance.kp_brake().value(), 0.8);
    assert_eq!(balance.booster_angle().as_degrees(), 7.0);
    assert_eq!(balance.booster_ramp().as_degrees(), 2.5);
    assert_eq!(balance.booster_current().current().as_amps(), 4.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 6.0);
    assert_eq!(balance.brake_booster_ramp().as_degrees(), 2.0);
    assert_eq!(balance.brake_booster_current().current().as_amps(), 3.0);
}

#[test]
fn parking_brake_mode_field_decodes_known_and_unknown_values() {
    let mut bytes = default_refloat_config_bytes();
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::RefloatParkingBrakeMode::Idle
    );

    bytes[101] = 0xff;
    assert_eq!(
        editable_config_from_bytes(&bytes)
            .motor_control()
            .parking_brake_mode(),
        crate::config::RefloatParkingBrakeMode::Unknown(0xff)
    );
}

#[test]
fn handtest_safety_overrides_encode_named_semantic_values() {
    let mut config = RefloatConfigImage::defaults();
    assert!(config.editor().apply_handtest_safety_overrides());

    let balance = config.balance();
    assert_eq!(balance.ki().as_amps_per_degree_per_tick(), 0.0);
    assert_eq!(balance.kp_brake().value(), 1.0);
    assert_eq!(balance.kp2_brake().value(), 1.0);
    assert_eq!(balance.booster_angle().as_degrees(), 100.0);
    assert_eq!(balance.brake_booster_angle().as_degrees(), 100.0);
    assert_eq!(config.faults().pitch_delay().as_seconds(), 0.05);
    assert_eq!(config.faults().roll_delay().as_seconds(), 0.05);

    // These currently unwired tune categories have no domain readers yet;
    // verify their generated float16 storage at the serializer boundary.
    for offset in [67, 71, 126, 128, 130, 145, 147] {
        assert_eq!(&config.as_bytes()[offset..offset + 2], &[0, 0]);
    }
}

#[test]
fn refloat_config_image_rejects_short_payload_like_confparser() {
    let bytes = default_refloat_config_bytes();

    // C map: `third_party/refloat/src/conf/confparser.h:11-12` fixes the serialized config length,
    // so shorter payloads must fail before any typed parsing or state mutation.
    assert!(RefloatConfigImage::from_serialized(&bytes[..275]).is_none());
}

#[test]
fn refloat_config_image_rejects_bad_signature_like_confparser() {
    let mut bytes = default_refloat_config_bytes();
    bytes[0] ^= 0xff;

    // C map: `third_party/refloat/src/conf/confparser.c:187-190` rejects the signature before field reads.
    assert!(RefloatConfigImage::from_serialized(&bytes).is_none());
}

#[test]
fn store_serialized_config_rejects_short_payload_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let bytes = default_refloat_config_bytes();

    assert!(!state.store_serialized_config(&bytes[..275]));

    // C map: upstream rejects truncated custom-config writes before storing them at
    // `third_party/refloat/src/main.c:2360-2368`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_bad_signature_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::Normal,
    ));
    let mut bytes = default_refloat_config_bytes();
    bytes[0] ^= 0xff;

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects bad config signatures before deserializing any fields at
    // `third_party/refloat/src/conf/confparser.c:187-190`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_rejects_special_modes_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::HandTest,
    ));
    let bytes = default_refloat_config_bytes();

    assert!(!state.store_serialized_config(&bytes));

    // C map: upstream rejects config writes outside `MODE_NORMAL` at
    // `third_party/refloat/src/main.c:2362-2365`.
    assert_eq!(
        state.serialized_config(),
        default_refloat_config_bytes().as_ref()
    );
}

#[test]
fn store_serialized_config_clears_default_and_keeps_enabled_while_running_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Running,
        RefloatMode::Normal,
    ));
    let mut bytes = default_refloat_config_bytes();
    bytes.edit_refloat_config(|config| {
        assert!(config.set_disabled(true));
        assert!(config.set_meta_is_default(true));
    });

    assert!(state.store_serialized_config(&bytes));

    // C map: running writes clear `disabled` at `third_party/refloat/src/main.c:2369-2372`
    // and always clear `meta.is_default` at `third_party/refloat/src/main.c:2375-2377`.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().disabled());
    assert!(!current.metadata().is_default());
}
