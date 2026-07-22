#![cfg(feature = "math")]
#![doc = "Integration coverage for package-owned AHRS state."]
#![allow(missing_docs)]

use vescpkg_rs::{
    AccelerationG, AngularVelocity, ImuAcceleration, ImuAccelerationX, ImuAccelerationY,
    ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw,
    ImuMagneticField, ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuQuaternionW,
    ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ, ImuReadSample, ImuSamplePeriod,
    MagneticFluxDensity, VescSeconds,
};

fn sample(period: f32, acceleration: [f32; 3], yaw_rate: f32) -> ImuReadSample {
    ImuReadSample::from_parts(
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(acceleration[0])),
            ImuAccelerationY::new(AccelerationG::from_g(acceleration[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(acceleration[2])),
        ),
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(yaw_rate)),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(1.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
        ),
        ImuSamplePeriod::new(VescSeconds::from_seconds(period)),
    )
}

#[test]
fn mahony_ahrs_integrates_rate_and_can_reset() {
    let mut ahrs = vescpkg_rs::Ahrs::new();
    let sample = sample(0.1, [0.0, 0.0, 1.0], 1.0);
    let estimate = ahrs.update(sample);
    assert_eq!(estimate, ahrs.orientation());
    let quaternion = ahrs.orientation().quaternion();
    assert!(f32::from(quaternion.w()) < 1.0);
    assert!(f32::from(quaternion.z()).abs() > 0.0);
    ahrs.reset();
    let quaternion = ahrs.orientation().quaternion();
    assert_eq!(quaternion.w(), ImuQuaternionW::new(1.0));
    assert_eq!(quaternion.x(), ImuQuaternionX::new(0.0));
    assert_eq!(quaternion.y(), ImuQuaternionY::new(0.0));
    assert_eq!(quaternion.z(), ImuQuaternionZ::new(0.0));
    assert!(!ahrs.set_gains(f32::NAN, 0.1));
    assert!(!ahrs.set_gains(1.0, -0.1));
}

#[test]
fn madgwick_ahrs_integrates_rate_and_validates_beta() {
    let mut ahrs = vescpkg_rs::Madgwick::new();
    let sample = sample(0.1, [0.0, 0.0, 1.0], 1.0);

    let estimate = ahrs.update(sample);
    assert_eq!(estimate, ahrs.orientation());
    assert!(f32::from(ahrs.orientation().quaternion().z()).abs() > 0.0);
    assert!(ahrs.set_beta(0.2));
    assert!(!ahrs.set_beta(f32::NAN));
    assert!(!ahrs.set_beta(-0.1));
    ahrs.reset();
    assert_eq!(
        ahrs.orientation().quaternion().w(),
        ImuQuaternionW::new(1.0)
    );
}

#[test]
fn madgwick_rejects_invalid_periods_and_survives_missing_acceleration() {
    let mut ahrs = vescpkg_rs::Madgwick::new();
    let identity = ahrs.orientation();
    assert_eq!(ahrs.update(sample(0.0, [0.0, 0.0, 1.0], 1.0)), identity);

    let estimate = ahrs.update(sample(0.1, [0.0, 0.0, 0.0], 1.0));
    assert_eq!(estimate, ahrs.orientation());
    for component in [
        f32::from(estimate.quaternion().w()),
        f32::from(estimate.quaternion().x()),
        f32::from(estimate.quaternion().y()),
        f32::from(estimate.quaternion().z()),
    ] {
        assert!(component.is_finite());
    }
}

#[test]
fn package_ahrs_initial_orientation_uses_accel_and_magnetometer() {
    let acceleration = ImuAcceleration::from_axes(
        ImuAccelerationX::new(AccelerationG::from_g(0.0)),
        ImuAccelerationY::new(AccelerationG::from_g(0.0)),
        ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
    );
    let magnetic = ImuMagneticField::from_axes(
        ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(1.0)),
        ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
        ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
    );

    let mut mahony = vescpkg_rs::Ahrs::new();
    assert_eq!(
        mahony.update_initial_orientation(acceleration, magnetic),
        mahony.orientation()
    );
    assert_eq!(
        mahony.orientation().quaternion().w(),
        ImuQuaternionW::new(1.0)
    );

    let mut madgwick = vescpkg_rs::Madgwick::new();
    assert_eq!(
        madgwick.update_initial_orientation(acceleration, magnetic),
        madgwick.orientation()
    );
    assert_eq!(
        madgwick.orientation().quaternion().w(),
        ImuQuaternionW::new(1.0)
    );
}

#[test]
fn package_ahrs_initial_orientation_resets_on_invalid_vectors() {
    let acceleration = ImuAcceleration::from_axes(
        ImuAccelerationX::new(AccelerationG::from_g(0.0)),
        ImuAccelerationY::new(AccelerationG::from_g(0.0)),
        ImuAccelerationZ::new(AccelerationG::from_g(0.0)),
    );
    let magnetic = ImuMagneticField::from_axes(
        ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(1.0)),
        ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
        ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
    );
    let mut ahrs = vescpkg_rs::Madgwick::new();
    assert_eq!(
        ahrs.update_initial_orientation(acceleration, magnetic),
        ahrs.orientation()
    );
    assert_eq!(
        ahrs.orientation().quaternion().w(),
        ImuQuaternionW::new(1.0)
    );
}
