#![doc = "Integration coverage for package-owned AHRS state."]
#![cfg(feature = "math")]
#![allow(missing_docs)]

use vescpkg_rs::{
    AccelerationG, AngularVelocity, AxisMahony, ImuAcceleration, ImuAccelerationX,
    ImuAccelerationY, ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll,
    ImuAngularRateYaw, ImuMagneticField, ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ,
    ImuOrientation, ImuQuaternion, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ,
    ImuReadSample, ImuSamplePeriod, MagneticFluxDensity, MahonyPitchGain, MahonyRollGain, Ratio,
    VescSeconds,
};

#[test]
fn axis_mahony_starts_at_identity_with_fixed_state_layout() {
    let filter = AxisMahony::default();

    assert_eq!(filter.pitch().as_radians().to_bits(), (-0.0_f32).to_bits());
    assert_eq!(core::mem::size_of::<AxisMahony>(), 32);
    assert_eq!(core::mem::align_of::<AxisMahony>(), 4);
}

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

fn axis_sample(period: f32, acceleration: [f32; 3], rate: [f32; 3]) -> ImuReadSample {
    ImuReadSample::from_parts(
        ImuAcceleration::from_axes(
            ImuAccelerationX::new(AccelerationG::from_g(acceleration[0])),
            ImuAccelerationY::new(AccelerationG::from_g(acceleration[1])),
            ImuAccelerationZ::new(AccelerationG::from_g(acceleration[2])),
        ),
        ImuAngularRate::from_axes(
            ImuAngularRateRoll::new(AngularVelocity::from_radians_per_second(rate[0])),
            ImuAngularRatePitch::new(AngularVelocity::from_radians_per_second(rate[1])),
            ImuAngularRateYaw::new(AngularVelocity::from_radians_per_second(rate[2])),
        ),
        ImuMagneticField::from_axes(
            ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
            ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
        ),
        ImuSamplePeriod::new(VescSeconds::from_seconds(period)),
    )
}

#[test]
fn axis_mahony_preserves_refloat_projection_gains_and_trajectory() {
    let mut filter = AxisMahony::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(1.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));
    assert_eq!(
        filter.pitch().as_radians().to_bits(),
        core::f32::consts::FRAC_PI_2.to_bits()
    );
    filter.configure(MahonyPitchGain::new(4.0), MahonyRollGain::new(2.0));
    assert_eq!(
        filter.configured_gains(),
        (MahonyPitchGain::new(4.0), MahonyRollGain::new(2.0))
    );

    let mut filter = AxisMahony::default();
    for (acceleration, angular_rate, period) in [
        ([0.2, -0.1, 0.97], [0.3, -0.2, 0.1], 0.01),
        ([-0.4, 0.3, 0.85], [-0.6, 0.4, -0.2], 0.02),
        ([0.05, 0.15, 1.1], [0.2, 0.7, 0.5], 0.015),
    ] {
        filter.update(
            axis_sample(period, acceleration, angular_rate),
            Ratio::from_ratio_const(0.1),
            0.02,
        );
    }
    let quaternion = filter.orientation().quaternion();
    assert_eq!(
        [
            f32::from(quaternion.w()),
            f32::from(quaternion.x()),
            f32::from(quaternion.y()),
            f32::from(quaternion.z()),
        ]
        .map(f32::to_bits),
        [
            0.999_904_33,
            0.002_009_128_7,
            0.013_503_214,
            0.002_211_600_8,
        ]
        .map(f32::to_bits),
    );
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
    assert_eq!(
        ahrs.set_gains(f32::NAN, 0.1),
        Err(vescpkg_rs::AhrsParameterError::NonFinite)
    );
    assert_eq!(
        ahrs.set_gains(1.0, -0.1),
        Err(vescpkg_rs::AhrsParameterError::Negative)
    );
    assert!(vescpkg_rs::Ahrs::with_gains(1.0, 0.1).is_ok());
    assert!(matches!(
        vescpkg_rs::Ahrs::with_gains(-1.0, 0.1),
        Err(vescpkg_rs::AhrsParameterError::Negative)
    ));
}

#[test]
fn madgwick_ahrs_integrates_rate_and_validates_beta() {
    let mut ahrs = vescpkg_rs::Madgwick::new();
    let sample = sample(0.1, [0.0, 0.0, 1.0], 1.0);

    let estimate = ahrs.update(sample);
    assert_eq!(estimate, ahrs.orientation());
    assert!(f32::from(ahrs.orientation().quaternion().z()).abs() > 0.0);
    assert_eq!(ahrs.set_beta(0.2), Ok(()));
    assert_eq!(
        ahrs.set_beta(f32::NAN),
        Err(vescpkg_rs::AhrsParameterError::NonFinite)
    );
    assert_eq!(
        ahrs.set_beta(-0.1),
        Err(vescpkg_rs::AhrsParameterError::Negative)
    );
    assert!(vescpkg_rs::Madgwick::with_beta(0.2).is_ok());
    assert!(matches!(
        vescpkg_rs::Madgwick::with_beta(f32::INFINITY),
        Err(vescpkg_rs::AhrsParameterError::NonFinite)
    ));
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
