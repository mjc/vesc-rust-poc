//! IMU helpers built on firmware IMU table slots.

use crate::types::{ImuAngularRate, ImuPitch, ImuRoll, ImuYaw};
#[cfg(not(test))]
use crate::units::{AngleRadians, AngularVelocity};

/// IMU operations backed by firmware slots.
pub trait ImuBindings {
    /// Return whether firmware IMU startup has completed.
    ///
    /// Refloat v1.2.1 gates `STATE_STARTUP` -> `STATE_READY` on this at
    /// `src/main.c:833-838`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:510`.
    fn startup_done(&self) -> bool;

    /// Return firmware IMU roll.
    ///
    /// Refloat v1.2.1 reads roll from `VESC_IF->imu_get_roll()` at
    /// `src/imu.c:35-38`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:511`.
    fn roll(&self) -> ImuRoll;

    /// Return firmware IMU pitch.
    ///
    /// Refloat v1.2.1 reads pitch from `VESC_IF->imu_get_pitch()` at
    /// `src/imu.c:37-38`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:512`.
    fn pitch(&self) -> ImuPitch;

    /// Return firmware IMU yaw.
    ///
    /// Refloat v1.2.1 reads yaw from `VESC_IF->imu_get_yaw()` at
    /// `src/imu.c:39-40`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:513`.
    fn yaw(&self) -> ImuYaw;

    /// Return firmware IMU gyro axes in degrees/sec.
    ///
    /// Refloat v1.2.1 reads gyro from `VESC_IF->imu_get_gyro(...)` at
    /// `src/imu.c:45-53`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:516`.
    fn angular_rate(&self) -> ImuAngularRate;

    /// Return firmware IMU quaternions.
    fn quaternions(&self) -> [f32; 4];
}

/// Rust implementation for a firmware IMU read callback.
pub trait ImuReadCallback {
    /// Handle one IMU read callback with copied accelerometer and gyro axes.
    fn read(accel: [f32; 3], gyro: [f32; 3], dt: f32);
}

/// Firmware ABI trampoline for a typed IMU read callback.
///
/// # Safety
///
/// `acc` and `gyro` must each point to three readable `f32` values for the duration of this call.
pub unsafe extern "C" fn imu_read_callback<T: ImuReadCallback>(
    acc: *mut f32,
    gyro: *mut f32,
    _mag: *mut f32,
    dt: f32,
) {
    let Some(accel) = crate::firmware_array(acc.cast_const()) else {
        return;
    };
    let Some(gyro) = crate::firmware_array(gyro.cast_const()) else {
        return;
    };
    T::read(accel, gyro, dt);
}

#[cfg(not(test))]
/// IMU binding implementation that forwards to the live firmware ABI.
pub struct RealImuBindings;

#[cfg(not(test))]
impl ImuBindings for RealImuBindings {
    fn startup_done(&self) -> bool {
        unsafe { vescpkg_rs_sys::raw::imu_startup_done() }
    }

    fn roll(&self) -> ImuRoll {
        ImuRoll::new(AngleRadians::from_radians(unsafe {
            vescpkg_rs_sys::raw::imu_get_roll()
        }))
    }

    fn pitch(&self) -> ImuPitch {
        ImuPitch::new(AngleRadians::from_radians(unsafe {
            vescpkg_rs_sys::raw::imu_get_pitch()
        }))
    }

    fn yaw(&self) -> ImuYaw {
        ImuYaw::new(AngleRadians::from_radians(unsafe {
            vescpkg_rs_sys::raw::imu_get_yaw()
        }))
    }

    fn angular_rate(&self) -> ImuAngularRate {
        let mut gyro = [0.0; 3];
        unsafe { vescpkg_rs_sys::raw::imu_get_gyro(gyro.as_mut_ptr()) };
        ImuAngularRate::new([
            AngularVelocity::from_degrees_per_second(gyro[0]),
            AngularVelocity::from_degrees_per_second(gyro[1]),
            AngularVelocity::from_degrees_per_second(gyro[2]),
        ])
    }

    fn quaternions(&self) -> [f32; 4] {
        let mut quaternions = [0.0; 4];
        unsafe { vescpkg_rs_sys::raw::vesc_imu_get_quaternions(quaternions.as_mut_ptr()) };
        quaternions
    }
}

/// High-level IMU API built on a binding implementation.
pub struct ImuApi<B> {
    bindings: B,
}

impl<B: ImuBindings> ImuApi<B> {
    /// Construct a new IMU API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return the wrapped IMU bindings.
    pub fn bindings(&self) -> &B {
        &self.bindings
    }

    /// Return whether firmware IMU startup has completed.
    pub fn startup_done(&self) -> bool {
        self.bindings.startup_done()
    }

    /// Return firmware IMU roll.
    pub fn roll(&self) -> ImuRoll {
        self.bindings.roll()
    }

    /// Return firmware IMU pitch.
    pub fn pitch(&self) -> ImuPitch {
        self.bindings.pitch()
    }

    /// Return firmware IMU yaw.
    pub fn yaw(&self) -> ImuYaw {
        self.bindings.yaw()
    }

    /// Return firmware IMU gyro axes in degrees/sec.
    pub fn angular_rate(&self) -> ImuAngularRate {
        self.bindings.angular_rate()
    }

    /// Return firmware IMU quaternions.
    pub fn quaternions(&self) -> [f32; 4] {
        self.bindings.quaternions()
    }
}

#[cfg(any(test, feature = "test-support"))]
/// IMU fake binding helpers exported for tests.
pub mod test_support {
    use super::ImuBindings;
    use crate::types::{ImuAngularRate, ImuPitch, ImuRoll, ImuYaw};
    use crate::units::{AngleRadians, AngularVelocity};
    use core::cell::Cell;

    /// Fake IMU binding implementation used by package tests.
    pub struct FakeImuBindings {
        /// Number of IMU-startup checks observed.
        pub startup_done_calls: Cell<usize>,
        /// Number of roll reads observed.
        pub roll_calls: Cell<usize>,
        /// Number of pitch reads observed.
        pub pitch_calls: Cell<usize>,
        /// Number of yaw reads observed.
        pub yaw_calls: Cell<usize>,
        /// Number of angular-rate reads observed.
        pub angular_rate_calls: Cell<usize>,
        /// Number of quaternion reads observed.
        pub quaternion_calls: Cell<usize>,
        startup_done: Cell<bool>,
        roll: Cell<ImuRoll>,
        pitch: Cell<ImuPitch>,
        yaw: Cell<ImuYaw>,
        angular_rate: Cell<ImuAngularRate>,
        quaternions: Cell<[f32; 4]>,
    }

    impl Default for FakeImuBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeImuBindings {
        /// Creates fake IMU bindings returning startup-not-done and zero attitude.
        pub fn new() -> Self {
            let zero = AngleRadians::from_radians(0.0);
            Self {
                startup_done_calls: Cell::new(0),
                roll_calls: Cell::new(0),
                pitch_calls: Cell::new(0),
                yaw_calls: Cell::new(0),
                angular_rate_calls: Cell::new(0),
                quaternion_calls: Cell::new(0),
                startup_done: Cell::new(false),
                roll: Cell::new(ImuRoll::new(zero)),
                pitch: Cell::new(ImuPitch::new(zero)),
                yaw: Cell::new(ImuYaw::new(zero)),
                angular_rate: Cell::new(ImuAngularRate::new([
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                    AngularVelocity::from_degrees_per_second(0.0),
                ])),
                quaternions: Cell::new([1.0, 0.0, 0.0, 0.0]),
            }
        }

        /// Return fake IMU bindings with the supplied startup state.
        pub fn with_startup_done(self, startup_done: bool) -> Self {
            self.startup_done.set(startup_done);
            self
        }

        /// Return fake IMU bindings with the supplied roll/pitch/yaw.
        pub fn with_attitude(self, roll: ImuRoll, pitch: ImuPitch, yaw: ImuYaw) -> Self {
            self.roll.set(roll);
            self.pitch.set(pitch);
            self.yaw.set(yaw);
            self
        }

        /// Return fake IMU bindings with the supplied angular-rate axes.
        pub fn with_angular_rate(self, angular_rate: ImuAngularRate) -> Self {
            self.angular_rate.set(angular_rate);
            self
        }

        /// Return fake IMU bindings with the supplied quaternion state.
        pub fn with_quaternions(self, quaternions: [f32; 4]) -> Self {
            self.quaternions.set(quaternions);
            self
        }
    }

    impl ImuBindings for FakeImuBindings {
        fn startup_done(&self) -> bool {
            self.startup_done_calls
                .set(self.startup_done_calls.get() + 1);
            self.startup_done.get()
        }

        fn roll(&self) -> ImuRoll {
            self.roll_calls.set(self.roll_calls.get() + 1);
            self.roll.get()
        }

        fn pitch(&self) -> ImuPitch {
            self.pitch_calls.set(self.pitch_calls.get() + 1);
            self.pitch.get()
        }

        fn yaw(&self) -> ImuYaw {
            self.yaw_calls.set(self.yaw_calls.get() + 1);
            self.yaw.get()
        }

        fn angular_rate(&self) -> ImuAngularRate {
            self.angular_rate_calls
                .set(self.angular_rate_calls.get() + 1);
            self.angular_rate.get()
        }

        fn quaternions(&self) -> [f32; 4] {
            self.quaternion_calls.set(self.quaternion_calls.get() + 1);
            self.quaternions.get()
        }
    }
}
