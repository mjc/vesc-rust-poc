use super::FloatOutBoyPackageState;
use super::float_out_boy_command_payload;
use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyMode, FloatOutBoyRideState,
};
use crate::domain::{FloatOutBoyAppDataCommand, FloatOutBoyRunState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct FloatOutBoyHandtestSafetyConfig(FloatOutBoyConfigImage);

impl FloatOutBoyHandtestSafetyConfig {
    fn from_config(mut config: FloatOutBoyConfigImage) -> Option<Self> {
        // C map: `cmd_handtest` applies temporary safety overrides only in
        // `third_party/float-out-boy/src/main.c:1431-1446`.
        if config.editor().apply_handtest_safety_overrides() {
            Some(Self(config))
        } else {
            None
        }
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
    fn from_packet(bytes: &[u8]) -> Option<Self> {
        // C map: `COMMAND_HANDTEST` uses the first payload byte as the on/off
        // flag at `third_party/float-out-boy/src/main.c:2226-2228`.
        match float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::HandTest) {
            Some([on, ..]) => Some(Self::from_flag(*on)),
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
        let ride_state = state.all_data_payloads.base().status().ride_state();
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

fn float_out_boy_ride_state_with_mode(
    ride_state: FloatOutBoyRideState,
    mode: FloatOutBoyMode,
) -> FloatOutBoyRideState {
    // C map: `cmd_handtest` writes only the mode field at
    // `third_party/float-out-boy/src/main.c:1430`.
    FloatOutBoyRideState::new(
        ride_state.run_state(),
        mode,
        ride_state.setpoint_adjustment(),
        ride_state.stop_condition(),
    )
    .with_charging(ride_state.charging())
    .with_wheelslip(ride_state.wheelslip())
    .with_darkride(ride_state.darkride())
}

fn float_out_boy_payloads_with_ride_state(
    payloads: FloatOutBoyAllDataPayloads,
    ride_state: FloatOutBoyRideState,
) -> FloatOutBoyAllDataPayloads {
    // C map: `cmd_handtest` preserves the packed ride-state fields while
    // swapping only mode at `third_party/float-out-boy/src/main.c:1430-1449`.
    let base = payloads.base();
    let status = base.status();
    let base = FloatOutBoyAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        FloatOutBoyAllDataStatus::new(ride_state, status.beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );

    FloatOutBoyAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

impl FloatOutBoyPackageState {
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(in crate::package) fn prepare_handtest_packet(&mut self, bytes: &[u8]) -> Option<bool> {
        // QML sends `[101, COMMAND_HANDTEST, on]` from `ui.qml.in:764-768`;
        // Float Out Boy C dispatches it at `third_party/float-out-boy/src/main.c:2226-2228`
        // and applies READY/NORMAL/HANDTEST gates at `third_party/float-out-boy/src/main.c:1421-1430`.
        FloatOutBoyHandtestRequest::from_packet(bytes).map(|request| request.apply_to(self))
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
                let migration = vescpkg_rs::test_support::with_firmware_effects(
                    super::migrate_legacy_firmware_imu_settings,
                );
                self.finish_configure_active(migration);
            }
        }
        true
    }

    pub(super) fn set_ride_mode(&mut self, mode: FloatOutBoyMode) {
        // HANDTEST changes only `state.mode` in C at `third_party/float-out-boy/src/main.c:1430`;
        // preserve the rest of the packed Rust ride state while swapping mode.
        let payloads = self.all_data_payloads;
        let ride_state = payloads.base().status().ride_state();
        self.all_data_payloads = float_out_boy_payloads_with_ride_state(
            payloads,
            float_out_boy_ride_state_with_mode(ride_state, mode),
        );
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
        let ride_state = self.all_data_payloads.base().status().ride_state();
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
