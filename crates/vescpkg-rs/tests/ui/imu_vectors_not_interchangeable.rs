use vescpkg_rs::types::{ImuAcceleration, ImuAngularRate};
use vescpkg_rs::units::{AccelerationG, AngularVelocity};

use vescpkg_rs::types::{
    ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ, ImuAngularRatePitch,
    ImuAngularRateRoll, ImuAngularRateYaw,
};

fn update_gyro(_: ImuAngularRate) {}

fn main() {
    let accel = ImuAcceleration::from_axes(
        ImuAccelerationX::new(AccelerationG::from_g(0.0)),
        ImuAccelerationY::new(AccelerationG::from_g(0.0)),
        ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
    );
    let gyro = ImuAngularRate::from_axes(
        ImuAngularRateRoll::new(AngularVelocity::from_degrees_per_second(0.0)),
        ImuAngularRatePitch::new(AngularVelocity::from_degrees_per_second(0.0)),
        ImuAngularRateYaw::new(AngularVelocity::from_degrees_per_second(0.0)),
    );

    update_gyro(gyro);
    update_gyro(accel);
}
