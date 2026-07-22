#![cfg(feature = "test-support")]
#![allow(missing_docs)]

use vescpkg_rs::{
    AccelerationG, AngularVelocity, FirmwareAhrs, FirmwareAhrsError, FirmwareAhrsParameters,
    ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRate,
    ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw, ImuMagneticField,
    ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuReadCallback, ImuReadCallbackError,
    ImuReadSample, ImuSamplePeriod, MagneticFluxDensity, VescSeconds, register_imu_read_callback,
};

fn vectors() -> (ImuAcceleration, ImuMagneticField) {
    (
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(0.0)),
            ImuAccelerationY::new(AccelerationG::from_g(0.0)),
            ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(10.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(20.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(30.0)),
        ),
    )
}

fn sample(period: f32) -> ImuReadSample {
    let (acceleration, magnetic_field) = vectors();
    ImuReadSample::from_parts(
        acceleration,
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(1.0)),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(2.0)),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(3.0)),
        ),
        magnetic_field,
        ImuSamplePeriod::new(VescSeconds::from_seconds(period)),
    )
}

#[test]
fn firmware_ahrs_copies_initialized_and_updated_state() {
    assert_eq!(
        FirmwareAhrsParameters::default(),
        FirmwareAhrsParameters::defaults()
    );
    let (acceleration, magnetic) = vectors();
    let mut ahrs = FirmwareAhrs::new();
    let initial = ahrs.snapshot();
    assert_eq!(initial.acceleration_magnitude().as_g(), 1.0);
    assert!(!initial.initial_update_done());
    assert_eq!(initial.proportional_gain(), 2.0);
    assert_eq!(initial.madgwick_beta(), 0.1);

    let updated = ahrs
        .update_initial_orientation(acceleration, magnetic)
        .unwrap();
    assert!(updated.initial_update_done());
    assert_eq!(f32::from(updated.orientation().quaternion().w()), 0.9);
    assert_eq!(updated.attitude().pitch().angle().as_radians(), 0.2);
    assert_eq!(ahrs.attitude().roll().angle().as_radians(), 0.1);

    let mahony = ahrs.update_mahony(sample(0.5)).unwrap();
    assert!((mahony.integral_feedback().roll().as_radians_per_second() - 0.5).abs() < 1e-5);
    assert!((mahony.integral_feedback().pitch().as_radians_per_second() - 1.0).abs() < 1e-5);
    assert_eq!(mahony.acceleration_magnitude().as_g(), 1.25);

    let madgwick = ahrs.update_madgwick(sample(0.5)).unwrap();
    assert_eq!(madgwick.madgwick_beta(), 0.2);

    let parameters = FirmwareAhrsParameters::try_new(0.3, 3.0, 0.2, 0.4).unwrap();
    ahrs.set_parameters(parameters).unwrap();
    let configured = ahrs.snapshot();
    assert_eq!(configured.acceleration_confidence_decay(), 0.3);
    assert_eq!(configured.proportional_gain(), 3.0);
    assert_eq!(configured.integral_gain(), 0.2);
    assert_eq!(configured.madgwick_beta(), 0.4);
    assert_eq!(ahrs.parameters(), parameters);
    assert_eq!(ahrs.reset().proportional_gain(), 3.0);
}

#[test]
fn firmware_ahrs_rejects_invalid_vectors_and_periods_before_ffi() {
    let (_, magnetic) = vectors();
    let invalid_acceleration = ImuAcceleration::from_axes(
        ImuAccelerationX::new(AccelerationG::from_g(f32::NAN)),
        ImuAccelerationY::new(AccelerationG::from_g(0.0)),
        ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
    );
    assert_eq!(
        FirmwareAhrs::new().update_initial_orientation(invalid_acceleration, magnetic),
        Err(FirmwareAhrsError::InvalidVector)
    );
    assert_eq!(
        FirmwareAhrs::new().update_mahony(sample(0.0)),
        Err(FirmwareAhrsError::InvalidPeriod)
    );
    assert_eq!(
        FirmwareAhrsParameters::try_new(f32::NAN, 1.0, 1.0, 1.0),
        Err(FirmwareAhrsError::InvalidParameter)
    );
}

struct Callback;

impl ImuReadCallback for Callback {
    fn read(_sample: ImuReadSample) {}
}

#[test]
fn imu_read_callback_registration_is_exclusive_and_released_on_drop() {
    let first = unsafe { register_imu_read_callback::<Callback>() }.unwrap();
    assert!(matches!(
        unsafe { register_imu_read_callback::<Callback>() },
        Err(ImuReadCallbackError::AlreadyRegistered)
    ));
    drop(first);
    assert!(unsafe { register_imu_read_callback::<Callback>() }.is_ok());
}
