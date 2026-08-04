use super::config_storage::{FLOAT_OUT_BOY_EEPROM_LEN, FloatOutBoyEepromImage};
use super::*;

impl FloatOutBoyEepromImage {
    pub(super) const fn into_bytes(self) -> [u8; FLOAT_OUT_BOY_EEPROM_LEN] {
        *self.as_bytes()
    }
}

impl FloatOutBoyPackageState {
    /// Build startup state and apply the config persisted by firmware.
    ///
    /// Upstream `data_init` reads EEPROM and falls back to generated defaults
    /// at `third_party/float-out-boy/src/main.c:1160-1185`.
    pub(in crate::package) fn from_persisted_config() -> Self {
        let mut state = Self::default();
        state.load_persisted_config_on_main_thread(vescpkg_rs::FirmwareClock::current_timestamp());
        state.configure_loaded_config_on_main_thread();
        state
    }

    pub(crate) const fn aux_backup_failures(&self) -> u32 {
        self.aux_backup_failures
    }

    pub(crate) const fn bms_sample_for_test(&self) -> FloatOutBoyBmsSample {
        self.bms.sample()
    }

    pub(crate) const fn bms_faults_for_test(&self) -> crate::bms::FloatOutBoyBmsFaults {
        self.bms.faults()
    }

    pub(crate) const fn recorded_firmware_version(&self) -> Option<FirmwareVersion> {
        self.firmware_version
    }

    /// Apply and clear a pending motor-current request.
    pub fn apply_requested_motor_current(&mut self, motor: &impl MotorOutput) -> bool {
        self.motor_control
            .apply_requested_current(motor)
            .unwrap_or(false)
    }

    pub(crate) fn set_balance_filter_for_test(&mut self, balance_filter: BalanceFilter) {
        self.balance_filter = balance_filter;
    }

    pub(crate) const fn configured_mahony_gains_for_test(
        &self,
    ) -> (vescpkg_rs::MahonyPitchGain, vescpkg_rs::MahonyRollGain) {
        self.balance_filter.configured_gains()
    }

    pub(crate) const fn lcm_hardware_mode_for_test(&self) -> u8 {
        self.lcm.hardware_mode()
    }

    pub(in crate::package) fn replace_idle_epoch_for_test(&mut self, now: TimestampTicks) {
        self.idle_ticks = now;
    }

    pub(in crate::package) const fn idle_epoch_for_test(&self) -> TimestampTicks {
        self.idle_ticks
    }

    pub(in crate::package) fn load_persisted_config_on_main_thread(&mut self, now: TimestampTicks) {
        let loaded = vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
        self.apply_persisted_config(&loaded);
        self.initialize_time_epochs(now);
    }

    pub(in crate::package) fn configure_loaded_config_on_main_thread(&mut self) {
        self.refresh_balance_filter_config();
        super::config_runtime::refresh_led_effects(self);
        self.refresh_config_runtime_state();
        let migration =
            vescpkg_rs::test_support::with_firmware_effects(migrate_legacy_firmware_imu_settings);
        self.finish_startup_configure(migration);
    }

    pub(in crate::package) fn replace_serialized_config_for_test(
        &mut self,
        config: &crate::config::FloatOutBoyConfigImage,
    ) {
        self.serialized_config = *config;
    }

    pub(in crate::package) fn balance_config_for_test(
        &self,
    ) -> crate::config::FloatOutBoyBalanceConfig<'_> {
        self.serialized_config.balance()
    }

    pub(in crate::package) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Some(config) = self.prepare_serialized_config(config) else {
            return false;
        };
        let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
            store_persisted_config(effects, &config)
        });
        if stored {
            self.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
        }
        self.replace_active_config(&config);
        let migration =
            vescpkg_rs::test_support::with_firmware_effects(migrate_legacy_firmware_imu_settings);
        self.finish_configure_active(migration);
        true
    }

    pub(super) const fn firmware_imu_migration_for_test(&self) -> FirmwareImuMigration {
        self.firmware_imu_migration
    }

    pub(super) const fn config_load_outcome_for_test(&self) -> FloatOutBoyConfigLoadOutcome {
        self.config_load_outcome
    }

    /// Refresh the source-backed runtime slices that Float Out Boy updates near the
    /// top of `float_out_boy_thd`.
    ///
    /// C map: Float Out Boy v1.2.1 `imu_ref_callback` starts at `third_party/float-out-boy/src/main.c:760`.
    ///
    /// Upstream applies `configure(d)` before runtime work at
    /// `third_party/float-out-boy/src/main.c:184-191`, updates IMU at `third_party/float-out-boy/src/main.c:775`, motor data at
    /// `third_party/float-out-boy/src/main.c:796`, and performs the `STATE_STARTUP` -> `STATE_READY`
    /// gate at `third_party/float-out-boy/src/main.c:833-838`.
    pub(crate) fn refresh_runtime_state(
        &mut self,
        telemetry: &impl MotorTelemetry,
        imu: &impl Imu,
        system_time_ticks: TimestampTicks,
    ) {
        self.refresh_config_runtime_state();
        self.refresh_motor_runtime_state(telemetry);
        self.alert_tracker.update_firmware_fault(
            telemetry.firmware_fault(),
            system_time_ticks,
            self.serialized_config.persistent_fatal_error(),
        );
        let _ = self.refresh_imu_runtime_state(imu, system_time_ticks);
    }

    pub(super) fn handle_effectful_packet_for_test(
        &mut self,
        now: &mut impl FnMut() -> TimestampTicks,
        bytes: &[u8],
    ) -> Option<bool> {
        let [package_id, command, payload @ ..] = bytes else {
            return None;
        };
        if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
            return None;
        }
        let Ok(command) = FloatOutBoyAppDataCommand::try_from(*command) else {
            return None;
        };

        match command {
            FloatOutBoyAppDataCommand::ConfigSave => {
                let config = self.active_config_image();
                let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
                    store_persisted_config(effects, &config)
                });
                if stored {
                    self.acknowledge_command_config_write(now());
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::ConfigRestore => {
                let loaded = vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                self.begin_restore_persisted_config(&loaded, now());
                let migration = vescpkg_rs::test_support::with_firmware_effects(
                    migrate_legacy_firmware_imu_settings,
                );
                self.finish_configure_active(migration);
                Some(true)
            }
            FloatOutBoyAppDataCommand::Lock => {
                let Some(disabled) = payload.first() else {
                    return Some(false);
                };
                if !self.is_running() {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    if let Some(config) =
                        self.apply_lock_from_persisted(&loaded, *disabled != 0, now())
                    {
                        let stored = vescpkg_rs::test_support::with_firmware_effects(|effects| {
                            store_persisted_config(effects, &config)
                        });
                        if stored {
                            self.acknowledge_command_config_write(now());
                        }
                        let migration = vescpkg_rs::test_support::with_firmware_effects(
                            migrate_legacy_firmware_imu_settings,
                        );
                        self.finish_configure_active(migration);
                    }
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::HandTest => {
                let Some(restore) = self.prepare_handtest_packet(bytes) else {
                    return Some(false);
                };
                if restore {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    if self.commit_handtest_restore(
                        &loaded,
                        vescpkg_rs::FirmwareClock::current_timestamp(),
                    ) {
                        let migration = vescpkg_rs::test_support::with_firmware_effects(
                            migrate_legacy_firmware_imu_settings,
                        );
                        self.finish_configure_active(migration);
                    }
                }
                Some(true)
            }
            FloatOutBoyAppDataCommand::Flywheel => {
                let Some(restore) = self.prepare_flywheel_packet(bytes) else {
                    return Some(false);
                };
                if restore {
                    let loaded =
                        vescpkg_rs::test_support::with_firmware_effects(load_persisted_config);
                    self.commit_flywheel_restore(
                        &loaded,
                        vescpkg_rs::FirmwareClock::current_timestamp(),
                    );
                    let migration = vescpkg_rs::test_support::with_firmware_effects(
                        migrate_legacy_firmware_imu_settings,
                    );
                    self.finish_configure_active(migration);
                }
                Some(true)
            }
            _ => None,
        }
    }
}
