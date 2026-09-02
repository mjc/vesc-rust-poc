use super::{FloatOutBoyBeeperAlert, FloatOutBoyPackageState};
use crate::config::FloatOutBoyMetadataConfig as Metadata;
use crate::config::{FLOAT_OUT_BOY_CONFIG_LEN, FloatOutBoyConfigImage};
use crate::domain::{FloatOutBoyAppDataCommand, FloatOutBoyMode, FloatOutBoyRunState};
use vescpkg_rs::{FirmwareEffects, TimestampTicks};

#[cfg(test)]
std::thread_local! {
    static CONFIG_RECONFIGURE_COUNT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

pub(super) const FLOAT_OUT_BOY_EEPROM_LEN: usize = 320;
const DEFERRED_CONFIG_PERSISTENCE_DELAY_SECONDS: u32 = 1;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeferredConfigPersistence {
    #[default]
    Clean,
    Pending(FloatOutBoyConfigImage),
    Writing(Option<FloatOutBoyConfigImage>),
    Failed(FloatOutBoyConfigImage),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigEepromReadState {
    #[default]
    Idle,
    Reading,
}

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
    let Some(proportional_gain) = vescpkg_rs::ImuMahonyProportionalGain::try_new(0.2) else {
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

pub(super) fn float_out_boy_eeprom_image(
    config: &FloatOutBoyConfigImage,
) -> [u8; FLOAT_OUT_BOY_EEPROM_LEN] {
    let mut bytes = [0; FLOAT_OUT_BOY_EEPROM_LEN];
    bytes[..FLOAT_OUT_BOY_CONFIG_LEN].copy_from_slice(config.as_bytes());
    bytes
}

pub(super) fn float_out_boy_config_from_eeprom(
    image: &[u8; FLOAT_OUT_BOY_EEPROM_LEN],
) -> Option<FloatOutBoyConfigImage> {
    FloatOutBoyConfigImage::from_serialized(&image[..FLOAT_OUT_BOY_CONFIG_LEN])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::package) struct FloatOutBoyPersistedConfig {
    config: FloatOutBoyConfigImage,
    outcome: FloatOutBoyConfigLoadOutcome,
}

pub(in crate::package) fn load_persisted_config(
    effects: &FirmwareEffects,
) -> FloatOutBoyPersistedConfig {
    let loaded =
        match vescpkg_rs::CustomEeprom::new().read_image::<FLOAT_OUT_BOY_EEPROM_LEN>(effects) {
            Ok(image) => match float_out_boy_config_from_eeprom(&image) {
                Some(config) => FloatOutBoyPersistedConfig {
                    config,
                    outcome: FloatOutBoyConfigLoadOutcome::Persisted,
                },
                None => FloatOutBoyPersistedConfig {
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
    let image = float_out_boy_eeprom_image(config);
    let stored = vescpkg_rs::CustomEeprom::new()
        .write_signature_committed_image(effects, &image)
        .is_ok();
    log_config_store_result(effects, stored);
    stored
}

impl FloatOutBoyPackageState {
    pub(in crate::package) fn acknowledge_command_config_write(&mut self, now: TimestampTicks) {
        self.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
        self.start_internal_led_confirmation(now);
        #[cfg(not(any(test, target_arch = "arm")))]
        let _ = now;
    }

    pub(in crate::package) const fn active_config_image(&self) -> FloatOutBoyConfigImage {
        self.serialized_config
    }

    pub(in crate::package) fn is_running(&self) -> bool {
        self.all_data_payloads.ride_state().run_state() == FloatOutBoyRunState::Running
    }

    fn config_can_persist_now(&self, now: TimestampTicks) -> bool {
        match self.all_data_payloads.ride_state().run_state() {
            FloatOutBoyRunState::Running => false,
            FloatOutBoyRunState::Ready => self
                .disengage_ticks
                .older_than_secs(now, DEFERRED_CONFIG_PERSISTENCE_DELAY_SECONDS),
            FloatOutBoyRunState::Disabled | FloatOutBoyRunState::Startup => true,
        }
    }

    fn queue_config_persistence(&mut self, config: &FloatOutBoyConfigImage) {
        // Keep one queued snapshot; each later AppUI apply replaces it.
        self.deferred_config_persistence = match self.deferred_config_persistence {
            DeferredConfigPersistence::Writing(_) => {
                DeferredConfigPersistence::Writing(Some(*config))
            }
            _ => DeferredConfigPersistence::Pending(*config),
        };
    }

    pub(in crate::package) fn begin_active_config_persistence(
        &mut self,
        now: TimestampTicks,
    ) -> Option<FloatOutBoyConfigImage> {
        let config = self.active_config_image();
        if self.config_can_persist_now(now)
            && matches!(self.config_eeprom_read_state, ConfigEepromReadState::Idle)
            && !matches!(
                self.deferred_config_persistence,
                DeferredConfigPersistence::Writing(_)
            )
        {
            self.deferred_config_persistence = DeferredConfigPersistence::Writing(None);
            Some(config)
        } else {
            self.queue_config_persistence(&config);
            None
        }
    }

    pub(in crate::package) fn begin_deferred_config_persistence(
        &mut self,
        now: TimestampTicks,
    ) -> Option<FloatOutBoyConfigImage> {
        if !self.config_can_persist_now(now)
            || !matches!(self.config_eeprom_read_state, ConfigEepromReadState::Idle)
        {
            return None;
        }
        let DeferredConfigPersistence::Pending(config) = self.deferred_config_persistence else {
            return None;
        };
        self.deferred_config_persistence = DeferredConfigPersistence::Writing(None);
        Some(config)
    }

    pub(in crate::package) fn finish_config_persistence(
        &mut self,
        config: &FloatOutBoyConfigImage,
        stored: bool,
        now: TimestampTicks,
    ) {
        self.deferred_config_persistence = match self.deferred_config_persistence {
            DeferredConfigPersistence::Writing(Some(pending)) => {
                DeferredConfigPersistence::Pending(pending)
            }
            DeferredConfigPersistence::Writing(None) if stored => DeferredConfigPersistence::Clean,
            DeferredConfigPersistence::Writing(None) => DeferredConfigPersistence::Failed(*config),
            state => state,
        };
        if stored {
            self.acknowledge_command_config_write(now);
        }
    }

    pub(super) const fn config_persistence_blocks_engagement(&self) -> bool {
        matches!(
            self.deferred_config_persistence,
            DeferredConfigPersistence::Writing(_)
        )
    }

    pub(super) const fn config_eeprom_operation_in_progress(&self) -> bool {
        !matches!(self.config_eeprom_read_state, ConfigEepromReadState::Idle)
            || self.config_persistence_blocks_engagement()
    }

    pub(in crate::package) fn begin_config_eeprom_read(&mut self) -> bool {
        if self.config_eeprom_operation_in_progress() {
            return false;
        }
        self.config_eeprom_read_state = ConfigEepromReadState::Reading;
        true
    }

    pub(in crate::package) fn finish_config_eeprom_read(&mut self) {
        self.config_eeprom_read_state = ConfigEepromReadState::Idle;
    }

    pub(super) fn retry_failed_config_persistence_after_ride(&mut self) {
        if let DeferredConfigPersistence::Failed(config) = self.deferred_config_persistence {
            self.deferred_config_persistence = DeferredConfigPersistence::Pending(config);
        }
    }

    pub(in crate::package) fn finish_configure_without_firmware_migration(&mut self) {
        self.alert_after_configure();
    }

    pub(in crate::package) fn record_firmware_imu_migration(
        &mut self,
        migration: FirmwareImuMigration,
    ) {
        self.firmware_imu_migration = migration;
    }

    pub(super) fn replace_active_config(&mut self, config: &FloatOutBoyConfigImage) {
        self.serialized_config = *config;
        self.reconfigure_active_config();
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn reconfigure_active_config(&mut self) {
        #[cfg(test)]
        CONFIG_RECONFIGURE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
        self.refresh_balance_filter_config();
        self.refresh_led_config_runtime_state();
        self.refresh_config_runtime_state();
    }

    #[cfg(test)]
    pub(super) fn reset_config_reconfigure_count_for_test() {
        CONFIG_RECONFIGURE_COUNT.with(|count| count.set(0));
    }

    #[cfg(test)]
    pub(super) fn config_reconfigure_count_for_test() -> u8 {
        CONFIG_RECONFIGURE_COUNT.with(std::cell::Cell::get)
    }

    pub(in crate::package) fn apply_persisted_config(
        &mut self,
        loaded: &FloatOutBoyPersistedConfig,
    ) {
        self.serialized_config = loaded.config;
        self.config_load_outcome = loaded.outcome;
        self.deferred_config_persistence = match self.deferred_config_persistence {
            DeferredConfigPersistence::Writing(_) => {
                DeferredConfigPersistence::Writing(Some(loaded.config))
            }
            _ => DeferredConfigPersistence::Clean,
        };
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
        let ride_state = self.all_data_payloads.ride_state();
        if !matches!(ride_state.mode(), FloatOutBoyMode::Normal) {
            return None;
        }
        if ride_state.run_state() == FloatOutBoyRunState::Running {
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
            self.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
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
        self.serialized_config
            .editor()
            .set(Metadata::DISABLED_FIELD, disabled);
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

    #[cfg(test)]
    pub(in crate::package) fn load_persisted_config_on_main_thread(&mut self, now: TimestampTicks) {
        let loaded = vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
        self.apply_persisted_config(&loaded);
        self.initialize_time_epochs(now);
    }

    #[cfg(test)]
    pub(in crate::package) fn configure_loaded_config_on_main_thread(&mut self) {
        self.refresh_balance_filter_config();
        super::config_runtime::refresh_led_effects(self);
        self.refresh_config_runtime_state();
        let migration =
            vescpkg_rs::test_support::with_firmware_effects(migrate_legacy_firmware_imu_settings);
        self.finish_startup_configure(migration);
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
        let run_state = self.all_data_payloads.ride_state().run_state();
        let alert = match run_state {
            FloatOutBoyRunState::Disabled => FloatOutBoyBeeperAlert::Short(3),
            // Intentional Refloat 1.2.1 bug fix from upstream 37cf343:
            // leave the beeper free for the READY battery-status alert.
            FloatOutBoyRunState::Startup => return,
            FloatOutBoyRunState::Ready | FloatOutBoyRunState::Running => {
                FloatOutBoyBeeperAlert::Short(1)
            }
        };
        self.alert_beeper(alert);
    }

    pub(super) fn handle_config_command(
        &mut self,
        command: FloatOutBoyAppDataCommand,
        now: &mut impl FnMut() -> TimestampTicks,
    ) -> bool {
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

    pub(in crate::package) fn serialized_config(
        &self,
    ) -> &[u8; crate::config::FLOAT_OUT_BOY_CONFIG_LEN] {
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

    #[cfg(test)]
    pub(in crate::package) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Some(config) = self.prepare_serialized_config(config) else {
            return false;
        };
        let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
            store_persisted_config(effects, &config)
        });
        if stored {
            self.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
        }
        self.replace_active_config(&config);
        let migration =
            vescpkg_rs::test_support::with_firmware_effects(migrate_legacy_firmware_imu_settings);
        self.finish_configure_active(migration);
        true
    }

    pub(super) fn refresh_balance_filter_config(&mut self) {
        // C map: `reconfigure(d)` refreshes Mahony filter gains through
        // `balance_filter_configure` at `third_party/float-out-boy/src/main.c:154-160`.
        let filter = self.serialized_config.filter();
        self.balance_filter
            .configure(filter.mahony_kp(), filter.mahony_kp_roll());
    }

    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        // Refloat 7c72c6d3 hardcodes the main thread to 500 Hz; legacy `hertz`
        // bytes are retained only as storage-layout padding.
        self.serialized_config.startup().loop_time_us()
    }

    #[cfg(target_arch = "arm")]
    pub(crate) fn configured_main_loop_sample_rate(&self) -> vescpkg_rs::prelude::SampleRate {
        self.serialized_config.startup().sample_rate()
    }

    #[cfg(test)]
    pub(super) const fn firmware_imu_migration_for_test(&self) -> FirmwareImuMigration {
        self.firmware_imu_migration
    }

    #[cfg(test)]
    pub(super) const fn config_load_outcome_for_test(&self) -> FloatOutBoyConfigLoadOutcome {
        self.config_load_outcome
    }
}
