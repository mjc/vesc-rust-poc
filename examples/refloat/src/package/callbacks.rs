//! Refloat package callback and loader-state plumbing.
//!
//! C map: package init stores loader ARG/stop handlers and registers app-data
//! callbacks at `third_party/refloat/src/main.c:2419-2461`.

#[cfg(any(test, target_arch = "arm"))]
use super::state::RefloatPackageState;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{Imu, MotorTelemetry};

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn handle_refloat_app_data_packet(
    state: &mut RefloatPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
    send: &mut impl FnMut(&[u8]) -> bool,
    packet: vescpkg_rs::AppDataPacket<'_>,
) -> bool {
    state.handle_packet_with_runtime(telemetry, imu, now, send, packet.as_bytes())
}

#[cfg(all(not(test), target_arch = "arm"))]
pub(crate) struct RefloatAppData;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::StatefulAppDataCallback for RefloatAppData {
    type State = RefloatPackageState;

    fn runtime_state() -> &'static vescpkg_rs::PackageStateStore<Self::State> {
        &super::REFLOAT_RUNTIME_STATE
    }

    fn handle(state: &mut Self::State, packet: vescpkg_rs::AppDataPacket<'_>) {
        // C map: upstream `on_command_received` recovers `Data *` through
        // `ARG(PROG_ADDR)` before app-data dispatch at
        // `third_party/refloat/src/main.c:2143-2225`.
        let firmware = vescpkg_rs::Firmware::new();
        let app_data = firmware.app_data();
        let mut now = || app_data.system_time_ticks();
        let mut send = |bytes: &[u8]| app_data.send(bytes).is_ok();
        let _ = handle_refloat_app_data_packet(
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
    refloat_app_data_callback,
    RefloatAppData,
    refloat_app_data_state_source
);

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::PackageRuntimeState for RefloatPackageState {
    fn runtime_store() -> &'static vescpkg_rs::PackageStateStore<Self> {
        &super::REFLOAT_RUNTIME_STATE
    }

    fn stop(&mut self) {
        // The SDK clears callbacks and joins both package threads before this
        // package-specific cleanup hook runs.
    }
}

#[cfg(test)]
mod tests {
    use super::handle_refloat_app_data_packet;
    use crate::domain::{
        REFLOAT_APP_DATA_PACKAGE_ID, RefloatAppDataCommand, RefloatMode, RefloatRunState,
    };
    use crate::package::RefloatPackageState;
    use crate::package::test_support::{
        sample_all_data_payloads, sample_all_data_payloads_with_ride_state,
    };
    use std::vec::Vec;
    use vescpkg_rs::AppDataPacket;
    use vescpkg_rs::TimestampTicks;
    use vescpkg_rs::test_support::FirmwareTest;

    fn handle_packet(
        state: &mut RefloatPackageState,
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
        handle_refloat_app_data_packet(state, telemetry, imu, &mut now, &mut send, packet)
    }

    #[test]
    fn handler_rejects_empty_and_sends_valid_packets() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let mut state = RefloatPackageState::new(sample_all_data_payloads());

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
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::GetAllData.id(),
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
        assert_eq!(sent.len(), 1);
        assert_eq!(&sent[0][..3], &request);
    }

    #[test]
    fn app_data_callback_dispatches_without_main_loop_refresh_like_refloat() {
        let app_data = TimestampTicks::from_ticks(0);
        let mut sent = Vec::new();
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_startup_done(true);
        let imu = telemetry.imu();
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Ready,
            RefloatMode::Normal,
        ));

        let request = [
            REFLOAT_APP_DATA_PACKAGE_ID.get(),
            RefloatAppDataCommand::RealtimeData.id(),
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
        // `third_party/refloat/src/main.c:2143-2225`; READY engage and
        // IMU/motor refresh stay in `refloat_thd` at `third_party/refloat/src/main.c:772-1080`.
        assert_eq!(
            state
                .all_data_payloads()
                .base()
                .status()
                .ride_state()
                .run_state(),
            RefloatRunState::Ready
        );
    }
}
