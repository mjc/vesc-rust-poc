#[cfg(any(test, all(not(test), target_arch = "arm")))]
use super::RefloatPackageState;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::ImuReadSample;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
struct RefloatImuRead;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RefloatImuReadSample {
    sample: ImuReadSample,
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl RefloatImuReadSample {
    const fn new(sample: ImuReadSample) -> Self {
        Self { sample }
    }

    fn apply_to(self, state: &mut RefloatPackageState) {
        // C map: `imu_ref_callback` ignores mag and feeds gyro/accel/dt into
        // `balance_filter_update` at `third_party/refloat/src/main.c:760-765`.
        state.update_balance_filter(self.sample);
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl vescpkg_rs::ImuReadHandler for RefloatImuRead {
    type State = RefloatPackageState;

    fn state_source() -> vescpkg_rs::PackageStateAccess<'static, Self::State> {
        #[cfg(test)]
        {
            vescpkg_rs::PackageStateAccess::runtime(&super::REFLOAT_RUNTIME_STATE)
        }
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            // C map: Refloat's IMU, app-data, and custom-config callbacks all
            // recover the same `Data *` through `ARG(PROG_ADDR)` at
            // `third_party/refloat/src/main.c:759-764`,
            // `third_party/refloat/src/main.c:2143-2225`, and
            // `third_party/refloat/src/main.c:2243-2288`. Reusing the generated
            // app-data source keeps that identity explicit and supplies the
            // loader fallback when the Rust runtime slot is not installed.
            super::callbacks::refloat_app_data_state_source(&super::REFLOAT_RUNTIME_STATE)
        }
    }

    fn read(state: &mut Self::State, sample: ImuReadSample) {
        // C map: `imu_ref_callback` resolves `Data` through `ARG` and updates
        // its balance filter at `third_party/refloat/src/main.c:759-764`.
        RefloatImuReadSample::new(sample).apply_to(state);
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
vescpkg_rs::firmware_imu_read_callback!(refloat_imu_read_callback, RefloatImuRead);

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(super) fn refloat_imu_callback_with_state(
    state: &mut RefloatPackageState,
    sample: ImuReadSample,
) {
    RefloatImuReadSample::new(sample).apply_to(state);
}

/// Register Refloat's concrete IMU read handler.
///
/// Upstream registers `imu_ref_callback` at `third_party/refloat/src/main.c:2454`; that callback
/// maintains the balance filter used by `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_imu_callback(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.register_imu_read_callback::<RefloatImuRead>()
}

#[cfg(test)]
mod tests {
    use super::refloat_imu_callback_with_state;
    use crate::domain::{RefloatMode, RefloatRunState};
    use crate::package::RefloatPackageState;
    use crate::package::test_support::{
        imu_accel_x, imu_accel_y, imu_accel_z, imu_acceleration, imu_angular_rate, imu_period,
        imu_pitch_rate, imu_read_sample, imu_roll_rate, imu_yaw_rate,
        sample_all_data_payloads_with_ride_state,
    };
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn imu_read_handler_updates_refloat_balance_filter() {
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_startup_done(true);
        let imu = telemetry.imu();
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Normal,
        ));

        <super::RefloatImuRead as vescpkg_rs::ImuReadHandler>::read(
            &mut state,
            imu_read_sample(
                imu_acceleration(
                    imu_accel_x(AccelerationG::from_g(0.0)),
                    imu_accel_y(AccelerationG::from_g(0.0)),
                    imu_accel_z(AccelerationG::from_g(1.0)),
                ),
                imu_angular_rate(
                    imu_roll_rate(AngularVelocity::from_degrees_per_second(0.0)),
                    imu_pitch_rate(AngularVelocity::from_degrees_per_second(1.0)),
                    imu_yaw_rate(AngularVelocity::from_degrees_per_second(0.0)),
                ),
                imu_period(VescSeconds::from_seconds(0.1)),
            ),
        );
        state.refresh_runtime_state(telemetry.telemetry(), imu, 0);

        // C map: `imu_ref_callback` applies each sample to the balance filter at
        // `third_party/refloat/src/main.c:759-764`; the main loop publishes that
        // filter's pitch at `third_party/refloat/src/imu.c:35-41`. SDK tests own
        // callback state-source routing; this test owns Refloat's sample handling.
        assert!(
            state
                .all_data_payloads()
                .base()
                .attitude()
                .balance_pitch()
                .angle()
                .as_radians()
                > 0.0
        );
    }

    #[test]
    fn imu_callback_state_update_feeds_normal_balance_pitch_like_refloat_loop() {
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_startup_done(true);
        let imu = telemetry.imu();
        let mut state = RefloatPackageState::new(sample_all_data_payloads_with_ride_state(
            RefloatRunState::Running,
            RefloatMode::Normal,
        ));

        refloat_imu_callback_with_state(
            &mut state,
            imu_read_sample(
                imu_acceleration(
                    imu_accel_x(AccelerationG::from_g(0.0)),
                    imu_accel_y(AccelerationG::from_g(0.0)),
                    imu_accel_z(AccelerationG::from_g(1.0)),
                ),
                imu_angular_rate(
                    imu_roll_rate(AngularVelocity::from_degrees_per_second(0.0)),
                    imu_pitch_rate(AngularVelocity::from_degrees_per_second(1.0)),
                    imu_yaw_rate(AngularVelocity::from_degrees_per_second(0.0)),
                ),
                imu_period(VescSeconds::from_seconds(0.1)),
            ),
        );
        state.refresh_runtime_state(telemetry.telemetry(), imu, 0);

        // Upstream `imu_ref_callback` updates the balance filter at
        // `third_party/refloat/src/main.c:760-765`; the main loop copies that
        // filter into `imu.balance_pitch` at `third_party/refloat/src/imu.c:35-41`
        // before RUNNING PID reads it at `third_party/refloat/src/pid.c:40`.
        assert!(
            state
                .all_data_payloads()
                .base()
                .attitude()
                .balance_pitch()
                .angle()
                .as_radians()
                > 0.0
        );
    }
}
