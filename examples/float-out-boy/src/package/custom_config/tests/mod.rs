use super::{
    FloatOutBoyCustomConfig, install_test_float_out_boy_runtime_state,
    lock_test_float_out_boy_config_state, set_float_out_boy_custom_config_for_test,
};
use crate::config::FLOAT_OUT_BOY_CONFIG_LEN;
use crate::domain::{FloatOutBoyAllDataPayloads, FloatOutBoyMode, FloatOutBoyRunState};
use crate::package::FloatOutBoyPackageState;
use crate::package::test_support::{
    FloatOutBoyConfigTestBytes, default_float_out_boy_config_bytes, editable_config_from_state,
    sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::test_support::{FirmwareTest, invoke_stateful_custom_config_handler};
use vescpkg_rs::{StatefulCustomConfigCallback, TimestampTicks};

fn nondefault_float_out_boy_config() -> [u8; FLOAT_OUT_BOY_CONFIG_LEN] {
    let mut config = default_float_out_boy_config_bytes();
    config.edit_float_out_boy_config(|config| {
        assert!(config.set_startup_pitch_tolerance(vescpkg_rs::AngleDegrees::from_degrees(7.0)));
        assert!(config.set_meta_is_default(false));
    });
    config
}

fn runtime_current_config() -> Option<[u8; FLOAT_OUT_BOY_CONFIG_LEN]> {
    crate::__VESCPKG_PACKAGE_STATE
        .with(|state| *FloatOutBoyCustomConfig::current_config(state).as_bytes())
}

fn runtime_set_config(config: &[u8; FLOAT_OUT_BOY_CONFIG_LEN]) -> bool {
    invoke_stateful_custom_config_handler::<FloatOutBoyCustomConfig, FLOAT_OUT_BOY_CONFIG_LEN>(
        config,
    )
}

#[test]
fn custom_config_xml_callback_returns_float_out_boy_settings_blob() {
    let bytes = FloatOutBoyCustomConfig::config_xml();

    assert_eq!(bytes.as_bytes().len(), 27_514);
    assert_eq!(
        &bytes.as_bytes()[..6],
        &[0x00, 0x05, 0x9e, 0xf7, 0x78, 0xda]
    );
}

#[test]
fn custom_config_default_callback_returns_upstream_serialized_defaults() {
    let config = FloatOutBoyCustomConfig::default_config();

    // The pinned cutoff default `get_cfg` allocates a temporary config,
    // applies generated defaults, and serializes it at `third_party/float-out-boy/src/main.c:2339-2350`.
    // The generated format comes from `third_party/float-out-boy/src/Makefile:28-31`;
    // generated `conf/confparser.h:11-12` fixes signature/length, and
    // generated `conf/confparser.c:8-178,363-531` writes these bytes.
    assert_eq!(*config.as_bytes(), default_float_out_boy_config_bytes());
    assert_eq!(&config.as_bytes()[..4], &[0x19, 0x1a, 0x6c, 0x1b]);
}

#[test]
fn stateful_custom_config_current_callback_reads_runtime_slot_state() {
    let _state_lock = lock_test_float_out_boy_config_state();
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_meta_is_default(false));
    });
    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    let runtime_state = install_test_float_out_boy_runtime_state(&mut state);
    assert!(runtime_state.is_some());

    let current = runtime_current_config();

    // C map: current `get_cfg` reads shared package state at
    // `third_party/float-out-boy/src/main.c:2347-2350`; the generic Rust callback
    // now supplies that state instead of Float Out Boy recovering firmware `ARG`.
    assert_eq!(current, Some(incoming));
}

#[test]
fn stateful_custom_config_current_callback_returns_none_without_runtime_state() {
    let _state_lock = lock_test_float_out_boy_config_state();

    // C map: upstream current `get_cfg` needs `Data *` to serialize
    // `d->float_conf` at `third_party/float-out-boy/src/main.c:2347-2350`; without
    // either Rust runtime state or firmware `ARG`, no current config exists.
    assert_eq!(runtime_current_config(), None);
}

#[test]
fn stateful_custom_config_set_callback_writes_runtime_state() {
    let _state_lock = lock_test_float_out_boy_config_state();
    let firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let runtime_state = install_test_float_out_boy_runtime_state(&mut state);
    assert!(runtime_state.is_some());
    let incoming = nondefault_float_out_boy_config();

    assert!(runtime_set_config(&incoming));
    let persisted = firmware
        .with_effects(|effects| firmware.eeprom().read_image::<320>(effects))
        .expect("custom-config set must write the complete EEPROM image");

    // C map: upstream `set_cfg` mutates `d->float_conf` at
    // `third_party/float-out-boy/src/main.c:2360-2368`.
    assert_eq!(runtime_current_config(), Some(incoming));
    assert_eq!(&persisted[..incoming.len()], &incoming);
}

#[test]
fn stateful_custom_config_set_callback_returns_false_without_runtime_state() {
    let _state_lock = lock_test_float_out_boy_config_state();
    let incoming = nondefault_float_out_boy_config();

    // C map: upstream `set_cfg` needs `Data *` before storing into
    // `d->float_conf` at `third_party/float-out-boy/src/main.c:2368`.
    assert!(!runtime_set_config(&incoming));
}

#[test]
fn custom_config_current_callback_reads_state_serialized_config() {
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let current = FloatOutBoyCustomConfig::current_config(&state);

    // Upstream current `get_cfg` serializes `d->float_conf` from shared
    // package state at `third_party/float-out-boy/src/main.c:2347-2350`; `data_init` populates it
    // from EEPROM or generated defaults at `third_party/float-out-boy/src/main.c:1160-1185`.
    assert_eq!(*current.as_bytes(), default_float_out_boy_config_bytes());
}

#[test]
fn custom_config_set_callback_stores_serialized_config_in_state() {
    let firmware = FirmwareTest::new();
    firmware.set_clock_ticks(1_500);
    let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    state.replace_idle_epoch_for_test(TimestampTicks::from_ticks(7));
    let mut incoming = default_float_out_boy_config_bytes();
    incoming[232] = crate::lcm::FloatOutBoyLedMode::Internal.id();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_meta_is_default(false));
    });

    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    let current = FloatOutBoyCustomConfig::current_config(&state);

    // Upstream `set_cfg` deserializes into `d->float_conf` at
    // `third_party/float-out-boy/src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
    // bad signature before reading the field bytes.
    assert_eq!(*current.as_bytes(), incoming);
    assert_eq!(
        state.idle_epoch_for_test(),
        TimestampTicks::from_ticks(1_500)
    );
    assert_eq!(state.internal_led_confirmation_start_for_test(), Some(0.15));
}

#[test]
fn custom_config_set_callback_rejects_bad_signature_like_float_out_boy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming[0] ^= 0xff;

    assert!(!set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    let current = FloatOutBoyCustomConfig::current_config(&state);

    // C map: `third_party/float-out-boy/src/conf/confparser.c:187-190` rejects bad signatures before
    // any field storage.
    assert_eq!(*current.as_bytes(), default_float_out_boy_config_bytes());
}

#[test]
fn custom_config_set_callback_rejects_zero_filter_time_constant_before_persistence() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_atr_filter_time_constant(vescpkg_rs::VescSeconds::ZERO));
    });

    assert!(!set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    assert_ne!(state.configured_loop_time_us(), u32::MAX);
    assert_eq!(
        *FloatOutBoyCustomConfig::current_config(&state).as_bytes(),
        default_float_out_boy_config_bytes()
    );
}

#[test]
fn custom_config_set_callback_rejects_full_input_deadband() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_input_tilt_deadband(vescpkg_rs::Ratio::from_ratio_const(1.0)));
    });

    assert!(!set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    assert_eq!(
        *FloatOutBoyCustomConfig::current_config(&state).as_bytes(),
        default_float_out_boy_config_bytes()
    );
}

#[test]
fn custom_config_set_callback_rejects_zero_runtime_divisors() {
    let _firmware = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    for offset in [153, 250, 254, 258] {
        let mut incoming = default_float_out_boy_config_bytes();
        incoming[offset..offset + 2].fill(0);
        assert!(
            state.prepare_serialized_config(&incoming).is_none(),
            "zero divisor at offset {offset} must be rejected"
        );
    }

    let mut incoming = default_float_out_boy_config_bytes();
    incoming[155] = 0;
    assert!(state.prepare_serialized_config(&incoming).is_none());
}

#[test]
fn custom_config_set_callback_rejects_invalid_modes_and_led_layout() {
    let _firmware = FirmwareTest::new();
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    for (offset, value) in [(99, 3), (118, u8::MAX), (232, u8::MAX)] {
        let mut incoming = default_float_out_boy_config_bytes();
        incoming[offset] = value;
        assert!(
            state.prepare_serialized_config(&incoming).is_none(),
            "invalid mode at offset {offset} must be rejected"
        );
    }
}

#[test]
fn custom_config_set_callback_resets_is_default_flag_like_float_out_boy() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_meta_is_default(true));
    });

    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));

    // Upstream clears `d->float_conf.meta.is_default` for every config
    // write at `third_party/float-out-boy/src/main.c:2375-2377`; C map:
    // `third_party/float-out-boy/src/conf/confparser.c:179` serializes that flag as the final byte.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().is_default());
}

#[test]
fn custom_config_set_callback_keeps_package_enabled_while_running_like_float_out_boy() {
    let _firmware = FirmwareTest::new();
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());
    let mut incoming = default_float_out_boy_config_bytes();
    incoming.edit_float_out_boy_config(|config| {
        assert!(config.set_disabled(true));
    });

    assert!(set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));

    // Upstream refuses to persist `disabled = true` while running at
    // `third_party/float-out-boy/src/main.c:2369-2372`; `disabled` is serialized at
    // `third_party/float-out-boy/src/conf/settings.xml:4064`.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().disabled());
}

#[test]
fn custom_config_set_callback_rejects_special_modes_like_float_out_boy() {
    let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
        FloatOutBoyRunState::Ready,
        FloatOutBoyMode::HandTest,
    ));
    let mut incoming = default_float_out_boy_config_bytes();
    incoming[4] = 0x12;

    assert!(!set_float_out_boy_custom_config_for_test(
        &mut state, &incoming
    ));
    let current = FloatOutBoyCustomConfig::current_config(&state);

    // Upstream rejects VESC Tool config writes outside `MODE_NORMAL` at
    // `third_party/float-out-boy/src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
    assert_eq!(*current.as_bytes(), default_float_out_boy_config_bytes());
}
