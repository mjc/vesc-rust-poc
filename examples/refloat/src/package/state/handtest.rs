use super::{RefloatPackageState, refloat_command_payload};
use crate::config::{
    REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET, REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET,
    REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET, REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET,
    REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET, REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET,
    REFLOAT_CONFIG_KI_OFFSET, REFLOAT_CONFIG_KP_BRAKE_OFFSET, REFLOAT_CONFIG_KP2_BRAKE_OFFSET,
    REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET, REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET,
    REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET, REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET,
    REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET,
};
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand,
    RefloatMode, RefloatRideState, RefloatRunState,
};

impl RefloatPackageState {
    pub(super) fn handle_handtest_packet(&mut self, bytes: &[u8]) -> bool {
        // QML sends `[101, COMMAND_HANDTEST, on]` from `ui.qml.in:764-768`;
        // Refloat C dispatches it at `third_party/refloat/src/main.c:2226-2228`
        // and applies READY/NORMAL/HANDTEST gates at `third_party/refloat/src/main.c:1421-1430`.
        match refloat_command_payload(bytes, RefloatAppDataCommand::HandTest) {
            Some([on, ..]) => {
                let ride_state = self.all_data_payloads.base().status().ride_state();
                if let (RefloatRunState::Ready, RefloatMode::Normal | RefloatMode::HandTest) =
                    (ride_state.run_state(), ride_state.mode())
                {
                    let mode = match *on {
                        0 => RefloatMode::Normal,
                        _ => RefloatMode::HandTest,
                    };
                    self.set_ride_mode(mode);
                    self.apply_handtest_config(matches!(mode, RefloatMode::HandTest));
                }
                true
            }
            _ => false,
        }
    }

    fn set_ride_mode(&mut self, mode: RefloatMode) {
        // HANDTEST changes only `state.mode` in C at `third_party/refloat/src/main.c:1430`;
        // preserve the rest of the packed Rust ride state while swapping mode.
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let status = base.status();
        let ride_state = status.ride_state();
        let ride_state = RefloatRideState::new(
            ride_state.run_state(),
            mode,
            ride_state.setpoint_adjustment(),
            ride_state.stop_condition(),
        )
        .with_charging(ride_state.charging())
        .with_wheelslip(ride_state.wheelslip())
        .with_darkride(ride_state.darkride());
        let base = RefloatAllDataBasePayload::new(
            base.balance_current(),
            base.attitude(),
            RefloatAllDataStatus::new(ride_state, status.beep_reason()),
            base.footpad(),
            base.setpoints(),
            base.booster_current(),
            base.motor(),
        );
        self.all_data_payloads =
            RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
    }

    fn apply_handtest_config(&mut self, enabled: bool) {
        if !enabled {
            if let Some(config) = self.handtest_config_backup.take() {
                self.serialized_config = config;
            }
            return;
        }

        if self.handtest_config_backup.is_none() {
            self.handtest_config_backup = Some(self.serialized_config);
        }

        // Refloat C applies temporary HANDTEST safety config at
        // `third_party/refloat/src/main.c:1431-1446` and restores from EEPROM on off at
        // `third_party/refloat/src/main.c:1447-1449`.
        let mut config = self.serialized_config;
        let writes_ok = [
            (REFLOAT_CONFIG_KI_OFFSET, 0),
            (REFLOAT_CONFIG_KP_BRAKE_OFFSET, 100),
            (REFLOAT_CONFIG_KP2_BRAKE_OFFSET, 100),
            (REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET, 10_000),
            (REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET, 10_000),
            (REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET, 0),
            (REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET, 0),
            (REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET, 0),
            (REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET, 0),
            (REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET, 0),
            (REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET, 0),
            (REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET, 0),
            (REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET, 50),
            (REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET, 50),
        ]
        .into_iter()
        .all(|(offset, value)| Self::set_config_be_u16(&mut config, offset, value));
        if writes_ok {
            self.serialized_config = config;
        }
    }
}
