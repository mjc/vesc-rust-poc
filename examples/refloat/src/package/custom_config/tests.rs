use super::{
    RefloatCustomConfig, install_test_refloat_runtime_state, lock_test_refloat_config_state_sources,
};
use crate::config::REFLOAT_CONFIG_LEN;
use crate::domain::{RefloatMode, RefloatRunState};
use crate::package::RefloatPackageState;
use crate::package::test_support::{
    RefloatConfigTestBytes, default_refloat_config_bytes, editable_config_from_state,
    sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
};
use vescpkg_rs::{ConfigBytes, SourceCustomConfigCallback};

fn refloat_config_with_hertz(hertz: u16) -> [u8; REFLOAT_CONFIG_LEN] {
    let mut config = default_refloat_config_bytes();
    config.edit_refloat_config(|config| {
        assert!(config.set_hertz(vescpkg_rs::SampleRate::from_hertz(f32::from(hertz))));
        assert!(config.set_meta_is_default(false));
    });
    config
}

fn source_current_config() -> Option<[u8; REFLOAT_CONFIG_LEN]> {
    RefloatCustomConfig::state_source()
        .with(|state| {
            RefloatCustomConfig::current_config(state)
                .and_then(|config| config.as_bytes().try_into().ok())
        })
        .flatten()
}

fn source_set_config(config: &[u8]) -> bool {
    RefloatCustomConfig::state_source()
        .with_mut(|state| RefloatCustomConfig::set_config(state, ConfigBytes::new(config)))
        .unwrap_or(false)
}

#[test]
fn custom_config_xml_callback_returns_upstream_settings_blob() {
    let bytes = RefloatCustomConfig::config_xml();

    // Refloat v1.2.1 returns generated `data_refloatconfig_` at
    // `third_party/refloat/src/main.c:2388-2396`, produced from
    // `third_party/refloat/src/conf/settings.xml` by
    // `third_party/refloat/src/Makefile:28-31`.
    assert_eq!(bytes.as_bytes().len(), 25_723);
    assert_eq!(
        &bytes.as_bytes()[..6],
        &[0x00, 0x05, 0x5c, 0xa1, 0x78, 0xda]
    );
}

#[test]
fn custom_config_default_callback_returns_upstream_serialized_defaults() {
    let config = RefloatCustomConfig::default_config();

    // Refloat v1.2.1 default `get_cfg` allocates a temporary config,
    // applies generated defaults, and serializes it at `third_party/refloat/src/main.c:2339-2350`.
    // The generated format comes from `third_party/refloat/src/Makefile:28-31`;
    // generated `conf/confparser.h:11-12` fixes signature/length, and
    // generated `conf/confparser.c:8-178,363-531` writes these bytes.
    assert_eq!(config.as_bytes(), default_refloat_config_bytes());
    assert_eq!(&config.as_bytes()[..4], &[0x90, 0xb7, 0xa9, 0xba]);
}

#[test]
fn stateful_custom_config_current_callback_reads_runtime_slot_state() {
    let _state_sources = lock_test_refloat_config_state_sources();
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let mut incoming = default_refloat_config_bytes();
    incoming.edit_refloat_config(|config| {
        assert!(config.set_meta_is_default(false));
    });
    assert!(RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));
    let _runtime_state = install_test_refloat_runtime_state(&mut state);

    let current = source_current_config();

    // C map: current `get_cfg` reads shared package state at
    // `third_party/refloat/src/main.c:2347-2350`; the generic Rust callback
    // now supplies that state instead of Refloat recovering firmware `ARG`.
    assert_eq!(current, Some(incoming));
}

#[test]
fn stateful_custom_config_current_callback_returns_none_without_state_source() {
    let _state_sources = lock_test_refloat_config_state_sources();

    // C map: upstream current `get_cfg` needs `Data *` to serialize
    // `d->float_conf` at `third_party/refloat/src/main.c:2347-2350`; without
    // either Rust runtime state or firmware `ARG`, no current config exists.
    assert_eq!(source_current_config(), None);
}

#[test]
fn stateful_custom_config_set_callback_writes_runtime_state() {
    let _state_sources = lock_test_refloat_config_state_sources();
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let _runtime_state = install_test_refloat_runtime_state(&mut state);
    let incoming = refloat_config_with_hertz(500);

    assert!(source_set_config(&incoming));

    // C map: upstream `set_cfg` mutates `d->float_conf` at
    // `third_party/refloat/src/main.c:2360-2368`.
    assert_eq!(source_current_config(), Some(incoming));
}

#[test]
fn stateful_custom_config_set_callback_returns_false_without_state_source() {
    let _state_sources = lock_test_refloat_config_state_sources();
    let incoming = refloat_config_with_hertz(500);

    // C map: upstream `set_cfg` needs `Data *` before storing into
    // `d->float_conf` at `third_party/refloat/src/main.c:2368`.
    assert!(!source_set_config(&incoming));
}

#[test]
fn custom_config_current_callback_reads_state_serialized_config() {
    let state = RefloatPackageState::new(sample_all_data_payloads());
    let current = RefloatCustomConfig::current_config(&state).expect("current config");

    // Upstream current `get_cfg` serializes `d->float_conf` from shared
    // package state at `third_party/refloat/src/main.c:2347-2350`; `data_init` populates it
    // from EEPROM or generated defaults at `third_party/refloat/src/main.c:1160-1185`.
    assert_eq!(current.as_bytes(), default_refloat_config_bytes());
}

#[test]
fn custom_config_set_callback_stores_serialized_config_in_state() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let mut incoming = default_refloat_config_bytes();
    incoming.edit_refloat_config(|config| {
        assert!(config.set_meta_is_default(false));
    });

    assert!(RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));
    let current = RefloatCustomConfig::current_config(&state).expect("current config");

    // Upstream `set_cfg` deserializes into `d->float_conf` at
    // `third_party/refloat/src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
    // bad signature before reading the field bytes.
    assert_eq!(current.as_bytes(), incoming);
}

#[test]
fn custom_config_set_callback_rejects_bad_signature_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let mut incoming = default_refloat_config_bytes();
    incoming[0] ^= 0xff;

    assert!(!RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));
    let current = RefloatCustomConfig::current_config(&state).expect("current config");

    // C map: `third_party/refloat/src/conf/confparser.c:187-190` rejects bad signatures before
    // any field storage.
    assert_eq!(current.as_bytes(), default_refloat_config_bytes());
}

#[test]
fn custom_config_set_callback_rejects_short_payload_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let incoming = default_refloat_config_bytes();

    assert!(!RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming[..275]),
    ));
    let current = RefloatCustomConfig::current_config(&state).expect("current config");

    // C map: `third_party/refloat/src/conf/confparser.h:11-12` fixes the serialized config
    // length; shorter buffers are rejected before storage.
    assert_eq!(current.as_bytes(), default_refloat_config_bytes());
}

#[test]
fn custom_config_set_callback_resets_is_default_flag_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let mut incoming = default_refloat_config_bytes();
    incoming.edit_refloat_config(|config| {
        assert!(config.set_meta_is_default(true));
    });

    assert!(RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));

    // Upstream clears `d->float_conf.meta.is_default` for every config
    // write at `third_party/refloat/src/main.c:2375-2377`; C map:
    // `third_party/refloat/src/conf/confparser.c:179` serializes that flag as the final byte.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().is_default());
}

#[test]
fn custom_config_set_callback_keeps_package_enabled_while_running_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads());
    let mut incoming = default_refloat_config_bytes();
    incoming.edit_refloat_config(|config| {
        assert!(config.set_disabled(true));
    });

    assert!(RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));

    // Upstream refuses to persist `disabled = true` while running at
    // `third_party/refloat/src/main.c:2369-2372`; `disabled` is serialized at
    // `third_party/refloat/src/conf/settings.xml:4064`.
    let current = editable_config_from_state(&state);
    assert!(!current.metadata().disabled());
}

#[test]
fn custom_config_set_callback_rejects_special_modes_like_refloat() {
    let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
        RefloatRunState::Ready,
        RefloatMode::HandTest,
    ));
    let mut incoming = default_refloat_config_bytes();
    incoming[4] = 0x12;

    assert!(!RefloatCustomConfig::set_config(
        &mut state,
        ConfigBytes::new(&incoming),
    ));
    let current = RefloatCustomConfig::current_config(&state).expect("current config");

    // Upstream rejects VESC Tool config writes outside `MODE_NORMAL` at
    // `third_party/refloat/src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
    assert_eq!(current.as_bytes(), default_refloat_config_bytes());
}
