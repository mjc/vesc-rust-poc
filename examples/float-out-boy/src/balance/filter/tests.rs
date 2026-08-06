use super::{BalanceFilter, MahonyPitchGain, MahonyRollGain};
use vescpkg_rs::prelude::{
    AccelerationG, AngularVelocity, ImuAcceleration, ImuAccelerationX, ImuAccelerationY,
    ImuAccelerationZ, ImuAngularRate, ImuAngularRatePitch, ImuAngularRateRoll, ImuAngularRateYaw,
    ImuMagneticField, ImuMagneticFieldX, ImuMagneticFieldY, ImuMagneticFieldZ, ImuOrientation,
    ImuQuaternion, ImuQuaternionW, ImuQuaternionX, ImuQuaternionY, ImuQuaternionZ, ImuReadSample,
    ImuSamplePeriod, MagneticFluxDensity, VescSeconds,
};

fn imu_accel_x(acceleration: AccelerationG) -> ImuAccelerationX {
    ImuAccelerationX::new(acceleration)
}

fn imu_accel_y(acceleration: AccelerationG) -> ImuAccelerationY {
    ImuAccelerationY::new(acceleration)
}

fn imu_accel_z(acceleration: AccelerationG) -> ImuAccelerationZ {
    ImuAccelerationZ::new(acceleration)
}

fn imu_acceleration(
    x: ImuAccelerationX,
    y: ImuAccelerationY,
    z: ImuAccelerationZ,
) -> ImuAcceleration {
    ImuAcceleration::from_axes(x, y, z)
}

fn imu_roll_rate(rate: AngularVelocity) -> ImuAngularRateRoll {
    ImuAngularRateRoll::new(rate)
}

fn imu_pitch_rate(rate: AngularVelocity) -> ImuAngularRatePitch {
    ImuAngularRatePitch::new(rate)
}

fn imu_yaw_rate(rate: AngularVelocity) -> ImuAngularRateYaw {
    ImuAngularRateYaw::new(rate)
}

fn imu_angular_rate(
    roll: ImuAngularRateRoll,
    pitch: ImuAngularRatePitch,
    yaw: ImuAngularRateYaw,
) -> ImuAngularRate {
    ImuAngularRate::from_axes(roll, pitch, yaw)
}

fn imu_period(period: VescSeconds) -> ImuSamplePeriod {
    ImuSamplePeriod::new(period)
}

fn imu_magnetic_field() -> ImuMagneticField {
    ImuMagneticField::from_axes(
        ImuMagneticFieldX::new(MagneticFluxDensity::from_microteslas(0.0)),
        ImuMagneticFieldY::new(MagneticFluxDensity::from_microteslas(0.0)),
        ImuMagneticFieldZ::new(MagneticFluxDensity::from_microteslas(0.0)),
    )
}

fn imu_sample(
    acceleration: ImuAcceleration,
    angular_rate: ImuAngularRate,
    period: ImuSamplePeriod,
) -> ImuReadSample {
    ImuReadSample::from_parts(acceleration, angular_rate, imu_magnetic_field(), period)
}

#[test]
fn balance_filter_update_integrates_positive_pitch_like_float_out_boy_callback() {
    let mut filter = BalanceFilter::source_startup();

    filter.update(imu_sample(
        imu_acceleration(
            imu_accel_x(AccelerationG::from_g(0.0)),
            imu_accel_y(AccelerationG::from_g(0.0)),
            imu_accel_z(AccelerationG::from_g(1.0)),
        ),
        imu_angular_rate(
            imu_roll_rate(AngularVelocity::from_radians_per_second(0.0)),
            imu_pitch_rate(AngularVelocity::from_radians_per_second(1.0)),
            imu_yaw_rate(AngularVelocity::from_radians_per_second(0.0)),
        ),
        imu_period(VescSeconds::from_seconds(0.1)),
    ));

    // Float Out Boy's `imu_ref_callback` forwards gyro/accel/dt at
    // `third_party/float-out-boy/src/main.c:760-765`; `balance_filter_update` integrates the
    // quaternion at `third_party/float-out-boy/src/balance_filter.c:73-134`, and
    // `balance_filter_get_pitch` reads it at `third_party/float-out-boy/src/balance_filter.c:145-154`.
    assert!(filter.balance_pitch().angle().as_radians() > 0.0);
}

#[test]
fn balance_filter_pitch_clamps_quaternion_projection_like_float_out_boy() {
    let positive = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(1.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));
    let negative = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(-1.0),
            ImuQuaternionX::new(0.0),
            ImuQuaternionY::new(1.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));

    // Float Out Boy clamps the asin input before converting to pitch at
    // `third_party/float-out-boy/src/balance_filter.c:145-154`.
    assert_f32_eq!(
        positive.balance_pitch().angle().as_radians(),
        core::f32::consts::FRAC_PI_2
    );
    assert_f32_eq!(
        negative.balance_pitch().angle().as_radians(),
        -core::f32::consts::FRAC_PI_2
    );
}

#[test]
fn balance_filter_configures_yaw_kp_from_pitch_and_roll_like_float_out_boy() {
    let mut filter = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(2.0),
            ImuQuaternionY::new(0.0),
            ImuQuaternionZ::new(0.0),
        ),
    ));
    let (_, measured_gravity) = BalanceFilter::measured_gravity(imu_acceleration(
        imu_accel_x(AccelerationG::from_g(1.0)),
        imu_accel_y(AccelerationG::from_g(0.0)),
        imu_accel_z(AccelerationG::from_g(0.0)),
    ))
    .expect("nonzero accel normalizes");
    let gravity_error = filter.accel_error(measured_gravity);

    filter.configure(MahonyPitchGain::new(4.0), MahonyRollGain::new(2.0));

    // Float Out Boy averages pitch and roll KP for yaw at
    // `third_party/float-out-boy/src/balance_filter.c:64-70`.
    let gains = filter.feedback_gains(0.5);
    let corrected: [AngularVelocity; 3] = core::array::from_fn(|axis| {
        AngularVelocity::from_radians_per_second(10.0)
            + AngularVelocity::from_radians_per_second(gains[axis] * gravity_error[axis])
    });

    assert!((corrected[2].as_radians_per_second() - 16.0).abs() < 0.000_001);
}

#[test]
fn balance_filter_normalizes_accel_before_correction_like_float_out_boy() {
    let (_, unit) = BalanceFilter::measured_gravity(imu_acceleration(
        imu_accel_x(AccelerationG::from_g(0.0)),
        imu_accel_y(AccelerationG::from_g(0.0)),
        imu_accel_z(AccelerationG::from_g(2.0)),
    ))
    .expect("nonzero accel normalizes");

    assert_f32_eq!(unit[0], 0.0);
    assert_f32_eq!(unit[1], 0.0);
    assert_f32_eq!(unit[2], 1.0);
}

#[test]
fn balance_filter_skips_accel_correction_for_tiny_sample_like_float_out_boy() {
    let mut filter = BalanceFilter::source_startup();

    let gyro = filter.gyro_with_accel_correction(
        imu_angular_rate(
            imu_roll_rate(AngularVelocity::from_radians_per_second(1.0)),
            imu_pitch_rate(AngularVelocity::from_radians_per_second(2.0)),
            imu_yaw_rate(AngularVelocity::from_radians_per_second(3.0)),
        ),
        imu_acceleration(
            imu_accel_x(AccelerationG::from_g(0.0)),
            imu_accel_y(AccelerationG::from_g(0.0)),
            imu_accel_z(AccelerationG::from_g(0.005)),
        ),
    );

    assert!((gyro[0].as_radians_per_second() - 1.0).abs() < 0.000_001);
    assert!((gyro[1].as_radians_per_second() - 2.0).abs() < 0.000_001);
    assert!((gyro[2].as_radians_per_second() - 3.0).abs() < 0.000_001);
}

#[test]
fn balance_filter_applies_gravity_error_feedback_like_float_out_boy() {
    let mut filter = BalanceFilter::source_startup();

    let gyro = filter.gyro_with_accel_correction(
        imu_angular_rate(
            imu_roll_rate(AngularVelocity::from_radians_per_second(1.0)),
            imu_pitch_rate(AngularVelocity::from_radians_per_second(2.0)),
            imu_yaw_rate(AngularVelocity::from_radians_per_second(3.0)),
        ),
        imu_acceleration(
            imu_accel_x(AccelerationG::from_g(0.0)),
            imu_accel_y(AccelerationG::from_g(1.0)),
            imu_accel_z(AccelerationG::from_g(0.0)),
        ),
    );

    assert!((gyro[0].as_radians_per_second() - 2.4).abs() < 0.000_001);
    assert!((gyro[1].as_radians_per_second() - 2.0).abs() < 0.000_001);
    assert!((gyro[2].as_radians_per_second() - 3.0).abs() < 0.000_001);
}

#[test]
fn balance_filter_integrates_gyro_components_like_float_out_boy() {
    let mut filter = BalanceFilter::from_orientation(ImuOrientation::from_quaternion(
        ImuQuaternion::from_components(
            ImuQuaternionW::new(1.0),
            ImuQuaternionX::new(2.0),
            ImuQuaternionY::new(3.0),
            ImuQuaternionZ::new(4.0),
        ),
    ));

    filter.integrate_gyro(
        [0.2, 0.4, 0.6].map(AngularVelocity::from_radians_per_second),
        VescSeconds::from_seconds(0.5),
    );

    let [scalar, body_x, body_y, body_z] = filter.orientation_for_test();
    assert!((scalar - 0.0).abs() < 0.000_001);
    assert!((body_x - 2.1).abs() < 0.000_001);
    assert!((body_y - 3.0).abs() < 0.000_001);
    assert!((body_z - 4.2).abs() < 0.000_001);
}

#[test]
fn balance_filter_preserves_multisample_refloat_trajectory() {
    let mut filter = BalanceFilter::source_startup();
    for (acceleration, angular_rate, period) in [
        ([0.2, -0.1, 0.97], [0.3, -0.2, 0.1], 0.01),
        ([-0.4, 0.3, 0.85], [-0.6, 0.4, -0.2], 0.02),
        ([0.05, 0.15, 1.1], [0.2, 0.7, 0.5], 0.015),
    ] {
        filter.update(imu_sample(
            imu_acceleration(
                imu_accel_x(AccelerationG::from_g(acceleration[0])),
                imu_accel_y(AccelerationG::from_g(acceleration[1])),
                imu_accel_z(AccelerationG::from_g(acceleration[2])),
            ),
            imu_angular_rate(
                imu_roll_rate(AngularVelocity::from_radians_per_second(angular_rate[0])),
                imu_pitch_rate(AngularVelocity::from_radians_per_second(angular_rate[1])),
                imu_yaw_rate(AngularVelocity::from_radians_per_second(angular_rate[2])),
            ),
            imu_period(VescSeconds::from_seconds(period)),
        ));
    }

    assert_eq!(
        filter.orientation_for_test().map(f32::to_bits),
        [
            0.999_904_33,
            0.002_009_128_7,
            0.013_503_214,
            0.002_211_600_8
        ]
        .map(f32::to_bits)
    );
}

#[test]
fn balance_filter_state_stays_one_quaternion_and_four_scalars() {
    assert_eq!(core::mem::size_of::<BalanceFilter>(), 32);
    assert_eq!(core::mem::align_of::<BalanceFilter>(), 4);
}
