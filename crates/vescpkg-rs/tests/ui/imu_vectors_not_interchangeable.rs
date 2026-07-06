use vescpkg_rs::types::{ImuAcceleration, ImuAngularRate};
use vescpkg_rs::units::{AccelerationG, AngularVelocity};

fn update_gyro(_: ImuAngularRate) {}

fn main() {
    let accel = ImuAcceleration::new([
        AccelerationG::from_g(0.0),
        AccelerationG::from_g(0.0),
        AccelerationG::from_g(1.0),
    ]);
    let gyro = ImuAngularRate::new([
        AngularVelocity::from_degrees_per_second(0.0),
        AngularVelocity::from_degrees_per_second(0.0),
        AngularVelocity::from_degrees_per_second(0.0),
    ]);

    update_gyro(gyro);
    update_gyro(accel);
}
