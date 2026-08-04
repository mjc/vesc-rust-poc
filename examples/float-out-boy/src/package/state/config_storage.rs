use super::{FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount, FloatOutBoyPackageState};
use crate::config::{FLOAT_OUT_BOY_CONFIG_LEN, FloatOutBoyConfigImage};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand, FloatOutBoyMode,
    FloatOutBoyRunState,
};
use vescpkg_rs::{FirmwareEffects, TimestampTicks};

pub(super) const FLOAT_OUT_BOY_EEPROM_LEN: usize = 320;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) enum FloatOutBoyConfigLoadOutcome {
    #[default]
    NotAttempted,
    Persisted,
    DefaultAfterReadFailure,
    DefaultAfterInvalidImage,
}

// Startup fallback logging is reachable from `package_lib_init`; an extra call
// frame would exceed the firmware's 2 KiB Lisp evaluator stack.
#[expect(
    clippy::inline_always,
    reason = "the package builder directly proves this firmware stack constraint"
)]
#[inline(always)]
fn log_config_message(effects: &FirmwareEffects, message: &[u8]) {
    let mut log = vescpkg_rs::FirmwareLog::<48>::new();
    log.write_bytes(message);
    let _ = log.flush(effects);
}

fn log_config_load_fallback(effects: &FirmwareEffects, outcome: FloatOutBoyConfigLoadOutcome) {
    let message = match outcome {
        FloatOutBoyConfigLoadOutcome::DefaultAfterReadFailure => b"read fail".as_slice(),
        FloatOutBoyConfigLoadOutcome::DefaultAfterInvalidImage => b"invalid".as_slice(),
        FloatOutBoyConfigLoadOutcome::NotAttempted | FloatOutBoyConfigLoadOutcome::Persisted => {
            return;
        }
    };
    log_config_message(effects, message);
}

fn log_config_store_result(effects: &FirmwareEffects, stored: bool) {
    let message = if stored {
        b"saved".as_slice()
    } else {
        b"save fail".as_slice()
    };
    log_config_message(effects, message);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) enum FirmwareImuMigration {
    #[default]
    Pending,
    NotRequired,
    Applied,
    InvalidRead,
    InvalidTarget,
    // Pinned VESC accepts all three supported parameter IDs unconditionally.
    // Retain the outcome for defensive ABI diagnostics and test fakes.
    UnexpectedRejection {
        proportional_gain: bool,
        integral_gain: bool,
        acceleration_confidence_decay: bool,
    },
}

pub(in crate::package) fn migrate_legacy_firmware_imu_settings(
    effects: &FirmwareEffects,
) -> FirmwareImuMigration {
    let settings = vescpkg_rs::FirmwareSettings;
    let Ok(gain) = settings.imu_mahony_proportional_gain() else {
        return FirmwareImuMigration::InvalidRead;
    };
    if gain.value() <= 1.0 {
        return FirmwareImuMigration::NotRequired;
    }
    let Some(proportional_gain) = vescpkg_rs::ImuMahonyProportionalGain::try_new(0.4) else {
        return FirmwareImuMigration::InvalidTarget;
    };
    let Some(integral_gain) = vescpkg_rs::ImuMahonyIntegralGain::try_new(0.0) else {
        return FirmwareImuMigration::InvalidTarget;
    };
    let proportional_gain = settings
        .set_imu_mahony_proportional_gain(effects, proportional_gain)
        .is_err();
    let integral_gain = settings
        .set_imu_mahony_integral_gain(effects, integral_gain)
        .is_err();
    let acceleration_confidence_decay = settings
        .set_imu_acceleration_confidence_decay(effects, vescpkg_rs::Ratio::from_ratio_const(0.1))
        .is_err();

    if proportional_gain || integral_gain || acceleration_confidence_decay {
        FirmwareImuMigration::UnexpectedRejection {
            proportional_gain,
            integral_gain,
            acceleration_confidence_decay,
        }
    } else {
        FirmwareImuMigration::Applied
    }
}

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

    fn load(effects: &FirmwareEffects) -> Result<Self, vescpkg_rs::EepromError> {
        vescpkg_rs::CustomEeprom::new()
            .read_image::<FLOAT_OUT_BOY_EEPROM_LEN>(effects)
            .map(|bytes| Self::from_bytes(&bytes))
    }

    fn store(self, effects: &FirmwareEffects) -> Result<(), vescpkg_rs::EepromError> {
        let eeprom = vescpkg_rs::CustomEeprom::new();
        let signature_offset = vescpkg_rs::EepromWordOffset::from_index(0);
        let payload_offset = vescpkg_rs::EepromWordOffset::from_index(1);

        eeprom.write_at(
            effects,
            signature_offset,
            vescpkg_rs::EepromWord::from_u32(0),
        )?;
        eeprom.write_bytes_at_offset(effects, payload_offset, self.payload_bytes())?;
        eeprom.write_at(effects, signature_offset, self.signature_word())
    }

    const fn signature_word(self) -> vescpkg_rs::EepromWord {
        let [first, second, third, fourth, ..] = self.0;
        vescpkg_rs::EepromWord::from_ne_bytes([first, second, third, fourth])
    }

    fn payload_bytes(&self) -> &[u8] {
        &self.0[vescpkg_rs::EepromWord::BYTE_LEN..]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) struct FloatOutBoyPersistedConfig {
    config: FloatOutBoyConfigImage,
    outcome: FloatOutBoyConfigLoadOutcome,
}

pub(in crate::package) fn load_persisted_config(
    effects: &FirmwareEffects,
) -> FloatOutBoyPersistedConfig {
    let loaded = match FloatOutBoyEepromImage::load(effects) {
        Ok(image) => match FloatOutBoyConfigImage::try_from(image) {
            Ok(config) => FloatOutBoyPersistedConfig {
                config,
                outcome: FloatOutBoyConfigLoadOutcome::Persisted,
            },
            Err(_) => FloatOutBoyPersistedConfig {
                config: FloatOutBoyConfigImage::defaults(),
                outcome: FloatOutBoyConfigLoadOutcome::DefaultAfterInvalidImage,
            },
        },
        Err(_) => FloatOutBoyPersistedConfig {
            config: FloatOutBoyConfigImage::defaults(),
            outcome: FloatOutBoyConfigLoadOutcome::DefaultAfterReadFailure,
        },
    };
    log_config_load_fallback(effects, loaded.outcome);
    loaded
}

pub(in crate::package) fn store_persisted_config(
    effects: &FirmwareEffects,
    config: &FloatOutBoyConfigImage,
) -> bool {
    let stored = FloatOutBoyEepromImage::from(*config).store(effects).is_ok();
    log_config_store_result(effects, stored);
    stored
}

impl FloatOutBoyPackageState {
    pub(in crate::package) fn acknowledge_command_config_write(&mut self, now: TimestampTicks) {
        self.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
        self.start_internal_led_confirmation(now);
    }

    pub(in crate::package) const fn active_config_image(&self) -> FloatOutBoyConfigImage {
        self.serialized_config
    }

    pub(in crate::package) fn is_running(&self) -> bool {
        matches!(
            self.all_data_payloads
                .base()
                .status()
                .ride_state()
                .run_state(),
            FloatOutBoyRunState::Running
        )
    }

    pub(super) fn replace_active_config(&mut self, config: &FloatOutBoyConfigImage) {
        self.serialized_config = *config;
        self.reconfigure_active_config();
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn reconfigure_active_config(&mut self) {
        self.refresh_balance_filter_config();
        self.refresh_led_config_runtime_state();
        self.refresh_config_runtime_state();
    }

    pub(in crate::package) fn apply_persisted_config(
        &mut self,
        loaded: &FloatOutBoyPersistedConfig,
    ) {
        self.serialized_config = loaded.config;
        self.config_load_outcome = loaded.outcome;
    }

    pub(in crate::package) fn begin_configure_active(&mut self, now: TimestampTicks) {
        self.reconfigure_active_config();
        self.refresh_idle_epoch(now);
    }

    pub(in crate::package) fn begin_restore_persisted_config(
        &mut self,
        loaded: &FloatOutBoyPersistedConfig,
        now: TimestampTicks,
    ) {
        self.apply_persisted_config(loaded);
        self.begin_configure_active(now);
    }

    pub(in crate::package) fn finish_configure_active(&mut self, migration: FirmwareImuMigration) {
        self.firmware_imu_migration = migration;
        self.alert_after_configure();
    }

    pub(in crate::package) fn prepare_serialized_config(
        &self,
        config: &[u8],
    ) -> Option<FloatOutBoyConfigImage> {
        let mut config = FloatOutBoyConfigImage::from_serialized(config)?;
        let ride_state = self.all_data_payloads.base().status().ride_state();
        if !matches!(ride_state.mode(), FloatOutBoyMode::Normal) {
            return None;
        }
        if matches!(ride_state.run_state(), FloatOutBoyRunState::Running) {
            config.editor().keep_enabled_while_running();
        }
        config.editor().clear_meta_is_default();
        Some(config)
    }

    pub(in crate::package) fn commit_custom_config(
        &mut self,
        mut config: FloatOutBoyConfigImage,
        stored: bool,
        now: TimestampTicks,
    ) {
        if self.is_running() {
            config.editor().keep_enabled_while_running();
        }
        if stored {
            self.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
        }
        self.serialized_config = config;
        self.begin_configure_active(now);
        if stored {
            self.start_internal_led_confirmation(now);
        }
    }

    pub(in crate::package) fn apply_lock_from_persisted(
        &mut self,
        loaded: &FloatOutBoyPersistedConfig,
        disabled: bool,
        now: TimestampTicks,
    ) -> Option<FloatOutBoyConfigImage> {
        if self.is_running() {
            return None;
        }
        self.apply_persisted_config(loaded);
        self.serialized_config.editor().set_disabled(disabled);
        self.begin_configure_active(now);
        Some(self.serialized_config)
    }

    #[cfg(target_arch = "arm")]
    pub(in crate::package) fn begin_startup_configure(
        &mut self,
        loaded: &FloatOutBoyPersistedConfig,
        now: TimestampTicks,
    ) {
        self.apply_persisted_config(loaded);
        self.initialize_time_epochs(now);
        self.refresh_balance_filter_config();
        super::config_runtime::refresh_led_effects(self);
        self.refresh_config_runtime_state();
    }

    pub(in crate::package) fn finish_startup_configure(&mut self, migration: FirmwareImuMigration) {
        self.firmware_imu_migration = migration;
        self.alert_after_configure();
        self.startup_configured = true;
    }

    pub(in crate::package) const fn startup_configured(&self) -> bool {
        self.startup_configured
    }

    pub(in crate::package) fn setup_loaded_led_hardware_after_threads(
        &mut self,
        adc1: vescpkg_rs::AdcVoltage,
        adc2: vescpkg_rs::AdcVoltage,
    ) {
        self.refresh_footpad_runtime_state(adc1, adc2);
        self.refresh_led_config_runtime_state();
        self.apply_pending_internal_led_refresh();
    }

    pub(super) fn alert_after_configure(&mut self) {
        let run_state = self
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
        let alert = match run_state {
            FloatOutBoyRunState::Disabled => {
                FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE)
            }
            // Intentional Refloat 1.2.1 bug fix from upstream 37cf343:
            // leave the beeper free for the READY battery-status alert.
            FloatOutBoyRunState::Startup => return,
            FloatOutBoyRunState::Ready | FloatOutBoyRunState::Running => {
                FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE)
            }
        };
        self.alert_beeper(alert);
    }

    pub(super) fn handle_config_command(
        &mut self,
        bytes: &[u8],
        now: &mut impl FnMut() -> TimestampTicks,
    ) -> bool {
        let [package_id, command, ..] = bytes else {
            return false;
        };
        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
            return false;
        }
        let Ok(command) = FloatOutBoyAppDataCommand::try_from(*command) else {
            return false;
        };

        match command {
            FloatOutBoyAppDataCommand::TuneDefaults => {
                let mut config = self.serialized_config;
                config.reset_tune_defaults();
                self.replace_active_config(&config);
                self.refresh_idle_epoch(now());
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn bms_enabled(&self) -> bool {
        self.serialized_config.bms().enabled()
    }

    pub(in crate::package) fn serialized_config(&self) -> &[u8; 276] {
        // C map: `get_cfg(..., is_default=false)` serializes the current
        // `d->float_conf` image at `third_party/float-out-boy/src/main.c:2335-2356`.
        self.serialized_config.as_bytes()
    }

    pub(super) fn refresh_balance_filter_config(&mut self) {
        // C map: `reconfigure(d)` refreshes Mahony filter gains through
        // `balance_filter_configure` at `third_party/float-out-boy/src/main.c:154-160`.
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/float-out-boy/src/main.c:190-191`, then `float_out_boy_thd`
        // sleeps that value at `third_party/float-out-boy/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        self.serialized_config.startup().loop_time_us()
    }
}
