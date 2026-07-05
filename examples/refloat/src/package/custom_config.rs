use super::RefloatPackageState;
#[cfg(all(not(test), target_arch = "arm"))]
use super::{loaded_image_base, refloat_state_from_arg};
use crate::config::{REFLOAT_CONFIG_XML, REFLOAT_DEFAULT_CONFIG};
use core::ffi::c_int;
use vescpkg_rs::{CustomConfigBindings, CustomConfigGetBuffer, CustomConfigXmlOut, ffi};

/// Register Refloat custom-config callbacks with VESC Tool.
///
/// Upstream registers `get_cfg`, `set_cfg`, and `get_cfg_xml` at
/// `third_party/refloat/src/main.c:2456`; those callbacks are implemented at `third_party/refloat/src/main.c:2334-2396`.
/// The Rust port keeps the generated serialized config image here until the
/// full typed `RefloatConfig` parser/deserializer exists.
pub fn register_refloat_custom_config<B: CustomConfigBindings>(bindings: &B) -> bool {
    bindings.register_custom_config_callbacks(refloat_get_cfg, refloat_set_cfg, refloat_get_cfg_xml)
}

extern "C" fn refloat_get_cfg(buffer: *mut u8, is_default: bool) -> c_int {
    // C map: Refloat v1.2.1 `get_cfg` starts at `third_party/refloat/src/main.c:2335`.
    let state = runtime_refloat_config_state();
    refloat_get_cfg_with_state(buffer, is_default, state)
}

fn refloat_get_cfg_with_state(
    buffer: *mut u8,
    is_default: bool,
    state: Option<&RefloatPackageState>,
) -> c_int {
    if !is_default {
        // Upstream serializes `d->float_conf` at `third_party/refloat/src/main.c:2347-2350`;
        // `data_init` first populates it from EEPROM or generated defaults at
        // `third_party/refloat/src/main.c:1160-1185`. The Rust state stores the serialized image
        // until the typed `RefloatConfig` parser/deserializer is ported.
        let Some(state) = state else {
            return 0;
        };
        return copy_refloat_config(buffer, state.serialized_config());
    }

    // Upstream default path is `third_party/refloat/src/main.c:2339-2350`: allocate config, call
    // `confparser_set_defaults_refloatconfig`, then
    // `confparser_serialize_refloatconfig`.
    copy_refloat_config(buffer, &REFLOAT_DEFAULT_CONFIG)
}

fn copy_refloat_config(buffer: *mut u8, config: &[u8; 276]) -> c_int {
    let Some(mut buffer) = CustomConfigGetBuffer::new(buffer, config.len()) else {
        return 0;
    };
    buffer.write(ffi::ConfigPayload(config))
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_state() -> Option<&'static RefloatPackageState> {
    refloat_state_from_arg().map(|state| &*state)
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_state() -> Option<&'static RefloatPackageState> {
    None
}

extern "C" fn refloat_set_cfg(buffer: *mut u8) -> bool {
    // C map: Refloat v1.2.1 `set_cfg` starts at `third_party/refloat/src/main.c:2360`.
    let state = runtime_refloat_config_state_mut();
    refloat_set_cfg_with_state(buffer, state)
}

pub(super) fn refloat_set_cfg_with_state(
    buffer: *mut u8,
    state: Option<&mut RefloatPackageState>,
) -> bool {
    let Some(state) = state else {
        return false;
    };
    let Some(config) = vescpkg_rs::custom_config_payload(buffer, REFLOAT_DEFAULT_CONFIG.len())
    else {
        return false;
    };
    // Upstream `set_cfg` gates special modes, deserializes, persists, and
    // reconfigures at `third_party/refloat/src/main.c:2360-2386`; generated
    // `conf/confparser.c:187-190` rejects bad signatures before field reads.
    // This byte-image step is intentionally only the deserialization/storage
    // part; EEPROM write and `configure(d)` remain separate parity work.
    state.store_serialized_config(config.0)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatPackageState> {
    refloat_state_from_arg()
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_state_mut() -> Option<&'static mut RefloatPackageState> {
    None
}

extern "C" fn refloat_get_cfg_xml(buffer: *mut *mut u8) -> c_int {
    // C map: Refloat v1.2.1 `get_cfg_xml` starts at `third_party/refloat/src/main.c:2389`.
    let Some(buffer) = CustomConfigXmlOut::new(buffer) else {
        return 0;
    };
    // Upstream returns `data_refloatconfig_ + PROG_ADDR` and
    // `DATA_REFLOATCONFIG__SIZE` at `third_party/refloat/src/main.c:2388-2396`.
    buffer.return_xml(runtime_refloat_config_xml())
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_refloat_config_xml() -> ffi::ConfigXmlBytes<'static> {
    let ptr = (loaded_image_base() as usize + REFLOAT_CONFIG_XML.as_ptr() as usize) as *const u8;
    vescpkg_rs::config_xml_bytes(ptr, REFLOAT_CONFIG_XML.len()).expect("refloat XML image")
}

#[cfg(any(test, not(target_arch = "arm")))]
fn runtime_refloat_config_xml() -> ffi::ConfigXmlBytes<'static> {
    ffi::ConfigXmlBytes(&REFLOAT_CONFIG_XML)
}

#[cfg(test)]
mod tests {
    use super::{
        refloat_get_cfg, refloat_get_cfg_with_state, refloat_get_cfg_xml,
        refloat_set_cfg_with_state,
    };
    use crate::config::{REFLOAT_CONFIG_DISABLED_OFFSET, REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET};
    use crate::domain::{RefloatMode, RefloatRunState};
    use crate::package::RefloatPackageState;
    use crate::package::test_support::{
        sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
    };

    #[test]
    fn custom_config_xml_callback_returns_upstream_settings_blob() {
        let mut buffer = core::ptr::null_mut();

        let len = refloat_get_cfg_xml(&mut buffer);

        // Refloat v1.2.1 returns generated `data_refloatconfig_` at
        // `third_party/refloat/src/main.c:2388-2396`, produced from `third_party/refloat/src/conf/settings.xml` by
        // `third_party/refloat/src/Makefile:28-31`.
        assert_eq!(len, 25_723);
        assert!(!buffer.is_null());
        let bytes = vescpkg_rs::config_xml_bytes(buffer.cast_const(), len as usize)
            .expect("returned XML bytes");
        assert_eq!(&bytes.0[..6], &[0x00, 0x05, 0x5c, 0xa1, 0x78, 0xda]);
    }

    #[test]
    fn custom_config_default_callback_returns_upstream_serialized_defaults() {
        let mut buffer = [0u8; 276];

        let len = refloat_get_cfg(buffer.as_mut_ptr(), true);

        // Refloat v1.2.1 default `get_cfg` allocates a temporary config,
        // applies generated defaults, and serializes it at `third_party/refloat/src/main.c:2339-2350`.
        // The generated format comes from `third_party/refloat/src/Makefile:28-31`;
        // generated `conf/confparser.h:11-12` fixes signature/length, and
        // generated `conf/confparser.c:8-178,363-531` writes these bytes.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("../conf/default_config.dat"));
        assert_eq!(&buffer[..4], &[0x90, 0xb7, 0xa9, 0xba]);
    }

    #[test]
    fn custom_config_current_callback_reads_state_serialized_config() {
        let state = RefloatPackageState::new(sample_all_data_payloads());
        let mut buffer = [0u8; 276];

        let len = refloat_get_cfg_with_state(buffer.as_mut_ptr(), false, Some(&state));

        // Upstream current `get_cfg` serializes `d->float_conf` from shared
        // package state at `third_party/refloat/src/main.c:2347-2350`; `data_init` populates it
        // from EEPROM or generated defaults at `third_party/refloat/src/main.c:1160-1185`.
        assert_eq!(len, 276);
        assert_eq!(buffer, *include_bytes!("../conf/default_config.dat"));
    }

    #[test]
    fn custom_config_set_callback_stores_serialized_config_in_state() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("../conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream `set_cfg` deserializes into `d->float_conf` at
        // `third_party/refloat/src/main.c:2368`; generated `conf/confparser.c:187-190` rejects a
        // bad signature before reading the field bytes.
        incoming[REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 0;
        assert_eq!(len, 276);
        assert_eq!(current, incoming);
    }

    #[test]
    fn custom_config_set_callback_resets_is_default_flag_like_refloat() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("../conf/default_config.dat");
        incoming[REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET] = 1;

        assert!(refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream clears `d->float_conf.meta.is_default` for every config
        // write at `third_party/refloat/src/main.c:2375-2377`; generated
        // `conf/confparser.c:179` serializes that flag as the final byte.
        assert_eq!(len, 276);
        assert_eq!(current[REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_keeps_package_enabled_while_running_like_refloat() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut incoming = *include_bytes!("../conf/default_config.dat");
        incoming[REFLOAT_CONFIG_DISABLED_OFFSET] = 1;

        assert!(refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream refuses to persist `disabled = true` while running at
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is serialized at
        // `third_party/refloat/src/conf/settings.xml:4064`.
        assert_eq!(len, 276);
        assert_eq!(current[REFLOAT_CONFIG_DISABLED_OFFSET], 0);
    }

    #[test]
    fn custom_config_set_callback_rejects_special_modes_like_refloat() {
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::HandTest,
        ));
        let mut incoming = *include_bytes!("../conf/default_config.dat");
        incoming[4] = 0x12;

        assert!(!refloat_set_cfg_with_state(
            incoming.as_mut_ptr(),
            Some(&mut state),
        ));

        let mut current = [0u8; 276];
        let len = refloat_get_cfg_with_state(current.as_mut_ptr(), false, Some(&state));

        // Upstream rejects VESC Tool config writes outside `MODE_NORMAL` at
        // `third_party/refloat/src/main.c:2362-2365`, before storing to EEPROM or reconfiguring.
        assert_eq!(len, 276);
        assert_eq!(current, *include_bytes!("../conf/default_config.dat"));
    }
}
