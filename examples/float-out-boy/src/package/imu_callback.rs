use super::FloatOutBoyPackageState;
use vescpkg_rs::ImuReadSample;

struct FloatOutBoyImuRead;

impl vescpkg_rs::ImuReadHandler for FloatOutBoyImuRead {
    type State = FloatOutBoyPackageState;

    fn read(state: &mut Self::State, sample: ImuReadSample) {
        // C map: `imu_ref_callback` resolves `Data` through `ARG` and updates
        // its balance filter at `third_party/float-out-boy/src/main.c:759-764`.
        state.update_balance_filter(sample);
    }
}

vescpkg_rs::firmware_imu_read_callback!(float_out_boy_imu_read_callback, FloatOutBoyImuRead);

#[cfg(test)]
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
mod tests;
