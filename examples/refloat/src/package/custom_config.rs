use super::RefloatPackageState;
use crate::config::{REFLOAT_CONFIG_LEN, REFLOAT_CONFIG_XML, REFLOAT_DEFAULT_CONFIG};
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::PackageStart;
use vescpkg_rs::{ConfigBytes, ConfigXml};

/// Refloat's typed custom-config callback set.
///
/// C map: `get_cfg`, `set_cfg`, and `get_cfg_xml` are implemented at
/// `third_party/refloat/src/main.c:2334-2396` and registered at `:2455`.
pub struct RefloatCustomConfig;

// C map: this trait is the typed equivalent of `get_cfg`, `set_cfg`, and
// `get_cfg_xml` in `third_party/refloat/src/main.c:2334-2396`.
impl vescpkg_rs::StatefulCustomConfigCallback<REFLOAT_CONFIG_LEN> for RefloatCustomConfig {
    type State = RefloatPackageState;

    // C map: `get_cfg` serializes `data_refloatconfig` for the default request.
    fn default_config() -> ConfigBytes<'static> {
        ConfigBytes::new(&REFLOAT_DEFAULT_CONFIG)
    }

    // C map: `get_cfg` serializes active `d->float_conf` for the current request.
    fn current_config<'state>(state: &'state Self::State) -> Option<ConfigBytes<'state>> {
        Some(ConfigBytes::new(state.serialized_config()))
    }

    // C map: `set_cfg` in upstream validates/sanitizes, stores into `d->float_conf`,
    // and (in C) persists/reconfigures via EEPROM + `configure(d)`.
    fn set_config(state: &mut Self::State, config: ConfigBytes<'_>) -> bool {
        refloat_set_cfg_payload_with_state(config, state)
    }

    // C map: `get_cfg_xml` in upstream returns `data_refloatconfig_` directly.
    fn config_xml() -> ConfigXml<'static> {
        runtime_refloat_config_xml()
    }
}

// Keep concrete package-local callback symbols: firmware rebases these loaded-image
// addresses before registration. C map: Refloat defines `get_cfg`, `set_cfg`, and
// `get_cfg_xml` at `third_party/refloat/src/main.c:2334-2396`, then registers
// those exact symbols at `third_party/refloat/src/main.c:2455`.
vescpkg_rs::firmware_stateful_custom_config_callbacks!(
    refloat_get_cfg,
    refloat_set_cfg,
    refloat_get_cfg_xml,
    RefloatCustomConfig,
    REFLOAT_CONFIG_LEN
);

/// Register Refloat custom config and app data through the typed package API.
///
/// C map: upstream registers custom config followed by `on_command_received`
/// at `third_party/refloat/src/main.c:2455-2456`.
#[cfg(all(not(test), target_arch = "arm"))]
pub(super) fn register_refloat_callbacks(
    start: &mut PackageStart,
) -> Result<(), vescpkg_rs::AppDataHandlerRegistrationError> {
    start.register_callbacks::<
        RefloatCustomConfig,
        super::callbacks::RefloatAppData,
        REFLOAT_CONFIG_LEN,
    >()
}

#[cfg(test)]
pub(super) fn lock_test_refloat_config_state() -> impl Drop {
    super::test_support::lock_refloat_runtime_state()
}

#[cfg(test)]
pub(super) fn install_test_refloat_runtime_state<'a>(
    state: &'a mut RefloatPackageState,
) -> impl Drop + 'a {
    vescpkg_rs::test_support::install_state(&super::REFLOAT_RUNTIME_STATE, state)
}

fn refloat_set_cfg_payload_with_state(
    config: ConfigBytes<'_>,
    state: &mut RefloatPackageState,
) -> bool {
    // Upstream `set_cfg` gates special modes, deserializes, persists, and
    // reconfigures at `third_party/refloat/src/main.c:2360-2386`; generated
    // `conf/confparser.c:187-190` rejects bad signatures before field reads.
    // This byte-image step is intentionally only the deserialization/storage
    // part; EEPROM write and `configure(d)` remain separate parity work.
    state.store_serialized_config(config.as_bytes())
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_xml() -> ConfigXml<'static> {
    // C map: Refloat returns generated `data_refloatconfig_` directly from
    // `third_party/refloat/src/main.c:2388-2396`. VESC calls this function from
    // the loaded native image, so Rust's PC-relative static reference is already
    // a loaded pointer; adding the loader base would double-rebase it.
    ConfigXml::new(&REFLOAT_CONFIG_XML)
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_xml() -> ConfigXml<'static> {
    ConfigXml::new(&REFLOAT_CONFIG_XML)
}

#[cfg(test)]
mod tests;
