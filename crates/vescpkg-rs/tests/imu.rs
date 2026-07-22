#![cfg(feature = "test-support")]
//! Integration coverage for typed firmware IMU vectors.

use vescpkg_rs::{AngleRadians, Imu, ImuYaw, test_support::FirmwareTest};

#[test]
fn firmware_imu_exposes_vectors_and_derotated_samples() {
    let firmware = FirmwareTest::new();
    let imu = firmware.imu();
    assert!(!imu.is_ready());
    firmware.set_imu_ready(true);
    assert!(imu.is_ready());
    imu.set_yaw(ImuYaw::new(AngleRadians::from_radians(0.5)));
    assert!((imu.yaw().angle().as_radians() - 0.5).abs() < 1.0e-6);

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
