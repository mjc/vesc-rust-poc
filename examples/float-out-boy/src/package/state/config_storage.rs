use super::{FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount, FloatOutBoyPackageState};
use crate::config::{FLOAT_OUT_BOY_CONFIG_LEN, FloatOutBoyConfigImage};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
    FloatOutBoyRunState,
};

pub(super) const FLOAT_OUT_BOY_EEPROM_LEN: usize = 320;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FloatOutBoyEepromImage([u8; FLOAT_OUT_BOY_EEPROM_LEN]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FloatOutBoyEepromImageError;

impl FloatOutBoyEepromImage {
    pub(super) const fn from_bytes(bytes: &[u8; FLOAT_OUT_BOY_EEPROM_LEN]) -> Self {
        Self(*bytes)
    }

    pub(super) const fn as_bytes(&self) -> &[u8; FLOAT_OUT_BOY_EEPROM_LEN] {
        &self.0
    }

    pub(super) const fn into_bytes(self) -> [u8; FLOAT_OUT_BOY_EEPROM_LEN] {
        self.0
    }
}

impl From<FloatOutBoyConfigImage> for FloatOutBoyEepromImage {
    fn from(config: FloatOutBoyConfigImage) -> Self {
        let mut bytes = [0; FLOAT_OUT_BOY_EEPROM_LEN];
        bytes[..FLOAT_OUT_BOY_CONFIG_LEN].copy_from_slice(config.as_bytes());
        Self(bytes)
    }
}

impl core::convert::TryFrom<FloatOutBoyEepromImage> for FloatOutBoyConfigImage {
    type Error = FloatOutBoyEepromImageError;

    fn try_from(image: FloatOutBoyEepromImage) -> Result<Self, Self::Error> {
        Self::from_serialized(&image.as_bytes()[..FLOAT_OUT_BOY_CONFIG_LEN])
            .ok_or(FloatOutBoyEepromImageError)
    }
}

impl FloatOutBoyPackageState {
    fn write_config_to_eeprom(&self) -> bool {
        let image = FloatOutBoyEepromImage::from(self.serialized_config);
        let bytes = image.into_bytes();
        vescpkg_rs::CustomEeprom::new().write_image(&bytes).is_ok()
    }

    pub(super) fn read_config_from_eeprom(&mut self) {
        self.serialized_config = vescpkg_rs::CustomEeprom::new()
            .read_image::<FLOAT_OUT_BOY_EEPROM_LEN>()
            .map(|bytes| FloatOutBoyEepromImage::from_bytes(&bytes))
            .ok()
            .and_then(|image| FloatOutBoyConfigImage::try_from(image).ok())
            .unwrap_or_else(FloatOutBoyConfigImage::defaults);
    }

    pub(in crate::package) fn load_persisted_config_on_startup(&mut self) {
        self.read_config_from_eeprom();
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
    }

    pub(super) fn handle_config_command(&mut self, bytes: &[u8]) -> bool {
        let [package_id, command, payload @ ..] = bytes else {
            return false;
        };
        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
            return false;
        }
        let Ok(command) = FloatOutBoyAppDataCommand::try_from_id(*command) else {
            return false;
        };

        match command {
            FloatOutBoyAppDataCommand::ConfigSave => {
                if self.write_config_to_eeprom() {
                    self.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
                }
            }
            FloatOutBoyAppDataCommand::ConfigRestore => self.load_persisted_config_on_startup(),
            FloatOutBoyAppDataCommand::TuneDefaults => {
                self.serialized_config.reset_tune_defaults();
                self.refresh_balance_filter_config();
            }
            FloatOutBoyAppDataCommand::Lock => {
                let Some(disabled) = payload.first() else {
                    return false;
                };
                let run_state = self
                    .all_data_payloads
                    .base()
                    .status()
                    .ride_state()
                    .run_state();
                if !matches!(run_state, FloatOutBoyRunState::Running) {
                    self.read_config_from_eeprom();
                    self.serialized_config.editor().set_disabled(*disabled != 0);
                    self.refresh_balance_filter_config();
                    self.refresh_config_runtime_state();
                    if self.write_config_to_eeprom() {
                        self.alert_beeper(FloatOutBoyBeeperAlert::Short(
                            FloatOutBoyBeeperCount::ONE,
                        ));
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
        // `d->float_conf` image at `third_party/float-out-boy/src/main.c:2335-2356`.
        self.serialized_config.as_bytes()
    }

    #[cfg(test)]
    pub(in crate::package) fn replace_serialized_config_for_test(
        &mut self,
        config: &crate::config::FloatOutBoyConfigImage,
    ) {
        self.serialized_config = *config;
    }

    #[cfg(test)]
    pub(in crate::package) fn balance_config_for_test(
        &self,
    ) -> crate::config::FloatOutBoyBalanceConfig<'_> {
        self.serialized_config.balance()
    }

    pub(in crate::package) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Some(mut config) = FloatOutBoyConfigImage::from_serialized(config) else {
            return false;
        };

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `third_party/float-out-boy/src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), FloatOutBoyMode::Normal) {
            return false;
        }

        // Upstream clears `d->float_conf.disabled` while running at
        // `third_party/float-out-boy/src/main.c:2369-2372`; `disabled` is
        // serialized from `third_party/float-out-boy/src/conf/settings.xml:3890-3902`
        // at byte 243.
        if matches!(ride_state.run_state(), FloatOutBoyRunState::Running) {
            config.editor().keep_enabled_while_running();
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `third_party/float-out-boy/src/main.c:2375-2377`; `meta.is_default`
        // is serialized from `third_party/float-out-boy/src/conf/settings.xml:3903-3914`
        // at byte 275.
        config.editor().clear_meta_is_default();
        let previous_config = self.serialized_config;
        self.serialized_config = config;
        if !self.write_config_to_eeprom() {
            self.serialized_config = previous_config;
            return false;
        }
        // After a successful write, C calls `configure(d)` at
        // `third_party/float-out-boy/src/main.c:2380-2382`, which refreshes the balance filter KP at
        // `third_party/float-out-boy/src/main.c:158-160`.
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
        // `configure(d)` applies the new beeper setting, then acknowledges
        // disabled state with three short beeps and every other state with one
        // at `third_party/float-out-boy/src/main.c:219-227`.
        let run_state = self
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
        self.alert_beeper(if matches!(run_state, FloatOutBoyRunState::Disabled) {
            FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE)
        } else {
            FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE)
        });
        true
    }

    pub(super) fn refresh_balance_filter_config(&mut self) {
        // C map: `reconfigure(d)` refreshes Mahony filter gains through
        // `balance_filter_configure` at `third_party/float-out-boy/src/main.c:154-160`.
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/float-out-boy/src/main.c:190-191`, then `float_out_boy_thd`
        // sleeps that value at `third_party/float-out-boy/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        self.serialized_config.startup().loop_time_us()
    }
}
