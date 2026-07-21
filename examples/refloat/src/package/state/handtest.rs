use super::{RefloatPackageState, refloat_command_payload};
use crate::config::RefloatConfigImage;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand,
    RefloatMode, RefloatRideState, RefloatRunState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct RefloatHandtestSafetyConfig(RefloatConfigImage);

impl RefloatHandtestSafetyConfig {
    fn from_config(mut config: RefloatConfigImage) -> Option<Self> {
        // C map: `cmd_handtest` applies temporary safety overrides only in
        // `third_party/refloat/src/main.c:1431-1446`.
        if config.editor().apply_handtest_safety_overrides() {
            Some(Self(config))
        } else {
            None
        }
    }

    const fn into_image(self) -> RefloatConfigImage {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefloatHandtestRequest {
    Enable,
    Disable,
}

impl RefloatHandtestRequest {
    fn from_packet(bytes: &[u8]) -> Option<Self> {
        // C map: `COMMAND_HANDTEST` uses the first payload byte as the on/off
        // flag at `third_party/refloat/src/main.c:2226-2228`.
        match refloat_command_payload(bytes, RefloatAppDataCommand::HandTest) {
            Some([on, ..]) => Some(Self::from_flag(*on)),
            _ => None,
        }
    }

    const fn from_flag(on: u8) -> Self {
        // C map: `cmd_handtest` treats nonzero as HANDTEST and zero as NORMAL
        // at `third_party/refloat/src/main.c:1430-1449`.
        match on {
            0 => Self::Disable,
            _ => Self::Enable,
        }
    }

    const fn mode(self) -> RefloatMode {
        // C map: HANDTEST toggles between NORMAL and HANDTEST in
        // `third_party/refloat/src/main.c:1430`.
        match self {
            Self::Enable => RefloatMode::HandTest,
            Self::Disable => RefloatMode::Normal,
        }
    }

    fn apply_to(self, state: &mut RefloatPackageState) {
        // C map: `cmd_handtest` only applies when the board is READY and mode
        // is NORMAL or HANDTEST at `third_party/refloat/src/main.c:1426-1430`.
        let ride_state = state.all_data_payloads.base().status().ride_state();
        if let Some(mode) = self.mode_for(ride_state) {
            state.set_ride_mode(mode);
            state.apply_handtest_config(self);
        }
    }

    fn mode_for(self, ride_state: RefloatRideState) -> Option<RefloatMode> {
        // C map: `cmd_handtest` refuses changes unless READY and the current
        // mode is NORMAL or HANDTEST at `third_party/refloat/src/main.c:1426-1430`.
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
    // C map: `cmd_handtest` writes only the mode field at
    // `third_party/refloat/src/main.c:1430`.
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
    // C map: `cmd_handtest` preserves the packed ride-state fields while
    // swapping only mode at `third_party/refloat/src/main.c:1430-1449`.
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

    pub(super) fn set_ride_mode(&mut self, mode: RefloatMode) {
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
        // C map: `cmd_handtest` branches to enable or restore behavior based
        // on the packet byte at `third_party/refloat/src/main.c:1431-1449`.
        match request {
            RefloatHandtestRequest::Enable => self.apply_handtest_safety_config(),
            RefloatHandtestRequest::Disable => self.restore_handtest_config(),
        }
    }

    fn restore_handtest_config(&mut self) {
        // C map: disabling HANDTEST restores the prior config from EEPROM in
        // `third_party/refloat/src/main.c:1447-1449`.
        if let Some(config) = self.handtest_config_backup.take() {
            self.serialized_config = config;
        }
    }

    fn apply_handtest_safety_config(&mut self) {
        // C map: enabling HANDTEST preserves the original config, then applies
        // the temporary safety overrides at `third_party/refloat/src/main.c:1431-1446`.
        self.handtest_config_backup
            .get_or_insert(self.serialized_config);

        if let Some(config) = Self::handtest_safety_config(self.serialized_config) {
            self.serialized_config = config;
        }
    }

    fn handtest_safety_config(config: RefloatConfigImage) -> Option<RefloatConfigImage> {
        // Refloat C applies temporary HANDTEST safety config at
        // `third_party/refloat/src/main.c:1431-1446` and restores from EEPROM on off at
        // `third_party/refloat/src/main.c:1447-1449`.
        RefloatHandtestSafetyConfig::from_config(config)
            .map(RefloatHandtestSafetyConfig::into_image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        REFLOAT_APP_DATA_PACKAGE_ID, RefloatAllDataAttitude, RefloatAllDataBasePayload,
        RefloatAllDataPayloads, RefloatAllDataStatus, RefloatAppDataCommand, RefloatMode,
        RefloatRealtimeBalanceCurrent, RefloatRealtimeBalancePitch, RefloatRealtimeBoosterCurrent,
        RefloatRealtimeRuntimeSetpoint, RefloatRealtimeRuntimeSetpoints, RefloatRideState,
        RefloatRunState, RefloatSetpointAdjustment, RefloatStopCondition,
    };
    use crate::package::test_support::{
        balance_filter_with_pitch, editable_config_from_bytes, editable_config_from_state,
        sample_all_data_payloads_with_ride_state, tick_refloat_state_and_handle_packet,
    };
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FirmwareTest;

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

    #[test]
    fn handtest_packet_toggles_ready_mode_and_safety_config_like_refloat_qml() {
        // QML sends COMMAND_HANDTEST at `refloat/ui.qml.in:764-768`; C toggles
        // mode and temporary safety config at `third_party/refloat/src/main.c:1421-1450`.
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));
        let original_config = *state.serialized_config();

        assert!(state.handle_handtest_packet(&[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::HandTest.id(),
            1,
        ]));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            RefloatMode::HandTest
        );
        let mut expected_handtest_config = editable_config_from_bytes(&original_config);
        assert!(
            expected_handtest_config
                .editor()
                .apply_handtest_safety_overrides()
        );
        assert_eq!(
            state.serialized_config(),
            expected_handtest_config.as_bytes()
        );

        assert!(state.handle_handtest_packet(&[
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::HandTest.id(),
            0,
        ]));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            RefloatMode::Normal
        );
        assert_eq!(state.serialized_config(), &original_config);
    }

    #[test]
    fn app_data_handtest_running_recenters_start_setpoint_like_refloat_loop() {
        let lifecycle = TimestampTicks::from_ticks(0);
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_ready(true);
        let imu = telemetry.imu();
        let payloads = sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::HandTest,
        );
        let base = payloads.base();
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::HandTest,
            RefloatSetpointAdjustment::Centering,
            RefloatStopCondition::None,
        );
        let board = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0));
        let zero = RefloatRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints = RefloatRealtimeRuntimeSetpoints::new(board, zero, zero, zero, zero, zero);
        let base = RefloatAllDataBasePayload::new(
            RefloatRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            RefloatAllDataAttitude::new(
                RefloatRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            RefloatAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            setpoints,
            RefloatRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = RefloatPackageState::new(RefloatAllDataPayloads::new(
            base,
            payloads.mode2(),
            payloads.mode3(),
            payloads.mode4(),
        ));
        state.set_balance_filter_for_test(balance_filter_with_pitch(AngleRadians::from_radians(
            0.0,
        )));
        let mut config = editable_config_from_state(&state);
        assert!(
            config
                .editor()
                .set_hertz(vescpkg_rs::SampleRate::from_hertz(100.0))
        );
        assert!(
            config
                .editor()
                .set_startup_speed(AngularVelocity::from_degrees_per_second(50.0))
        );
        state.replace_serialized_config_for_test(config);

        assert!(tick_refloat_state_and_handle_packet(
            &mut state,
            lifecycle,
            telemetry.telemetry(),
            imu,
            &[
                REFLOAT_APP_DATA_PACKAGE_ID.get(),
                RefloatAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        // Refloat RUNNING `SAT_CENTERING` uses `startup_speed / hertz` from
        // `third_party/refloat/src/main.c:172` via
        // `get_setpoint_adjustment_step_size` at
        // `third_party/refloat/src/main.c:304-310`; `rate_limitf` applies that
        // step toward target zero at `third_party/refloat/src/utils.c:25-33`,
        // and the main loop publishes the new setpoint at
        // `third_party/refloat/src/main.c:869-875`.
        assert_eq!(base.setpoints().board().angle().as_degrees(), 1.5);
        assert_eq!(
            base.status().ride_state().setpoint_adjustment(),
            RefloatSetpointAdjustment::Centering
        );
    }
}
