//! Float Out Boy package callback and loader-state plumbing.
//!
//! C map: package init stores loader ARG/stop handlers and registers app-data
//! callbacks at `third_party/float-out-boy/src/main.c:2419-2461`.

#[cfg(any(test, target_arch = "arm"))]
use super::state::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{Imu, MotorTelemetry};

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn handle_float_out_boy_app_data_packet(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
    send: &mut impl FnMut(&[u8]) -> bool,
    packet: vescpkg_rs::AppDataPacket<'_>,
) -> bool {
    state.handle_packet_with_runtime(telemetry, imu, now, send, packet.as_bytes())
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) struct FloatOutBoyAppData;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::AppDataHandler for FloatOutBoyAppData {
    type State = FloatOutBoyPackageState;

    fn handle(state: &mut Self::State, packet: vescpkg_rs::AppDataPacket<'_>) {
        // C map: upstream `on_command_received` recovers `Data *` through
        // `ARG(PROG_ADDR)` before app-data dispatch at
        // `third_party/float-out-boy/src/main.c:2143-2225`.
        let firmware = vescpkg_rs::Firmware::new();
        let mut now = || firmware.clock().now();
        let mut send = |bytes: &[u8]| firmware.app_data().send(bytes).is_ok();
        let _ = handle_float_out_boy_app_data_packet(
            state,
            firmware.telemetry(),
            firmware.imu(),
            &mut now,
            &mut send,
            packet,
        );
    }
}

vescpkg_rs::firmware_stateful_app_data_callback!(
    float_out_boy_app_data_callback,
    FloatOutBoyAppData
);

#[cfg(test)]
mod tests {
    use super::handle_float_out_boy_app_data_packet;
    use crate::domain::{
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAllDataMode, FloatOutBoyAppDataCommand,
        FloatOutBoyMode, FloatOutBoyRunState,
    };
    use crate::package::FloatOutBoyPackageState;
    use crate::package::protocol::encode_float_out_boy_get_realtime_data_response;
    use crate::package::test_support::{
        sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
    };
    use std::vec::Vec;
    use vescpkg_rs::AppDataPacket;
    use vescpkg_rs::TimestampTicks;
    use vescpkg_rs::test_support::FirmwareTest;

    fn handle_packet(
        state: &mut FloatOutBoyPackageState,
        now: TimestampTicks,
        sent: &mut Vec<Vec<u8>>,
        telemetry: &impl vescpkg_rs::MotorTelemetry,
        imu: &impl vescpkg_rs::Imu,
        packet: AppDataPacket<'_>,
    ) -> bool {
        let mut now = || now;
        let mut send = |bytes: &[u8]| {
            sent.push(Vec::from(bytes));
            true
        };
        handle_float_out_boy_app_data_packet(state, telemetry, imu, &mut now, &mut send, packet)
    }

    #[test]
    fn handler_rejects_empty_and_sends_valid_packets() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

        let telemetry = FirmwareTest::new();
        let imu = telemetry.imu();
        let empty_packet = AppDataPacket::from_bytes(&[]);
        assert!(!handle_packet(
            &mut state,
            app_data,
            &mut sent,
            telemetry.telemetry(),
            imu,
            empty_packet,
        ));
        assert!(sent.is_empty());

        let request = [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::GetAllData.id(),
            FloatOutBoyAllDataMode::base().source_id(),
        ];
        let packet = AppDataPacket::from_bytes(&request);
        assert!(handle_packet(
            &mut state,
            app_data,
            &mut sent,
            telemetry.telemetry(),
            imu,
            packet,
        ));
        assert_eq!(sent.len(), 1);
        assert_eq!(&sent[0][..3], &request);
    }

    #[test]
    fn app_data_callback_dispatches_legacy_realtime_data_like_float_out_boy() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let telemetry = FirmwareTest::new();
        let imu = telemetry.imu();
        let payloads = sample_all_data_payloads();
        let expected = encode_float_out_boy_get_realtime_data_response(&payloads);
        let mut state = FloatOutBoyPackageState::new(payloads);
        let request = [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::GetRealtimeData.id(),
        ];

        assert!(handle_packet(
            &mut state,
            app_data,
            &mut sent,
            telemetry.telemetry(),
            imu,
            AppDataPacket::from_bytes(&request),
        ));
        assert_eq!(sent.as_slice(), [expected.as_slice()]);
    }

    #[test]
    fn app_data_callback_rejects_malformed_legacy_realtime_data_requests() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let telemetry = FirmwareTest::new();
        let imu = telemetry.imu();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

        for request in [
            &[][..],
            &[FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get()][..],
            &[100, 1][..],
        ] {
            assert!(!handle_packet(
                &mut state,
                app_data,
                &mut sent,
                telemetry.telemetry(),
                imu,
                AppDataPacket::from_bytes(request),
            ));
        }
        assert!(sent.is_empty());
    }

    #[test]
    fn app_data_callback_dispatches_without_main_loop_refresh_like_float_out_boy() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_ready(true);
        let imu = telemetry.imu();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Ready,
            FloatOutBoyMode::Normal,
        ));

        let request = [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            FloatOutBoyAppDataCommand::RealtimeData.id(),
            0,
        ];
        let packet = AppDataPacket::from_bytes(&request);
        assert!(handle_packet(
            &mut state,
            app_data,
            &mut sent,
            telemetry.telemetry(),
            imu,
            packet,
        ));

        // Upstream `on_command_received` only dispatches app commands at
        // `third_party/float-out-boy/src/main.c:2143-2225`; READY engage and
        // IMU/motor refresh stay in `float_out_boy_thd` at `third_party/float-out-boy/src/main.c:772-1080`.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            FloatOutBoyRunState::Ready
        );
    }
}
