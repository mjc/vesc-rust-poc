use super::{RefloatBeeperAlert, RefloatBeeperCount, RefloatPackageState};
use crate::config::{REFLOAT_CONFIG_LEN, RefloatConfigImage};
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, RefloatAppDataCommand, RefloatMode, RefloatRunState,
};

impl RefloatPackageState {
    fn write_config_to_eeprom(&self) -> bool {
        let eeprom = vescpkg_rs::CustomEeprom::new();
        self.serialized_config
            .as_bytes()
            .chunks_exact(4)
            .enumerate()
            .all(|(index, bytes)| {
                let Some(address) = vescpkg_rs::CustomEepromAddress::from_index(index) else {
                    return false;
                };
                let Some(bytes) = <&[u8; 4]>::try_from(bytes).ok() else {
                    return false;
                };
                eeprom.write(address, vescpkg_rs::EepromWord::from_ne_bytes(*bytes))
            })
    }

    fn read_config_from_eeprom(&mut self) {
        let eeprom = vescpkg_rs::CustomEeprom::new();
        let mut bytes = [0_u8; REFLOAT_CONFIG_LEN];
        let read = bytes.chunks_exact_mut(4).enumerate().all(|(index, bytes)| {
            let Some(address) = vescpkg_rs::CustomEepromAddress::from_index(index) else {
                return false;
            };
            let Some(word) = eeprom.read(address) else {
                return false;
            };
            bytes.copy_from_slice(&word.to_ne_bytes());
            true
        });
        self.serialized_config = read
            .then(|| RefloatConfigImage::from_serialized(&bytes))
            .flatten()
            .unwrap_or_else(RefloatConfigImage::defaults);
    }

    pub(super) fn load_persisted_config_on_startup(&mut self) {
        self.read_config_from_eeprom();
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
    }

    pub(super) fn handle_config_command(&mut self, bytes: &[u8]) -> bool {
        let [package_id, command, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != REFLOAT_APP_DATA_PACKAGE_ID.get() {
            return false;
        }
        let Ok(command) = RefloatAppDataCommand::try_from_id(*command) else {
            return false;
        };

        match command {
            RefloatAppDataCommand::ConfigSave => {
                if self.write_config_to_eeprom() {
                    self.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::ONE));
                }
            }
            RefloatAppDataCommand::ConfigRestore => self.read_config_from_eeprom(),
            RefloatAppDataCommand::TuneDefaults => {
                self.serialized_config.reset_tune_defaults();
                self.refresh_balance_filter_config();
            }
            RefloatAppDataCommand::Lock => {
                let Some(disabled) = payload.first() else {
                    return false;
                };
                let run_state = self
                    .all_data_payloads
                    .base()
                    .status()
                    .ride_state()
                    .run_state();
                if !matches!(run_state, RefloatRunState::Running) {
                    self.read_config_from_eeprom();
                    self.serialized_config.editor().set_disabled(*disabled != 0);
                    self.refresh_balance_filter_config();
                    self.refresh_config_runtime_state();
                    if self.write_config_to_eeprom() {
                        self.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::ONE));
                    }
                }
            }
            _ => return false,
        }
        true
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn bms_enabled(&self) -> bool {
        self.serialized_config.bms().enabled()
    }

    pub(in crate::package) fn serialized_config(&self) -> &[u8; 276] {
        // C map: `get_cfg(..., is_default=false)` serializes the current
        // `d->float_conf` image at `third_party/refloat/src/main.c:2335-2356`.
        self.serialized_config.as_bytes()
    }

    #[cfg(test)]
    pub(in crate::package) fn replace_serialized_config_for_test(
        &mut self,
        config: crate::config::RefloatConfigImage,
    ) {
        self.serialized_config = config;
        self.write_config_to_eeprom();
    }

    #[cfg(test)]
    pub(in crate::package) fn balance_config_for_test(
        &self,
    ) -> crate::config::RefloatBalanceConfig<'_> {
        self.serialized_config.balance()
    }

    pub(in crate::package) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Some(mut config) = RefloatConfigImage::from_serialized(config) else {
            return false;
        };

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `third_party/refloat/src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        // Upstream clears `d->float_conf.disabled` while running at
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is
        // serialized from `third_party/refloat/src/conf/settings.xml:3890-3902`
        // at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            config.editor().keep_enabled_while_running();
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `third_party/refloat/src/main.c:2375-2377`; `meta.is_default`
        // is serialized from `third_party/refloat/src/conf/settings.xml:3903-3914`
        // at byte 275.
        config.editor().clear_meta_is_default();
        self.serialized_config = config;
        // After a successful write, C calls `configure(d)` at
        // `third_party/refloat/src/main.c:2380-2382`, which refreshes the balance filter KP at
        // `third_party/refloat/src/main.c:158-160`.
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
        // `configure(d)` applies the new beeper setting, then acknowledges
        // disabled state with three short beeps and every other state with one
        // at `third_party/refloat/src/main.c:219-227`.
        let run_state = self
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
        self.alert_beeper(if matches!(run_state, RefloatRunState::Disabled) {
            RefloatBeeperAlert::Short(RefloatBeeperCount::THREE)
        } else {
            RefloatBeeperAlert::Short(RefloatBeeperCount::ONE)
        });
        true
    }

    fn refresh_balance_filter_config(&mut self) {
        // C map: `reconfigure(d)` refreshes Mahony filter gains through
        // `balance_filter_configure` at `third_party/refloat/src/main.c:154-160`.
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/refloat/src/main.c:190-191`, then `refloat_thd`
        // sleeps that value at `third_party/refloat/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        self.serialized_config.startup().loop_time_us()
    }
}
