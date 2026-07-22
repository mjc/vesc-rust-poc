#![cfg(feature = "test-support")]
//! Integration coverage for typed firmware IMU vectors.

use vescpkg_rs::{
    AngleDegrees, AngleRadians, Imu, ImuCalibrationError, ImuPitch, ImuRoll, ImuYaw,
    test_support::FirmwareTest,
};

#[test]
fn firmware_imu_exposes_vectors_and_derotated_samples() {
    let firmware = FirmwareTest::new();
    let imu = firmware.imu();
    assert!(!imu.is_ready());
    firmware.set_imu_ready(true);
    assert!(imu.is_ready());
    imu.set_yaw(ImuYaw::new(AngleRadians::from_radians(0.5)));
    assert!((imu.yaw().angle().as_radians() - 0.5).abs() < 1.0e-6);
    firmware.set_imu_attitude(
        ImuRoll::new(AngleRadians::from_radians(0.1)),
        ImuPitch::new(AngleRadians::from_radians(-0.2)),
        ImuYaw::new(AngleRadians::from_radians(0.3)),
    );
    let rpy = imu.rpy();
    assert!((rpy.roll().angle().as_radians() - 0.1).abs() < 1.0e-6);
    assert!((rpy.pitch().angle().as_radians() + 0.2).abs() < 1.0e-6);
    assert!((rpy.yaw().angle().as_radians() - 0.3).abs() < 1.0e-6);
    let calibration = imu
        .calibrate(AngleDegrees::from_degrees(90.0))
        .expect("calibration snapshot");
    assert_eq!(calibration.roll().as_degrees(), 1.0);
    assert_eq!(calibration.pitch().as_degrees(), 2.0);
    assert_eq!(calibration.yaw().as_degrees(), 3.0);
    assert_eq!(
        calibration.acceleration_offset().map_axes(|x, y, z| [
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        ]),
        [4.0, 5.0, 6.0]
    );
    assert_eq!(
        calibration.angular_rate_offset().map_axes(|x, y, z| [
            x.angular_velocity().as_degrees_per_second(),
            y.angular_velocity().as_degrees_per_second(),
            z.angular_velocity().as_degrees_per_second(),
        ]),
        [7.0, 8.0, 9.0]
    );
    assert_eq!(
        imu.calibrate(AngleDegrees::from_degrees(f32::NAN)),
        Err(ImuCalibrationError::InvalidYaw)
    );

    assert_eq!(
        imu.acceleration().map_axes(|x, y, z| [
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        ]),
        [1.0, 2.0, 3.0]
    );
    assert_eq!(
        imu.magnetic_field().map_axes(|x, y, z| [
            x.magnetic_flux_density().as_microteslas(),
            y.magnetic_flux_density().as_microteslas(),
            z.magnetic_flux_density().as_microteslas(),
        ]),
        [10.0, 20.0, 30.0]
    );
    assert_eq!(
        imu.derotated_acceleration().map_axes(|x, y, z| [
            x.acceleration().as_g(),
            y.acceleration().as_g(),
            z.acceleration().as_g(),
        ]),
        [4.0, 5.0, 6.0]
    );
    assert_eq!(
        imu.derotate_acceleration(imu.acceleration())
            .map_axes(|x, y, z| [
                x.acceleration().as_g(),
                y.acceleration().as_g(),
                z.acceleration().as_g(),
            ]),
        [4.0, 5.0, 6.0]
    );
    assert_eq!(
        imu.derotated_angular_rate().map_axes(|x, y, z| [
            x.angular_velocity().as_degrees_per_second(),
            y.angular_velocity().as_degrees_per_second(),
            z.angular_velocity().as_degrees_per_second(),
        ]),
        [7.0, 8.0, 9.0]
    );
}
