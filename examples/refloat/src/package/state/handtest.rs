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

const REFLOAT_HANDTEST_CONFIG_WRITES: [(usize, u16); 14] = [
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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefloatHandtestRequest {
    Enable,
    Disable,
}

impl RefloatHandtestRequest {
    fn from_packet(bytes: &[u8]) -> Option<Self> {
        match refloat_command_payload(bytes, RefloatAppDataCommand::HandTest) {
            Some([on, ..]) => Some(Self::from_flag(*on)),
            _ => None,
        }
    }

    const fn from_flag(on: u8) -> Self {
        match on {
            0 => Self::Disable,
            _ => Self::Enable,
        }
    }

    const fn mode(self) -> RefloatMode {
        match self {
            Self::Enable => RefloatMode::HandTest,
            Self::Disable => RefloatMode::Normal,
        }
    }

    fn apply_to(self, state: &mut RefloatPackageState) {
        let ride_state = state.all_data_payloads.base().status().ride_state();
        if let Some(mode) = self.mode_for(ride_state) {
            state.set_ride_mode(mode);
            state.apply_handtest_config(self);
        }
    }

    fn mode_for(self, ride_state: RefloatRideState) -> Option<RefloatMode> {
        match (ride_state.run_state(), ride_state.mode()) {
            (RefloatRunState::Ready, RefloatMode::Normal | RefloatMode::HandTest) => {
                Some(self.mode())
            }
            _ => None,
        }
    }
}

fn refloat_ride_state_with_mode(
    ride_state: RefloatRideState,
    mode: RefloatMode,
) -> RefloatRideState {
    RefloatRideState::new(
        ride_state.run_state(),
        mode,
        ride_state.setpoint_adjustment(),
        ride_state.stop_condition(),
    )
    .with_charging(ride_state.charging())
    .with_wheelslip(ride_state.wheelslip())
    .with_darkride(ride_state.darkride())
}

fn refloat_payloads_with_ride_state(
    payloads: RefloatAllDataPayloads,
    ride_state: RefloatRideState,
) -> RefloatAllDataPayloads {
    let base = payloads.base();
    let status = base.status();
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, status.beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );

    RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4())
}

impl RefloatPackageState {
    pub(super) fn handle_handtest_packet(&mut self, bytes: &[u8]) -> bool {
        // QML sends `[101, COMMAND_HANDTEST, on]` from `ui.qml.in:764-768`;
        // Refloat C dispatches it at `third_party/refloat/src/main.c:2226-2228`
        // and applies READY/NORMAL/HANDTEST gates at `third_party/refloat/src/main.c:1421-1430`.
        match RefloatHandtestRequest::from_packet(bytes) {
            Some(request) => {
                request.apply_to(self);
                true
            }
            None => false,
        }
    }

    fn set_ride_mode(&mut self, mode: RefloatMode) {
        // HANDTEST changes only `state.mode` in C at `third_party/refloat/src/main.c:1430`;
        // preserve the rest of the packed Rust ride state while swapping mode.
        let payloads = self.all_data_payloads;
        let ride_state = payloads.base().status().ride_state();
        self.all_data_payloads = refloat_payloads_with_ride_state(
            payloads,
            refloat_ride_state_with_mode(ride_state, mode),
        );
    }

    fn apply_handtest_config(&mut self, request: RefloatHandtestRequest) {
        match request {
            RefloatHandtestRequest::Enable => self.apply_handtest_safety_config(),
            RefloatHandtestRequest::Disable => self.restore_handtest_config(),
        }
    }

    fn restore_handtest_config(&mut self) {
        if let Some(config) = self.handtest_config_backup.take() {
            self.serialized_config = config;
        }
    }

    fn apply_handtest_safety_config(&mut self) {
        self.handtest_config_backup
            .get_or_insert(self.serialized_config);

        if let Some(config) = Self::handtest_safety_config(self.serialized_config) {
            self.serialized_config = config;
        }
    }

    fn handtest_safety_config(mut config: [u8; 276]) -> Option<[u8; 276]> {
        // Refloat C applies temporary HANDTEST safety config at
        // `third_party/refloat/src/main.c:1431-1446` and restores from EEPROM on off at
        // `third_party/refloat/src/main.c:1447-1449`.
        if REFLOAT_HANDTEST_CONFIG_WRITES
            .into_iter()
            .all(|(offset, value)| Self::set_config_be_u16(&mut config, offset, value))
        {
            Some(config)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RefloatSetpointAdjustment, RefloatStopCondition};

    fn ride_state(run_state: RefloatRunState, mode: RefloatMode) -> RefloatRideState {
        RefloatRideState::new(
            run_state,
            mode,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::None,
        )
    }

    #[test]
    fn handtest_request_selects_mode_only_while_ready_like_refloat() {
        assert_eq!(
            RefloatHandtestRequest::from_flag(0).mode(),
            RefloatMode::Normal
        );
        assert_eq!(
            RefloatHandtestRequest::from_flag(1).mode(),
            RefloatMode::HandTest
        );
        assert_eq!(
            RefloatHandtestRequest::Enable
                .mode_for(ride_state(RefloatRunState::Ready, RefloatMode::Normal,)),
            Some(RefloatMode::HandTest),
        );
        assert_eq!(
            RefloatHandtestRequest::Enable
                .mode_for(ride_state(RefloatRunState::Running, RefloatMode::Normal,)),
            None,
        );
        assert_eq!(
            RefloatHandtestRequest::Enable
                .mode_for(ride_state(RefloatRunState::Ready, RefloatMode::Flywheel,)),
            None,
        );
    }
}
