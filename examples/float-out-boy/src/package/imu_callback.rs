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
        let firmware = vescpkg_rs::Firmware::new();
        state.handle_imu_control_sample(
            sample,
            firmware.imu(),
            firmware.motor(),
            firmware.clock().now(),
        );
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
vescpkg_rs::firmware_imu_read_callback!(float_out_boy_imu_read_callback, FloatOutBoyImuRead);

#[cfg(test)]
pub(super) fn float_out_boy_imu_callback_with_state(
    state: &mut FloatOutBoyPackageState,
    sample: ImuReadSample,
) {
    <FloatOutBoyImuRead as vescpkg_rs::ImuReadHandler>::read(state, sample);
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
mod tests;
