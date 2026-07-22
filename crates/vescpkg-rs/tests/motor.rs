#![cfg(feature = "test-support")]
//! Integration coverage for typed motor handbrake commands.

use vescpkg_rs::{Current, HandbrakeCurrent, HandbrakeRelative, MotorOutput, Ratio};

#[test]
fn motor_exposes_typed_handbrake_commands() {
    let firmware = vescpkg_rs::test_support::FirmwareTest::new();
    firmware
        .motor()
        .set_handbrake(HandbrakeCurrent::new(Current::from_amps(2.0)));
    firmware
        .motor()
        .set_handbrake_relative(HandbrakeRelative::new(Ratio::from_ratio_const(0.25)));
}
