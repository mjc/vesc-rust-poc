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

#[test]
fn mahony_ahrs_integrates_rate_and_can_reset() {
    let mut ahrs = vescpkg_rs::Ahrs::new();
    let sample = ImuReadSample::from_parts(
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(0.0)),
            ImuAccelerationY::new(AccelerationG::from_g(0.0)),
            ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
        ),
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(1.0)),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(1.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
        ),
        ImuSamplePeriod::new(VescSeconds::from_seconds(0.1)),
    );
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
    let sample = ImuReadSample::from_parts(
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(0.0)),
            ImuAccelerationY::new(AccelerationG::from_g(0.0)),
            ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
        ),
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(0.0)),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(1.0)),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(1.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
        ),
        ImuSamplePeriod::new(VescSeconds::from_seconds(0.1)),
    );

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
