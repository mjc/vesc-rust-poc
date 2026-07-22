#[cfg(any(test, all(not(test), target_arch = "arm")))]
use super::FloatOutBoyPackageState;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::ImuReadSample;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
struct FloatOutBoyImuRead;

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl vescpkg_rs::ImuReadHandler for FloatOutBoyImuRead {
    type State = FloatOutBoyPackageState;

    fn read(state: &mut Self::State, sample: ImuReadSample) {
        // C map: `imu_ref_callback` resolves `Data` through `ARG` and updates
        // its balance filter at `third_party/float-out-boy/src/main.c:759-764`.
        state.update_balance_filter(sample);
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
vescpkg_rs::firmware_imu_read_callback!(float_out_boy_imu_read_callback, FloatOutBoyImuRead);

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(super) fn float_out_boy_imu_callback_with_state(
    state: &mut FloatOutBoyPackageState,
    sample: ImuReadSample,
) {
    state.update_balance_filter(sample);
}

/// Register Float Out Boy's concrete IMU read handler.
///
/// Upstream registers `imu_ref_callback` at `third_party/float-out-boy/src/main.c:2454`; that callback
/// maintains the balance filter used by `imu_update` at `third_party/float-out-boy/src/imu.c:35-41`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_float_out_boy_imu_callback(
    start: &mut vescpkg_rs::PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.register_imu_read_callback::<FloatOutBoyImuRead>()
}

#[cfg(test)]
mod tests {
    use super::float_out_boy_imu_callback_with_state;
    use crate::domain::{FloatOutBoyMode, FloatOutBoyRunState};
    use crate::package::FloatOutBoyPackageState;
    use crate::package::test_support::{
        imu_accel_x, imu_accel_y, imu_accel_z, imu_acceleration, imu_angular_rate, imu_period,
        imu_pitch_rate, imu_read_sample, imu_roll_rate, imu_yaw_rate,
        sample_all_data_payloads_with_ride_state,
    };
    use vescpkg_rs::prelude::*;
    use vescpkg_rs::test_support::FirmwareTest;

    #[test]
    fn imu_read_handler_updates_float_out_boy_balance_filter() {
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_ready(true);
        let imu = telemetry.imu();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
        ));

        <super::FloatOutBoyImuRead as vescpkg_rs::ImuReadHandler>::read(
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
        state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));

        // C map: `imu_ref_callback` applies each sample to the balance filter at
        // `third_party/float-out-boy/src/main.c:759-764`; the main loop publishes that
        // filter's pitch at `third_party/float-out-boy/src/imu.c:35-41`. SDK tests own
        // callback state-source routing; this test owns Float Out Boy's sample handling.
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
    fn imu_callback_state_update_feeds_normal_balance_pitch_like_float_out_boy_loop() {
        let telemetry = FirmwareTest::new();
        telemetry.set_imu_ready(true);
        let imu = telemetry.imu();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads_with_ride_state(
            FloatOutBoyRunState::Running,
            FloatOutBoyMode::Normal,
        ));

        float_out_boy_imu_callback_with_state(
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
        state.refresh_runtime_state(telemetry.telemetry(), imu, TimestampTicks::from_ticks(0));

        // Upstream `imu_ref_callback` updates the balance filter at
        // `third_party/float-out-boy/src/main.c:760-765`; the main loop copies that
        // filter into `imu.balance_pitch` at `third_party/float-out-boy/src/imu.c:35-41`
        // before RUNNING PID reads it at `third_party/float-out-boy/src/pid.c:40`.
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
