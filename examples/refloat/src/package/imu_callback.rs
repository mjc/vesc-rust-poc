#[cfg(any(test, all(not(test), target_arch = "arm")))]
use super::RefloatPackageState;
#[cfg(all(not(test), target_arch = "arm"))]
use super::refloat_state_from_arg;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::ImuReadCallbackBindings;

#[cfg(all(not(test), target_arch = "arm"))]
extern "C" fn refloat_imu_read_callback(acc: *mut f32, gyro: *mut f32, _mag: *mut f32, dt: f32) {
    let Some(accel) = vescpkg_rs::firmware_array(acc.cast_const()) else {
        return;
    };
    let Some(gyro) = vescpkg_rs::firmware_array(gyro.cast_const()) else {
        return;
    };
    let Some(state) = refloat_state_from_arg() else {
        return;
    };
    refloat_imu_callback_with_state(state, accel, gyro, dt);
}

#[cfg(test)]
extern "C" fn refloat_imu_read_callback(_acc: *mut f32, _gyro: *mut f32, _mag: *mut f32, _dt: f32) {
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(super) fn refloat_imu_callback_with_state(
    state: &mut RefloatPackageState,
    accel: [f32; 3],
    gyro: [f32; 3],
    dt: f32,
) {
    // C `imu_ref_callback` ignores mag and feeds gyro/accel/dt into
    // `balance_filter_update` at `third_party/refloat/src/main.c:760-765`.
    state.balance_filter.update(gyro, accel, dt);
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn register_refloat_imu_callback_with<B: ImuReadCallbackBindings>(bindings: &B) -> bool {
    // C registers `imu_ref_callback` between thread startup and app-data
    // registration at `third_party/refloat/src/main.c:2455-2457`.
    bindings.set_imu_read_callback_handler(refloat_imu_read_callback);
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
