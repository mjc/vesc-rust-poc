use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::config::FloatOutBoyConfigImage;
use crate::domain::{
    FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
    FloatOutBoyAppDataCommand, FloatOutBoyMode, FloatOutBoyRideState, FloatOutBoyRunState,
};

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

    const fn mode(self) -> FloatOutBoyMode {
        // C map: HANDTEST toggles between NORMAL and HANDTEST in
        // `third_party/float-out-boy/src/main.c:1430`.
        match self {
            Self::Enable => FloatOutBoyMode::HandTest,
            Self::Disable => FloatOutBoyMode::Normal,
        }
    }

    fn apply_to(self, state: &mut FloatOutBoyPackageState) {
        // C map: `cmd_handtest` only applies when the board is READY and mode
        // is NORMAL or HANDTEST at `third_party/float-out-boy/src/main.c:1426-1430`.
        let ride_state = state.all_data_payloads.base().status().ride_state();
        if let Some(mode) = self.mode_for(ride_state) {
            state.set_ride_mode(mode);
            state.apply_handtest_config(self);
        }
    }

    fn mode_for(self, ride_state: FloatOutBoyRideState) -> Option<FloatOutBoyMode> {
        // C map: `cmd_handtest` refuses changes unless READY and the current
        // mode is NORMAL or HANDTEST at `third_party/float-out-boy/src/main.c:1426-1430`.
        match (ride_state.run_state(), ride_state.mode()) {
            (FloatOutBoyRunState::Ready, FloatOutBoyMode::Normal | FloatOutBoyMode::HandTest) => {
                Some(self.mode())
            }
            _ => None,
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
    pub(super) fn handle_handtest_packet(&mut self, bytes: &[u8]) -> bool {
        // QML sends `[101, COMMAND_HANDTEST, on]` from `ui.qml.in:764-768`;
        // Float Out Boy C dispatches it at `third_party/float-out-boy/src/main.c:2226-2228`
        // and applies READY/NORMAL/HANDTEST gates at `third_party/float-out-boy/src/main.c:1421-1430`.
        match FloatOutBoyHandtestRequest::from_packet(bytes) {
            Some(request) => {
                request.apply_to(self);
                true
            }
            None => false,
        }
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

    fn apply_handtest_config(&mut self, request: FloatOutBoyHandtestRequest) {
        // C map: `cmd_handtest` branches to enable or restore behavior based
        // on the packet byte at `third_party/float-out-boy/src/main.c:1431-1449`.
        match request {
            FloatOutBoyHandtestRequest::Enable => self.apply_handtest_safety_config(),
            FloatOutBoyHandtestRequest::Disable => self.restore_handtest_config(),
        }
    }

    fn restore_handtest_config(&mut self) {
        // C map: disabling HANDTEST restores the prior config from EEPROM in
        // `third_party/float-out-boy/src/main.c:1447-1449`.
        self.read_config_from_eeprom();
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
    }

    fn apply_handtest_safety_config(&mut self) {
        // C map: enabling HANDTEST applies temporary safety overrides at
        // `third_party/float-out-boy/src/main.c:1431-1446`.
        if let Some(config) = Self::handtest_safety_config(&self.serialized_config) {
            self.serialized_config = config;
        }
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
mod tests {
    use super::*;
    use crate::domain::{
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataAttitude,
        FloatOutBoyAllDataBasePayload, FloatOutBoyAllDataPayloads, FloatOutBoyAllDataStatus,
        FloatOutBoyAppDataCommand, FloatOutBoyMode, FloatOutBoyRealtimeBalanceCurrent,
        FloatOutBoyRealtimeBalancePitch, FloatOutBoyRealtimeBoosterCurrent,
        FloatOutBoyRealtimeRuntimeSetpoint, FloatOutBoyRealtimeRuntimeSetpoints,
        FloatOutBoyRideState, FloatOutBoyRunState, FloatOutBoySetpointAdjustment,
        FloatOutBoyStopCondition,
    };
    use crate::package::test_support::{
        balance_filter_with_pitch, editable_config_from_bytes, editable_config_from_state,
        sample_all_data_payloads_with_ride_state, tick_float_out_boy_state_and_handle_packet,
    };
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FirmwareTest;

    fn ride_state(run_state: FloatOutBoyRunState, mode: FloatOutBoyMode) -> FloatOutBoyRideState {
        FloatOutBoyRideState::new(
            run_state,
            mode,
            FloatOutBoySetpointAdjustment::None,
            FloatOutBoyStopCondition::None,
        )
    }

    #[test]
    fn handtest_request_selects_mode_only_while_ready_like_float_out_boy() {
        assert_eq!(
            FloatOutBoyHandtestRequest::from_flag(0).mode(),
            FloatOutBoyMode::Normal
        );
        assert_eq!(
            FloatOutBoyHandtestRequest::from_flag(1).mode(),
            FloatOutBoyMode::HandTest
        );
        assert_eq!(
            FloatOutBoyHandtestRequest::Enable.mode_for(ride_state(
                FloatOutBoyRunState::Ready,
                FloatOutBoyMode::Normal,
            )),
            Some(FloatOutBoyMode::HandTest),
        );
        assert_eq!(
            FloatOutBoyHandtestRequest::Enable.mode_for(ride_state(
                FloatOutBoyRunState::Running,
                FloatOutBoyMode::Normal,
            )),
            None,
        );
        assert_eq!(
            FloatOutBoyHandtestRequest::Enable.mode_for(ride_state(
                FloatOutBoyRunState::Ready,
                FloatOutBoyMode::Flywheel,
            )),
            None,
        );
    }

    #[test]
    fn handtest_packet_toggles_ready_mode_and_safety_config_like_float_out_boy_qml() {
        let _firmware = FirmwareTest::new();
        // QML sends COMMAND_HANDTEST at `float-out-boy/ui.qml.in:764-768`; C toggles
        // mode and temporary safety config at `third_party/float-out-boy/src/main.c:1421-1450`.
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ));
        let original_config = *state.serialized_config();

        assert!(state.handle_handtest_packet(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::HandTest.id(),
            1,
        ]));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            FloatOutBoyMode::HandTest
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
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::HandTest.id(),
            0,
        ]));
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .mode(),
            FloatOutBoyMode::Normal
        );
        assert_eq!(state.serialized_config(), &original_config);
    }

    #[test]
    fn handtest_disable_restores_eeprom_not_the_enable_time_image_like_float_out_boy() {
        let _firmware = FirmwareTest::new();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ));
        let mut persisted = editable_config_from_state(&state);
        assert!(persisted.editor().set_kp(AngleCurrentGain::new(1.2)));
        let persisted = *persisted.as_bytes();
        assert!(state.store_serialized_config(&persisted));

        let mut volatile = editable_config_from_state(&state);
        assert!(volatile.editor().set_kp(AngleCurrentGain::new(-9.0)));
        state.replace_serialized_config_for_test(&volatile);

        assert!(state.handle_handtest_packet(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::HandTest.id(),
            1,
        ]));
        assert!(state.handle_handtest_packet(&[
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::HandTest.id(),
            0,
        ]));

        assert_f32_eq!(
            state.serialized_config.balance().kp().as_amps_per_degree(),
            1.2
        );
    }

    #[test]
    fn app_data_handtest_running_recenters_start_setpoint_like_float_out_boy_loop() {
        let lifecycle = TimestampTicks::from_ticks(0);
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_ready(true);
        let imu = telemetry.imu();
        let payloads = sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::HandTest,
        );
        let base = payloads.base();
        let ride_state = FloatOutBoyRideState::new(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::HandTest,
            FloatOutBoySetpointAdjustment::Centering,
            FloatOutBoyStopCondition::None,
        );
        let board = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(2.0));
        let zero = FloatOutBoyRealtimeRuntimeSetpoint::new(AngleDegrees::from_degrees(0.0));
        let setpoints =
            FloatOutBoyRealtimeRuntimeSetpoints::new(board, zero, zero, zero, zero, zero);
        let base = FloatOutBoyAllDataBasePayload::new(
            FloatOutBoyRealtimeBalanceCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            FloatOutBoyAllDataAttitude::new(
                FloatOutBoyRealtimeBalancePitch::new(AngleRadians::from_radians(0.0)),
                base.attitude().roll(),
                base.attitude().pitch(),
            ),
            FloatOutBoyAllDataStatus::new(ride_state, base.status().beep_reason()),
            base.footpad(),
            setpoints,
            FloatOutBoyRealtimeBoosterCurrent::new(MotorCurrent::new(Current::from_amps(0.0))),
            base.motor(),
        );
        let mut state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::new(
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
        state.replace_serialized_config_for_test(&config);

        assert!(tick_float_out_boy_state_and_handle_packet(
            &mut state,
            lifecycle,
            telemetry.telemetry(),
            imu,
            &[
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
                FloatOutBoyAppDataCommand::RealtimeData.id(),
            ],
        ));

        let base = state.all_data_payloads().base();
        // Float Out Boy RUNNING `SAT_CENTERING` uses `startup_speed / hertz` from
        // `third_party/float-out-boy/src/main.c:172` via
        // `get_setpoint_adjustment_step_size` at
        // `third_party/float-out-boy/src/main.c:304-310`; `rate_limitf` applies that
        // step toward target zero at `third_party/float-out-boy/src/utils.c:25-33`,
        // and the main loop publishes the new setpoint at
        // `third_party/float-out-boy/src/main.c:869-875`.
        assert_f32_eq!(base.setpoints().board().angle().as_degrees(), 1.5);
        assert_eq!(
            base.status().ride_state().setpoint_adjustment(),
            FloatOutBoySetpointAdjustment::Centering
        );
    }
}
