#![cfg(feature = "math")]

use vescpkg_rs::{
    AccelerationG, AngularVelocity, ImuAcceleration, ImuAccelerationX, ImuAccelerationY,
    ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll,
    ImuAngularRateYaw, ImuMagneticField, ImuMagneticFieldX, ImuMagneticFieldY,
    ImuMagneticFieldZ, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ,
    ImuReadSample, ImuSamplePeriod, MagneticFluxDensity, SampleRate, VescSeconds,
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
    ahrs.update(sample);
    let quaternion = ahrs.orientation().quaternion();
    assert!(f32::from(quaternion.w()) < 1.0);
    assert!(f32::from(quaternion.z()).abs() > 0.0);
    ahrs.reset();
    let quaternion = ahrs.orientation().quaternion();
    assert_eq!(quaternion.w(), ImuQuaternionW::new(1.0));
    assert_eq!(quaternion.x(), ImuQuaternionX::new(0.0));
    assert_eq!(quaternion.y(), ImuQuaternionY::new(0.0));
    assert_eq!(quaternion.z(), ImuQuaternionZ::new(0.0));
}
