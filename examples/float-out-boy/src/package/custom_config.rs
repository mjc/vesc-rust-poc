use super::FloatOutBoyPackageState;
use crate::config::{
    FLOAT_OUT_BOY_CONFIG_LEN, FLOAT_OUT_BOY_CONFIG_XML, FLOAT_OUT_BOY_DEFAULT_CONFIG,
};
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::PackageStart;
use vescpkg_rs::{ConfigBytes, ConfigXml};

/// Float Out Boy's typed custom-config callback set.
///
/// C map: `get_cfg`, `set_cfg`, and `get_cfg_xml` are implemented at
/// `third_party/float-out-boy/src/main.c:2334-2396` and registered at `:2455`.
pub struct FloatOutBoyCustomConfig;

// C map: this trait is the typed equivalent of `get_cfg`, `set_cfg`, and
// `get_cfg_xml` in `third_party/float-out-boy/src/main.c:2334-2396`.
impl vescpkg_rs::StatefulCustomConfigCallback<FLOAT_OUT_BOY_CONFIG_LEN>
    for FloatOutBoyCustomConfig
{
    type State = FloatOutBoyPackageState;
    type Error = ();

    // C map: `get_cfg` serializes `data_float_out_boy_config` for the default request.
    fn default_config() -> ConfigBytes<'static, FLOAT_OUT_BOY_CONFIG_LEN> {
        ConfigBytes::new(&FLOAT_OUT_BOY_DEFAULT_CONFIG)
    }

    // C map: `get_cfg` serializes active `d->float_conf` for the current request.
    fn current_config(state: &Self::State) -> ConfigBytes<'_, FLOAT_OUT_BOY_CONFIG_LEN> {
        ConfigBytes::new(state.serialized_config())
    }

    // C map: `set_cfg` in upstream validates/sanitizes, stores into `d->float_conf`,
    // and (in C) persists/reconfigures via EEPROM + `configure(d)`.
    fn set_config(
        context: &mut vescpkg_rs::StatefulCallbackContext<'_, Self::State>,
        config: ConfigBytes<'_, FLOAT_OUT_BOY_CONFIG_LEN>,
    ) -> Result<(), Self::Error> {
        // VESC 6.06+ releases motor control during flash. Apply in RAM first and
        // defer persistence until the package has remained safely stopped.
        let now = vescpkg_rs::FirmwareClock::current_timestamp();
        let Some(config) = context.with_state(|state| {
            let config = state.prepare_serialized_config(config.as_bytes())?;
            state.commit_custom_config(config, false, now);
            let config = state.begin_active_config_persistence(now);
            if config.is_none() {
                state.finish_configure_without_firmware_migration();
            }
            Some(config)
        }) else {
            return Err(());
        };
        let Some(config) = config else {
            return Ok(());
        };
        let stored =
            context.with_effects(|effects| super::state::store_persisted_config(effects, &config));
        let migration = context.with_effects(super::state::migrate_legacy_firmware_imu_settings);
        let finished_at = vescpkg_rs::FirmwareClock::current_timestamp();
        context.with_state(|state| {
            state.finish_config_persistence(&config, stored, finished_at);
            state.finish_configure_active(migration);
        });
        Ok(())
    }

    // C map: `get_cfg_xml` in upstream returns `data_float_out_boy_config_` directly.
    fn config_xml() -> ConfigXml<'static> {
        runtime_float_out_boy_config_xml()
    }
}

// Keep concrete package-local callback symbols: firmware rebases these loaded-image
// addresses before registration. C map: Float Out Boy defines `get_cfg`, `set_cfg`, and
// `get_cfg_xml` at `third_party/float-out-boy/src/main.c:2334-2396`, then registers
// those exact symbols at `third_party/float-out-boy/src/main.c:2455`.
vescpkg_rs::firmware_stateful_custom_config_callbacks!(
    float_out_boy_get_cfg,
    float_out_boy_set_cfg,
    float_out_boy_get_cfg_xml,
    FloatOutBoyCustomConfig,
    FLOAT_OUT_BOY_CONFIG_LEN
);

/// Register Float Out Boy custom config and app data through the typed package API.
///
/// C map: upstream registers custom config followed by `on_command_received`
/// at `third_party/float-out-boy/src/main.c:2455-2456`.
#[cfg(all(not(test), target_arch = "arm"))]
pub(super) fn register_float_out_boy_callbacks(
    start: &mut PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.register_callbacks::<
        FloatOutBoyCustomConfig,
        super::callbacks::FloatOutBoyAppData,
        FLOAT_OUT_BOY_CONFIG_LEN,
    >()
}

#[cfg(test)]
pub(super) fn lock_test_float_out_boy_config_state() -> impl Drop {
    super::test_support::lock_float_out_boy_runtime_state()
}

#[cfg(test)]
pub(super) fn install_test_float_out_boy_runtime_state(
    state: &mut FloatOutBoyPackageState,
) -> Option<impl Drop + '_> {
    vescpkg_rs::test_support::install_state(&crate::__VESCPKG_PACKAGE_STATE, state)
}

#[cfg(test)]
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
    let now = vescpkg_rs::FirmwareClock::current_timestamp();
    state.commit_custom_config(config, false, now);
    let Some(config) = state.begin_active_config_persistence(now) else {
        state.finish_configure_without_firmware_migration();
        return true;
    };
    let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
        super::state::store_persisted_config(effects, &config)
    });
    let migration = vescpkg_rs::test_support::with_firmware_effects(
        super::state::migrate_legacy_firmware_imu_settings,
    );
    state.finish_config_persistence(&config, stored, now);
    state.finish_configure_active(migration);
    true
}

#[cfg(test)]
pub(crate) fn set_float_out_boy_custom_config_for_test(
    state: &mut FloatOutBoyPackageState,
    config: &[u8; FLOAT_OUT_BOY_CONFIG_LEN],
) -> bool {
    float_out_boy_set_cfg_payload_with_state(ConfigBytes::new(config), state)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_float_out_boy_config_xml() -> ConfigXml<'static> {
    // C map: Float Out Boy returns generated `data_float_out_boy_config_` directly from
    // `third_party/float-out-boy/src/main.c:2388-2396`. VESC calls this function from
    // the loaded native image, so Rust's PC-relative static reference is already
    // a loaded pointer; adding the loader base would double-rebase it.
    ConfigXml::new(&FLOAT_OUT_BOY_CONFIG_XML)
}

#[cfg(test)]
fn runtime_float_out_boy_config_xml() -> ConfigXml<'static> {
    ConfigXml::new(&FLOAT_OUT_BOY_CONFIG_XML)
}

#[cfg(test)]
mod tests;
