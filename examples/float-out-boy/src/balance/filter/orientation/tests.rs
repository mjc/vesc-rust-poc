use super::EstimatedOrientation;
use vescpkg_rs::prelude::{
    AngleRadians, ImuOrientation, ImuQuaternion, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY,
    ImuQuaternionZ,
};

#[test]
fn balance_pitch_clamps_projection_like_float_out_boy() {
    let positive = EstimatedOrientation::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(1.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));
    let negative = EstimatedOrientation::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(-1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(1.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));

    assert_eq!(
        positive.balance_pitch().angle(),
        AngleRadians::from_radians(core::f32::consts::FRAC_PI_2)
    );
    assert_eq!(
        negative.balance_pitch().angle(),
        AngleRadians::from_radians(-core::f32::consts::FRAC_PI_2)
    );
}

#[test]
fn zero_firmware_quaternion_remains_invalid_like_refloat() {
    let mut orientation = EstimatedOrientation::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(0.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(0.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));

    orientation.normalize();

    assert!(orientation.wxyz_for_test().into_iter().all(f32::is_nan));
    assert!(orientation.balance_pitch().angle().as_radians().is_nan());
}
