use super::*;

pub(in crate::package) fn lock_test_float_out_boy_config_state() -> impl Drop {
    super::super::test_support::lock_float_out_boy_runtime_state()
}

pub(in crate::package) fn install_test_float_out_boy_runtime_state(
    state: &mut FloatOutBoyPackageState,
) -> Option<impl Drop + '_> {
    vescpkg_rs::test_support::install_state(&crate::__VESCPKG_PACKAGE_STATE, state)
}

fn float_out_boy_set_cfg_payload_with_state(
    config: ConfigBytes<'_, FLOAT_OUT_BOY_CONFIG_LEN>,
    state: &mut FloatOutBoyPackageState,
) -> bool {
    // Upstream `set_cfg` gates special modes, deserializes, persists, and
    // reconfigures at `third_party/float-out-boy/src/main.c:2360-2386`; generated
    // `conf/confparser.c:187-190` rejects bad signatures before field reads.
    // This test helper mirrors the production state/effect/state sequence
    // without invoking the unsafe firmware pointer trampoline.
    let Some(config) = state.prepare_serialized_config(config.as_bytes()) else {
        return false;
    };
    let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
        super::super::state::store_persisted_config(effects, &config)
    });
    let now = vescpkg_rs::FirmwareClock::current_timestamp();
    state.commit_custom_config(config, stored, now);
    let migration = vescpkg_rs::test_support::with_firmware_effects(
        super::super::state::migrate_legacy_firmware_imu_settings,
    );
    state.finish_configure_active(migration);
    true
}

pub(crate) fn set_float_out_boy_custom_config_for_test(
    state: &mut FloatOutBoyPackageState,
    config: &[u8; FLOAT_OUT_BOY_CONFIG_LEN],
) -> bool {
    float_out_boy_set_cfg_payload_with_state(ConfigBytes::new(config), state)
}
