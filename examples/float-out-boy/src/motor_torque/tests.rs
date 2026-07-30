use super::*;
use vescpkg_rs::prelude::FluxLinkage;

fn flux(webers: f32) -> FocMotorFluxLinkage {
    FocMotorFluxLinkage::new(FluxLinkage::from_webers(webers))
}

#[test]
fn refloat_compat_constant_matches_the_legacy_current_domain() {
    let torque = MotorTorqueConstant::REFLOAT_COMPAT.torque_from_current(Current::from_amps(30.0));

    assert_f32_eq!(torque.as_newton_meters(), 18.225);
    assert_f32_eq!(
        MotorTorqueConstant::REFLOAT_COMPAT
            .current_from_torque(torque)
            .as_amps(),
        30.0
    );
}

#[test]
fn valid_firmware_motor_config_derives_the_foc_torque_constant() {
    let constant =
        MotorTorqueConstant::from_firmware_config(flux(0.004), MotorPoleCount::try_new(14).ok());

    assert_f32_eq!(constant.newton_meters_per_amp(), 0.042);
    assert_f32_eq!(
        constant
            .torque_from_current(Current::from_amps(30.0))
            .as_newton_meters(),
        1.26
    );
}

#[test]
fn missing_poles_and_old_firmware_flux_use_the_compatibility_constant() {
    assert_eq!(
        MotorTorqueConstant::from_firmware_config(flux(0.004), None),
        MotorTorqueConstant::REFLOAT_COMPAT
    );
    assert_eq!(
        MotorTorqueConstant::from_firmware_config(flux(0.001), MotorPoleCount::try_new(14).ok()),
        MotorTorqueConstant::REFLOAT_COMPAT
    );
}

#[test]
fn torque_sign_and_arithmetic_stay_in_the_torque_domain() {
    let torque = MotorTorque::from_newton_meters(-2.0);

    assert!(torque.is_negative());
    assert_f32_eq!(torque.abs().as_newton_meters(), 2.0);
    assert_eq!(torque.signum(), SignedRatio::from_ratio_const(-1.0));
    assert_f32_eq!(
        (torque + MotorTorque::from_newton_meters(3.0)).as_newton_meters(),
        1.0
    );
}
