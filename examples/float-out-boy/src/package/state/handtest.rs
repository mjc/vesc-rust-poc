use super::FloatOutBoyPackageState;
#[cfg(test)]
use super::float_out_boy_command_payload;
use crate::config::FloatOutBoyConfigImage;
#[cfg(test)]
use crate::domain::FloatOutBoyAppDataCommand;
use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct FloatOutBoyHandtestSafetyConfig(FloatOutBoyConfigImage);

impl FloatOutBoyHandtestSafetyConfig {
    fn from_config(mut config: FloatOutBoyConfigImage) -> Option<Self> {
        // C map: `cmd_handtest` applies temporary safety overrides only in
        // `third_party/float-out-boy/src/main.c:1431-1446`.
        config
            .editor()
            .apply_handtest_safety_overrides()
            .then_some(Self(config))
    }

    const fn into_image(self) -> FloatOutBoyConfigImage {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyHandtestRequest {
    Enable,
    Disable,
}

impl FloatOutBoyHandtestRequest {
    fn from_payload(payload: &[u8]) -> Option<Self> {
        // C map: `COMMAND_HANDTEST` uses the first payload byte as the on/off
        // flag at `third_party/float-out-boy/src/main.c:2226-2228`.
        match payload {
            [on, ..] => Some(Self::from_flag(*on)),
            _ => None,
        }
    }

    const fn from_flag(on: u8) -> Self {
        // C map: `cmd_handtest` treats nonzero as HANDTEST and zero as NORMAL
        // at `third_party/float-out-boy/src/main.c:1430-1449`.
        match on {
            0 => Self::Disable,
            _ => Self::Enable,
        }
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn apply_to(self, state: &mut FloatOutBoyPackageState) -> bool {
        // C map: `cmd_handtest` only applies when the board is READY and mode
        // is NORMAL or HANDTEST at `third_party/float-out-boy/src/main.c:1426-1430`.
        let ride_state = state.all_data_payloads.ride_state();
        if ride_state.run_state() == FloatOutBoyRunState::Ready
            && matches!(
                ride_state.mode(),
                FloatOutBoyMode::Normal | FloatOutBoyMode::HandTest
            )
        {
            let mode = match self {
                Self::Enable => FloatOutBoyMode::HandTest,
                Self::Disable => FloatOutBoyMode::Normal,
            };
            state.set_ride_mode(mode);
            match self {
                Self::Enable => {
                    state.apply_handtest_safety_config();
                    false
                }
                Self::Disable => true,
            }
        } else {
            false
        }
    }
}

impl FloatOutBoyPackageState {
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(in crate::package) fn prepare_handtest_command(&mut self, payload: &[u8]) -> Option<bool> {
        // QML sends `[101, COMMAND_HANDTEST, on]` from `ui.qml.in:764-768`;
        // Float Out Boy C dispatches it at `third_party/float-out-boy/src/main.c:2226-2228`
        // and applies READY/NORMAL/HANDTEST gates at `third_party/float-out-boy/src/main.c:1421-1430`.
        let request = FloatOutBoyHandtestRequest::from_payload(payload)?;
        if self.config_eeprom_operation_in_progress() {
            return None;
        }
        let restore = request.apply_to(self);
        if restore {
            debug_assert!(self.begin_config_eeprom_read());
        }
        Some(restore)
    }

    #[cfg(test)]
    pub(in crate::package) fn prepare_handtest_packet(&mut self, bytes: &[u8]) -> Option<bool> {
        let payload = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::HandTest)?;
        self.prepare_handtest_command(payload)
    }

    #[cfg(test)]
    pub(super) fn handle_handtest_packet(&mut self, bytes: &[u8]) -> bool {
        let Some(restore) = self.prepare_handtest_packet(bytes) else {
            return false;
        };
        if restore {
            let loaded =
                vescpkg_rs::test_support::with_firmware_effects(super::load_persisted_config);
            let now = vescpkg_rs::FirmwareClock::current_timestamp();
            if self.commit_handtest_restore(&loaded, now) {
                self.finish_config_eeprom_read();
                let migration = vescpkg_rs::test_support::with_firmware_effects(
                    super::migrate_legacy_firmware_imu_settings,
                );
                self.finish_configure_active(migration);
            } else {
                self.finish_config_eeprom_read();
            }
        }
        true
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(super) fn set_ride_mode(&mut self, mode: FloatOutBoyMode) {
        // HANDTEST changes only `state.mode` in C at `third_party/float-out-boy/src/main.c:1430`;
        // preserve the rest of the packed Rust ride state while swapping mode.
        let ride_state = self.all_data_payloads.ride_state().with_mode(mode);
        self.all_data_payloads = self.all_data_payloads.with_ride_state(ride_state);
    }

    #[cfg_attr(target_arch = "arm", inline(never))]
    fn apply_handtest_safety_config(&mut self) {
        // C map: enabling HANDTEST applies temporary safety overrides at
        // `third_party/float-out-boy/src/main.c:1431-1446`.
        if let Some(config) = Self::handtest_safety_config(&self.serialized_config) {
            self.serialized_config = config;
        }
    }

    pub(in crate::package) fn commit_handtest_restore(
        &mut self,
        loaded: &super::FloatOutBoyPersistedConfig,
        now: vescpkg_rs::TimestampTicks,
    ) -> bool {
        let ride_state = self.all_data_payloads.ride_state();
        if ride_state.run_state() != FloatOutBoyRunState::Ready
            || ride_state.mode() != FloatOutBoyMode::Normal
        {
            return false;
        }
        self.apply_persisted_config(loaded);
        self.begin_configure_active(now);
        true
    }

    fn handtest_safety_config(config: &FloatOutBoyConfigImage) -> Option<FloatOutBoyConfigImage> {
        // Float Out Boy C applies temporary HANDTEST safety config at
        // `third_party/float-out-boy/src/main.c:1431-1446` and restores from EEPROM on off at
        // `third_party/float-out-boy/src/main.c:1447-1449`.
        FloatOutBoyHandtestSafetyConfig::from_config(*config)
            .map(FloatOutBoyHandtestSafetyConfig::into_image)
    }
}

#[cfg(test)]
mod tests;
