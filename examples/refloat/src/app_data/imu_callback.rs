#[cfg(any(test, all(not(test), target_arch = "arm")))]
use super::RefloatAppDataState;
#[cfg(all(not(test), target_arch = "arm"))]
use super::refloat_state_from_arg;
#[cfg(any(test, all(not(test), target_arch = "arm")))]
use vescpkg_rs::ImuReadCallbackBindings;
#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::ffi;

#[cfg(all(not(test), target_arch = "arm"))]
unsafe extern "C" fn refloat_imu_read_callback(
    acc: *mut f32,
    gyro: *mut f32,
    _mag: *mut f32,
    dt: f32,
) {
    let Some(accel) = refloat_imu_vector(acc) else {
        return;
    };
    let Some(gyro) = refloat_imu_vector(gyro) else {
        return;
    };
    let Some(state) = (unsafe { refloat_state_from_arg() }) else {
        return;
    };
    refloat_imu_callback_with_state(state, accel, gyro, dt);
}

#[cfg(test)]
unsafe extern "C" fn refloat_imu_read_callback(
    _acc: *mut f32,
    _gyro: *mut f32,
    _mag: *mut f32,
    _dt: f32,
) {
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub(super) fn refloat_imu_callback_with_state(
    state: &mut RefloatAppDataState,
    accel: [f32; 3],
    gyro: [f32; 3],
    dt: f32,
) {
    // C `imu_ref_callback` ignores mag and feeds gyro/accel/dt into
    // `balance_filter_update` at `third_party/refloat/src/main.c:760-765`.
    state.balance_filter.update(gyro, accel, dt);
}

#[cfg(all(not(test), target_arch = "arm"))]
fn refloat_imu_vector(values: *mut f32) -> Option<[f32; 3]> {
    if values.is_null() {
        return None;
    }
    let values = unsafe { core::slice::from_raw_parts(values as *const f32, 3) };
    Some([*values.first()?, *values.get(1)?, *values.get(2)?])
}

#[cfg(any(test, all(not(test), target_arch = "arm")))]
fn register_refloat_imu_callback_with<B: ImuReadCallbackBindings>(bindings: &B) -> bool {
    unsafe {
        // C registers `imu_ref_callback` between thread startup and app-data
        // registration at `third_party/refloat/src/main.c:2455-2457`.
        bindings.set_imu_read_callback(refloat_imu_read_callback);
    }
    true
}

/// Register Refloat's IMU read callback.
///
/// Upstream registers `imu_ref_callback` at `third_party/refloat/src/main.c:2455`; that callback
/// maintains the balance filter used by `imu_update` at `third_party/refloat/src/imu.c:35-41`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_imu_callback(_info: *mut ffi::LibInfo) -> bool {
    register_refloat_imu_callback_with(&vescpkg_rs::RealBindings)
}

#[cfg(test)]
mod tests {
    use super::register_refloat_imu_callback_with;
    use crate::app_data::test_support::RecordingAppDataBindings;

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
