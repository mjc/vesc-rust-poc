//! IMU helpers built on firmware IMU table slots.

#[cfg(not(test))]
use crate::types::ImuQuaternion;
use crate::types::{
    ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRate,
    ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw, ImuAttitude, ImuMagneticField,
    ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuOrientation, ImuPitch,
    ImuReadSample, ImuRoll, ImuSamplePeriod, ImuYaw,
};
#[cfg(not(test))]
use crate::units::AngleRadians;
use crate::units::{AccelerationG, AngularVelocity, MagneticFluxDensity, VescSeconds};

/// IMU operations backed by firmware slots.
#[cfg(not(test))]
pub trait ImuBindings {
    /// Return whether firmware IMU startup has completed.
    ///
    /// Float Out Boy v1.2.1 gates `STATE_STARTUP` -> `STATE_READY` on this at
    /// `src/main.c:833-838`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:510`.
    fn is_ready(&self) -> bool;

    /// Return firmware IMU roll.
    ///
    /// Float Out Boy v1.2.1 reads roll from `VESC_IF->imu_get_roll()` at
    /// `src/imu.c:35-38`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:511`.
    fn roll(&self) -> ImuRoll;

    /// Return firmware IMU pitch.
    ///
    /// Float Out Boy v1.2.1 reads pitch from `VESC_IF->imu_get_pitch()` at
    /// `src/imu.c:37-38`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:512`.
    fn pitch(&self) -> ImuPitch;

    /// Return firmware IMU yaw.
    ///
    /// Float Out Boy v1.2.1 reads yaw from `VESC_IF->imu_get_yaw()` at
    /// `src/imu.c:39-40`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:513`.
    fn yaw(&self) -> ImuYaw;

    /// Return firmware IMU attitude.
    fn attitude(&self) -> ImuAttitude {
        ImuAttitude::new(self.roll(), self.pitch(), self.yaw())
    }

    /// Return firmware IMU gyro axes in degrees/sec.
    ///
    /// Float Out Boy v1.2.1 reads gyro from `VESC_IF->imu_get_gyro(...)` at
    /// `src/imu.c:45-53`; the VESC ABI slot is declared at
    /// `vesc_pkg_lib/vesc_c_if.h:516`.
    fn angular_rate(&self) -> ImuAngularRate;

    /// Return firmware IMU orientation.
    fn orientation(&self) -> ImuOrientation;
    /// Return firmware acceleration axes in g units.
    fn acceleration(&self) -> ImuAcceleration;
    /// Return firmware magnetic-field axes in microteslas.
    fn magnetic_field(&self) -> ImuMagneticField;
    /// Return derotated firmware acceleration axes in g units.
    fn derotated_acceleration(&self) -> ImuAcceleration;
    /// Derotate a caller-provided acceleration vector into the firmware frame.
    fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration;
    /// Return derotated firmware gyro axes in degrees/sec.
    fn derotated_angular_rate(&self) -> ImuAngularRate;
}

#[cfg(not(test))]
impl<B: ImuBindings + ?Sized> ImuBindings for &B {
    fn is_ready(&self) -> bool {
        (**self).is_ready()
    }

    fn roll(&self) -> ImuRoll {
        (**self).roll()
    }

    fn pitch(&self) -> ImuPitch {
        (**self).pitch()
    }

    fn yaw(&self) -> ImuYaw {
        (**self).yaw()
    }

    fn angular_rate(&self) -> ImuAngularRate {
        (**self).angular_rate()
    }

    fn orientation(&self) -> ImuOrientation {
        (**self).orientation()
    }

    fn acceleration(&self) -> ImuAcceleration {
        (**self).acceleration()
    }

    fn magnetic_field(&self) -> ImuMagneticField {
        (**self).magnetic_field()
    }

    fn derotated_acceleration(&self) -> ImuAcceleration {
        (**self).derotated_acceleration()
    }

    fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration {
        (**self).derotate_acceleration(input)
    }

    fn derotated_angular_rate(&self) -> ImuAngularRate {
        (**self).derotated_angular_rate()
    }
}

/// Rust implementation for a firmware IMU read callback.
pub trait ImuReadCallback {
    /// Handle one typed hardware IMU read sample copied from the firmware callback.
    fn read(sample: ImuReadSample);
}

/// Typed IMU sample handler for package-owned state.
///
/// VESC package callbacks recover package state through `ARG(PROG_ADDR)` at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:697-700`; the generated callback
/// shares the package's runtime state and package code only handles typed samples.
pub trait ImuReadHandler {
    /// Package state installed during startup.
    type State: crate::PackageRuntimeState;

    /// Handle one typed hardware IMU sample.
    fn read(state: &mut Self::State, sample: ImuReadSample);
}

/// Concrete package-local IMU callback generated for a typed handler.
#[doc(hidden)]
pub unsafe trait PackageImuReadCallback: ImuReadHandler {
    /// Return state access rooted in this package's loader identity.
    #[doc(hidden)]
    #[cfg(target_arch = "arm")]
    fn state_source() -> crate::PackageStateAccess<'static, Self::State>;

    /// Return host-test state access rooted in the package runtime store.
    #[doc(hidden)]
    #[cfg(not(target_arch = "arm"))]
    fn state_source() -> crate::PackageStateAccess<'static, Self::State> {
        crate::PackageStateAccess::runtime(
            <Self::State as crate::PackageRuntimeState>::runtime_store(),
        )
    }

    /// Return the callback's package-local function address.
    #[doc(hidden)]
    fn image_address() -> usize;
}

/// Define a concrete package-local firmware callback for a typed IMU handler.
#[macro_export]
macro_rules! firmware_imu_read_callback {
    ($name:ident, $handler:ty) => {
        #[doc = "Package-local firmware IMU callback generated by `firmware_imu_read_callback!`."]
        #[cfg_attr(target_arch = "arm", unsafe(no_mangle))]
        #[inline(never)]
        pub unsafe extern "C" fn $name(acc: *mut f32, gyro: *mut f32, mag: *mut f32, dt: f32) {
            unsafe { $crate::__macro_support::imu_read_callback::<$handler>(acc, gyro, mag, dt) }
        }

        $crate::__vescpkg_image_offset!($name);

        unsafe impl $crate::__macro_support::PackageImuReadCallback for $handler {
            #[cfg(target_arch = "arm")]
            #[inline(always)]
            fn state_source() -> $crate::PackageStateAccess<'static, Self::State> {
                unsafe fn package_state_ptr()
                -> Option<core::ptr::NonNull<<$handler as $crate::ImuReadHandler>::State>> {
                    unsafe {
                        $crate::__macro_support::__firmware_package_state_ptr::<
                            <$handler as $crate::ImuReadHandler>::State,
                        >($crate::firmware_package_program_address!($name))
                    }
                }

                unsafe {
                    $crate::__macro_support::__package_state_access(
                        <<$handler as $crate::ImuReadHandler>::State as $crate::PackageRuntimeState>::runtime_store(),
                        package_state_ptr,
                    )
                }
            }

            #[inline(always)]
            fn image_address() -> usize {
                $name as *const () as usize
            }
        }
    };
}
#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
#[inline(always)]
fn dispatch_imu_read<T: ImuReadHandler>(
    state_source: crate::PackageStateAccess<'static, T::State>,
    sample: ImuReadSample,
) {
    // C map: Float Out Boy's `imu_ref_callback` resolves `Data *` through `ARG` and
    // passes the sample to `balance_filter_update` at
    // `third_party/float-out-boy/src/main.c:759-764`. This must use the same state
    // source as app-data and custom-config: resolving only the Rust runtime
    // slot dropped IMU samples when that slot was absent, leaving raw VESC
    // pitch live while Float Out Boy's balance pitch stayed frozen.
    let _ = state_source.with_mut(|state| T::read(state, sample));
}

impl<T> ImuReadCallback for T
where
    T: PackageImuReadCallback,
{
    #[inline(always)]
    fn read(sample: ImuReadSample) {
        #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
        dispatch_imu_read::<T>(<T as PackageImuReadCallback>::state_source(), sample);
        #[cfg(all(not(test), not(feature = "test-support"), not(target_arch = "arm")))]
        let _ = sample;
    }
}

/// Firmware ABI trampoline for a typed IMU read callback.
///
/// # Safety
///
/// `acc`, `gyro`, and `mag` must each point to three readable `f32` values for the duration of
/// this call.
#[doc(hidden)]
pub unsafe extern "C" fn imu_read_callback<T: ImuReadCallback>(
    acc: *mut f32,
    gyro: *mut f32,
    mag: *mut f32,
    dt: f32,
) {
    let Some(accel) = (unsafe { crate::firmware_array::<f32, 3>(acc.cast_const()) }) else {
        return;
    };
    let Some(gyro) = (unsafe { crate::firmware_array::<f32, 3>(gyro.cast_const()) }) else {
        return;
    };
    let Some(mag) = (unsafe { crate::firmware_array::<f32, 3>(mag.cast_const()) }) else {
        return;
    };
    // BLDC calls package IMU read callbacks with gyro axes already converted
    // from `m_gyro` degrees/sec to radians/sec at `imu/imu.c:704-727`.
    T::read(ImuReadSample::from_parts(
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(accel[0])),
            ImuAccelerationY::new(AccelerationG::from_g(accel[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(accel[2])),
        ),
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(gyro[0])),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(gyro[1])),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(gyro[2])),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(mag[0])),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(mag[1])),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(mag[2])),
        ),
        ImuSamplePeriod::new(VescSeconds::from_seconds(dt)),
    ));
}

/// Install a typed IMU read callback against the live package ABI.
///
/// Package code supplies behavior through the `ImuReadCallback` trait; the SDK owns the
/// firmware callback signature and live binding selection.
#[cfg(not(test))]
/// IMU binding implementation that forwards to the live firmware ABI.
pub struct RealImuBindings;

#[cfg(not(test))]
impl ImuBindings for RealImuBindings {
    fn is_ready(&self) -> bool {
        unsafe { crate::ffi::imu_startup_done() }
    }

    fn roll(&self) -> ImuRoll {
        ImuRoll::new(AngleRadians::from_radians(unsafe {
            crate::ffi::imu_get_roll()
        }))
    }

    fn pitch(&self) -> ImuPitch {
        ImuPitch::new(AngleRadians::from_radians(unsafe {
            crate::ffi::imu_get_pitch()
        }))
    }

    fn yaw(&self) -> ImuYaw {
        ImuYaw::new(AngleRadians::from_radians(unsafe {
            crate::ffi::imu_get_yaw()
        }))
    }

    fn angular_rate(&self) -> ImuAngularRate {
        let mut gyro = [0.0; 3];
        unsafe { crate::ffi::imu_get_gyro(gyro.as_mut_ptr()) };
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(gyro[0])),
            ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(gyro[1])),
            ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(gyro[2])),
        )
    }

    fn orientation(&self) -> ImuOrientation {
        let mut quaternions = [0.0; 4];
        unsafe { crate::ffi::vesc_imu_get_quaternions(quaternions.as_mut_ptr()) };
        ImuOrientation::from_quaternion(ImuQuaternion::from_firmware_wxyz(quaternions))
    }

    fn acceleration(&self) -> ImuAcceleration {
        let mut values = [0.0; 3];
        unsafe { crate::ffi::imu_get_accel(values.as_mut_ptr()) };
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(values[0])),
            ImuAccelerationY::new(AccelerationG::from_g(values[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(values[2])),
        )
    }

    fn magnetic_field(&self) -> ImuMagneticField {
        let mut values = [0.0; 3];
        unsafe { crate::ffi::imu_get_mag(values.as_mut_ptr()) };
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(values[0])),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(values[1])),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(values[2])),
        )
    }

    fn derotated_acceleration(&self) -> ImuAcceleration {
        let mut values = [0.0; 3];
        unsafe { crate::ffi::imu_get_accel_derotated(values.as_mut_ptr()) };
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(values[0])),
            ImuAccelerationY::new(AccelerationG::from_g(values[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(values[2])),
        )
    }

    fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration {
        let input = input.map_axes(|x, y, z| {
            [
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            ]
        });
        let mut output = [0.0; 3];
        unsafe { crate::ffi::imu_derotate(input.as_ptr(), output.as_mut_ptr()) };
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(output[0])),
            ImuAccelerationY::new(AccelerationG::from_g(output[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(output[2])),
        )
    }

    fn derotated_angular_rate(&self) -> ImuAngularRate {
        let mut values = [0.0; 3];
        unsafe { crate::ffi::imu_get_gyro_derotated(values.as_mut_ptr()) };
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(values[0])),
            ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(values[1])),
            ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(values[2])),
        )
    }
}

/// High-level IMU API built on a binding implementation.
#[cfg(not(test))]
pub struct ImuApi<B> {
    bindings: B,
}

mod private {
    pub trait Imu {}
}

/// Semantic IMU capability used by package code.
pub trait Imu: private::Imu {
    /// Return whether firmware IMU startup has completed.
    fn is_ready(&self) -> bool;
    /// Return the current typed roll angle.
    fn roll(&self) -> ImuRoll;
    /// Return the current typed pitch angle.
    fn pitch(&self) -> ImuPitch;
    /// Return the current typed yaw angle.
    fn yaw(&self) -> ImuYaw;
    /// Return the current typed attitude.
    fn attitude(&self) -> ImuAttitude;
    /// Return typed angular rates.
    fn angular_rate(&self) -> ImuAngularRate;
    /// Return the current typed orientation.
    fn orientation(&self) -> ImuOrientation;
    /// Return firmware acceleration axes in g units.
    fn acceleration(&self) -> ImuAcceleration;
    /// Return firmware magnetic-field axes in microteslas.
    fn magnetic_field(&self) -> ImuMagneticField;
    /// Return derotated firmware acceleration axes in g units.
    fn derotated_acceleration(&self) -> ImuAcceleration;
    /// Derotate a caller-provided acceleration vector into the firmware frame.
    fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration;
    /// Return derotated firmware gyro axes in degrees/sec.
    fn derotated_angular_rate(&self) -> ImuAngularRate;
}

#[cfg(not(test))]
impl<B: ImuBindings> ImuApi<B> {
    /// Construct a new IMU API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Return whether firmware IMU startup has completed.
    pub fn is_ready(&self) -> bool {
        self.bindings.is_ready()
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

    /// Return firmware IMU attitude.
    pub fn attitude(&self) -> ImuAttitude {
        self.bindings.attitude()
    }

    /// Return firmware IMU gyro axes in degrees/sec.
    pub fn angular_rate(&self) -> ImuAngularRate {
        self.bindings.angular_rate()
    }

    /// Return firmware IMU orientation.
    pub fn orientation(&self) -> ImuOrientation {
        self.bindings.orientation()
    }

    /// Return firmware acceleration axes in g units.
    pub fn acceleration(&self) -> ImuAcceleration {
        self.bindings.acceleration()
    }

    /// Return firmware magnetic-field axes in microteslas.
    pub fn magnetic_field(&self) -> ImuMagneticField {
        self.bindings.magnetic_field()
    }

    /// Return derotated firmware acceleration axes in g units.
    pub fn derotated_acceleration(&self) -> ImuAcceleration {
        self.bindings.derotated_acceleration()
    }

    /// Derotate a caller-provided acceleration vector into the firmware frame.
    pub fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration {
        self.bindings.derotate_acceleration(input)
    }

    /// Return derotated firmware gyro axes in degrees/sec.
    pub fn derotated_angular_rate(&self) -> ImuAngularRate {
        self.bindings.derotated_angular_rate()
    }
}

#[cfg(not(test))]
impl<B: ImuBindings> private::Imu for ImuApi<B> {}

#[cfg(not(test))]
impl<B: ImuBindings> Imu for ImuApi<B> {
    fn is_ready(&self) -> bool {
        self.is_ready()
    }

    fn roll(&self) -> ImuRoll {
        self.roll()
    }

    fn pitch(&self) -> ImuPitch {
        self.pitch()
    }

    fn yaw(&self) -> ImuYaw {
        self.yaw()
    }

    fn attitude(&self) -> ImuAttitude {
        self.attitude()
    }

    fn angular_rate(&self) -> ImuAngularRate {
        self.angular_rate()
    }

    fn orientation(&self) -> ImuOrientation {
        self.orientation()
    }

    fn acceleration(&self) -> ImuAcceleration {
        self.acceleration()
    }

    fn magnetic_field(&self) -> ImuMagneticField {
        self.magnetic_field()
    }

    fn derotated_acceleration(&self) -> ImuAcceleration {
        self.derotated_acceleration()
    }

    fn derotate_acceleration(&self, input: ImuAcceleration) -> ImuAcceleration {
        self.derotate_acceleration(input)
    }

    fn derotated_angular_rate(&self) -> ImuAngularRate {
        self.derotated_angular_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::{ImuReadCallback, ImuReadHandler, PackageImuReadCallback, imu_read_callback};
    use crate::types::{
        ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRate,
        ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw, ImuMagneticField,
        ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuReadSample, ImuSamplePeriod,
    };
    use crate::units::{AccelerationG, AngularVelocity, MagneticFluxDensity, VescSeconds};
    use core::f32::consts::FRAC_PI_2;
    use core::ptr::NonNull;
    use core::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
    use std::sync::Mutex;

    static LAST_SAMPLE: Mutex<Option<ImuReadSample>> = Mutex::new(None);

    struct CaptureImuRead;

    impl ImuReadCallback for CaptureImuRead {
        fn read(sample: ImuReadSample) {
            *LAST_SAMPLE.lock().expect("capture lock poisoned") = Some(sample);
        }
    }

    struct State {
        samples: u8,
    }

    struct RuntimeImuRead;

    static RUNTIME_STATE: crate::PackageStateStore<State> = crate::PackageStateStore::new();
    static LOADER_RUNTIME_STATE: crate::PackageStateStore<State> = crate::PackageStateStore::new();
    static LOADER_STATE: AtomicPtr<State> = AtomicPtr::new(core::ptr::null_mut());
    static OBSERVED_SAMPLES: AtomicU8 = AtomicU8::new(0);

    unsafe fn loader_state() -> Option<NonNull<State>> {
        NonNull::new(LOADER_STATE.load(Ordering::Acquire))
    }

    impl crate::PackageRuntimeState for State {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &RUNTIME_STATE
        }
    }

    impl ImuReadHandler for RuntimeImuRead {
        type State = State;

        fn read(state: &mut Self::State, _sample: ImuReadSample) {
            state.samples += 1;
            OBSERVED_SAMPLES.store(state.samples, Ordering::SeqCst);
        }
    }

    unsafe impl PackageImuReadCallback for RuntimeImuRead {
        fn image_address() -> usize {
            0
        }
    }

    struct LoaderImuRead;

    impl ImuReadHandler for LoaderImuRead {
        type State = State;

        fn read(state: &mut Self::State, _sample: ImuReadSample) {
            state.samples += 1;
        }
    }

    unsafe impl PackageImuReadCallback for LoaderImuRead {
        fn state_source() -> crate::PackageStateAccess<'static, Self::State> {
            // C map: generated package callbacks use `ARG(PROG_ADDR)` from
            // `third_party/vesc_pkg_lib/vesc_c_if.h:697-700` as this fallback.
            unsafe {
                crate::__macro_support::__package_state_access(&LOADER_RUNTIME_STATE, loader_state)
            }
        }

        fn image_address() -> usize {
            0
        }
    }

    fn typed_sample() -> ImuReadSample {
        ImuReadSample::from_parts(
            ImuAcceleration::from_axes(
                ImuAccelerationX::new(AccelerationG::from_g(1.0)),
                ImuAccelerationY::new(AccelerationG::from_g(2.0)),
                ImuAccelerationZ::new(AccelerationG::from_g(3.0)),
            ),
            ImuAngularRate::from_axes(
                ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(FRAC_PI_2)),
                ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(
                    core::f32::consts::PI,
                )),
                ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(-FRAC_PI_2)),
            ),
            ImuMagneticField::from_axes(
                ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(4.0)),
                ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(5.0)),
                ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(6.0)),
            ),
            ImuSamplePeriod::new(VescSeconds::from_seconds(0.02)),
        )
    }

    #[test]
    fn imu_read_callback_maps_firmware_callback_gyro_radians_per_second() {
        let mut acc = [1.0, 2.0, 3.0];
        let mut gyro = [FRAC_PI_2, core::f32::consts::PI, -FRAC_PI_2];
        let mut mag = [4.0, 5.0, 6.0];

        unsafe {
            imu_read_callback::<CaptureImuRead>(
                acc.as_mut_ptr(),
                gyro.as_mut_ptr(),
                mag.as_mut_ptr(),
                0.02,
            );
        }

        let sample = LAST_SAMPLE.lock().expect("capture lock poisoned").unwrap();
        assert_eq!(
            sample.acceleration().map_axes(|x, y, z| {
                (
                    x.acceleration().as_g(),
                    y.acceleration().as_g(),
                    z.acceleration().as_g(),
                )
            }),
            (1.0, 2.0, 3.0)
        );
        assert_eq!(
            sample.angular_rate().map_axes(|roll, pitch, yaw| {
                (
                    roll.angular_velocity().as_degrees_per_second(),
                    pitch.angular_velocity().as_degrees_per_second(),
                    yaw.angular_velocity().as_degrees_per_second(),
                )
            }),
            (90.0, 180.0, -90.0)
        );
        assert_eq!(
            sample.magnetic_field().map_axes(|x, y, z| {
                (
                    x.magnetic_flux_density().as_microteslas(),
                    y.magnetic_flux_density().as_microteslas(),
                    z.magnetic_flux_density().as_microteslas(),
                )
            }),
            (4.0, 5.0, 6.0)
        );
        assert_eq!(sample.period().duration().as_seconds(), 0.02);
    }

    #[test]
    fn imu_read_handler_dispatches_resolved_package_state() {
        let mut state = State { samples: 0 };
        OBSERVED_SAMPLES.store(0, Ordering::SeqCst);

        // C map: VESC's `lib_get_arg` returns the package ARG at
        // `third_party/vesc/lispBM/lispif_c_lib.c:151-158`; the IMU adapter
        // dispatches the package state source like Float Out Boy's callback at
        // `third_party/float-out-boy/src/main.c:759-764`.
        unsafe { RUNTIME_STATE.install(&mut state) }.unwrap();
        <RuntimeImuRead as ImuReadCallback>::read(typed_sample());
        RUNTIME_STATE.clear();
        assert_eq!(state.samples, 1);
        assert_eq!(OBSERVED_SAMPLES.load(Ordering::SeqCst), 1);

        <RuntimeImuRead as ImuReadCallback>::read(typed_sample());
        assert_eq!(OBSERVED_SAMPLES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn imu_read_handler_validates_loader_package_state_identity() {
        let mut state = State { samples: 0 };
        unsafe { LOADER_RUNTIME_STATE.install(&mut state) }.unwrap();
        LOADER_STATE.store(&mut state, Ordering::Release);

        <LoaderImuRead as ImuReadCallback>::read(typed_sample());

        LOADER_STATE.store(core::ptr::null_mut(), Ordering::Release);
        LOADER_RUNTIME_STATE.clear();
        assert_eq!(state.samples, 1);
    }

    #[test]
    fn runtime_imu_read_callback_keeps_null_firmware_arrays_inert() {
        OBSERVED_SAMPLES.store(0, Ordering::SeqCst);

        let mut gyro = [FRAC_PI_2, core::f32::consts::PI, -FRAC_PI_2];
        let mut mag = [0.0; 3];

        unsafe {
            imu_read_callback::<RuntimeImuRead>(
                core::ptr::null_mut(),
                gyro.as_mut_ptr(),
                mag.as_mut_ptr(),
                0.02,
            );
        }
        assert_eq!(OBSERVED_SAMPLES.load(Ordering::SeqCst), 0);
    }
}
