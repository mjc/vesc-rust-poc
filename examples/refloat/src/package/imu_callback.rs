#[cfg(any(test, all(not(test), target_arch = "arm")))]
use super::RefloatPackageState;
#[cfg(all(not(test), target_arch = "arm"))]
use super::refloat_state_from_arg;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::{ImuReadCallbackBindings, ImuReadSample};

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
        // C `imu_ref_callback` ignores mag and feeds gyro/accel/dt into
        // `balance_filter_update` at `third_party/refloat/src/main.c:760-765`.
        state.balance_filter.update(self.sample);
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    fn apply_to_firmware_state(self) {
        if let Some(state) = refloat_state_from_arg() {
            self.apply_to(state);
        }
    }

    #[cfg(any(test, not(target_arch = "arm")))]
    const fn ignore_on_host(self) {
        let _ = self;
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
impl vescpkg_rs::ImuReadCallback for RefloatImuRead {
    fn read(sample: ImuReadSample) {
        let sample = RefloatImuReadSample::new(sample);
        #[cfg(all(not(test), target_arch = "arm"))]
        sample.apply_to_firmware_state();
        #[cfg(any(test, not(target_arch = "arm")))]
        sample.ignore_on_host();
    }
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(super) fn refloat_imu_callback_with_state(
    state: &mut RefloatPackageState,
    sample: ImuReadSample,
) {
    RefloatImuReadSample::new(sample).apply_to(state);
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn register_refloat_imu_callback_with<B: ImuReadCallbackBindings>(bindings: &B) -> bool {
    // C registers `imu_ref_callback` between thread startup and app-data
    // registration at `third_party/refloat/src/main.c:2455-2457`.
    bindings.set_imu_read_callback_handler(vescpkg_rs::imu_read_callback::<RefloatImuRead>);
    true
}

/// Register Refloat's IMU read callback.
///
/// Upstream registers `imu_ref_callback` at `third_party/refloat/src/main.c:2455`; that callback
/// maintains the balance filter used by `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_imu_callback(_start: &mut vescpkg_rs::PackageStart) -> bool {
    register_refloat_imu_callback_with(&vescpkg_rs::RealBindings)
}

#[cfg(test)]
mod tests {
    use super::register_refloat_imu_callback_with;
    use crate::package::test_support::RecordingAppDataBindings;

    #[test]
    fn registers_imu_callback_like_refloat_startup() {
        let bindings = RecordingAppDataBindings::accepting();

        assert!(register_refloat_imu_callback_with(&bindings));

        // Refloat registers `imu_ref_callback` during startup at
        // `third_party/refloat/src/main.c:2455`.
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);
        assert_ne!(bindings.last_imu_read_callback.get(), 0);
    }
}
