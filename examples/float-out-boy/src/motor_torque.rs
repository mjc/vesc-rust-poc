#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{FocMotorFluxLinkage, MotorPoleCount};
pub(crate) use vescpkg_rs::prelude::{MotorTorque, MotorTorqueConstant};

const REFLOAT_COMPAT_NEWTON_METERS_PER_AMP: f32 = 1.5 * 15.0 * 0.027;
pub(crate) const REFLOAT_COMPAT_TORQUE_CONSTANT: MotorTorqueConstant =
    MotorTorqueConstant::from_newton_meters_per_amp(REFLOAT_COMPAT_NEWTON_METERS_PER_AMP);

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn motor_torque_constant_from_firmware_config(
    flux_linkage: FocMotorFluxLinkage,
    pole_count: Option<MotorPoleCount>,
) -> MotorTorqueConstant {
    let webers = flux_linkage.flux_linkage().as_webers();
    pole_count
        .filter(|_| webers > 0.001)
        .map_or(REFLOAT_COMPAT_TORQUE_CONSTANT, |poles| {
            MotorTorqueConstant::from_newton_meters_per_amp(
                1.5 * 0.5 * f32::from(poles.as_u16()) * webers,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::prelude::{
        Current, FluxLinkage, MotorCurrent, MotorCurrentLimit, MotorTorqueLimit,
    };

    fn flux(webers: f32) -> FocMotorFluxLinkage {
        FocMotorFluxLinkage::new(FluxLinkage::from_webers(webers))
    }

    #[test]
    fn refloat_compat_constant_matches_the_legacy_current_domain() {
        let torque = REFLOAT_COMPAT_TORQUE_CONSTANT.torque_from_current(Current::from_amps(30.0));

        assert_f32_eq!(torque.as_newton_meters(), 18.225);
        assert_f32_eq!(
            REFLOAT_COMPAT_TORQUE_CONSTANT
                .current_from_torque(torque)
                .as_amps(),
            30.0
        );
    }

    #[test]
    fn valid_firmware_motor_config_derives_the_foc_torque_constant() {
        let constant = motor_torque_constant_from_firmware_config(
            flux(0.004),
            MotorPoleCount::try_new(14).ok(),
        );

        assert_f32_eq!(constant.as_newton_meters_per_amp(), 0.042);
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
            motor_torque_constant_from_firmware_config(flux(0.004), None),
            REFLOAT_COMPAT_TORQUE_CONSTANT
        );
        assert_eq!(
            motor_torque_constant_from_firmware_config(
                flux(0.001),
                MotorPoleCount::try_new(14).ok(),
            ),
            REFLOAT_COMPAT_TORQUE_CONSTANT
        );
    }

    #[test]
    fn torque_sign_and_arithmetic_stay_in_the_torque_domain() {
        let torque = MotorTorque::from_newton_meters(-2.0);

        assert!(torque.is_negative());
        assert_f32_eq!(torque.abs().as_newton_meters(), 2.0);
        assert_f32_eq!(torque.signum(), -1.0);
        assert_f32_eq!(
            (torque + MotorTorque::from_newton_meters(3.0)).as_newton_meters(),
            1.0
        );
        assert!(MotorTorque::from_newton_meters(1.0).is_positive());
        assert_eq!(-torque, MotorTorque::from_newton_meters(2.0));
    }

    #[test]
    fn torque_limits_are_magnitudes_and_preserve_requested_sign() {
        let limit = MotorTorqueLimit::new(MotorTorque::from_newton_meters(-2.0));

        assert_eq!(
            MotorTorque::from_newton_meters(3.0).clamped_to(limit),
            MotorTorque::from_newton_meters(2.0)
        );
        assert_eq!(
            MotorTorque::from_newton_meters(-3.0).clamped_to(limit),
            MotorTorque::from_newton_meters(-2.0)
        );
        assert_eq!(
            MotorTorque::from_newton_meters(1.0).clamped_to(limit),
            MotorTorque::from_newton_meters(1.0)
        );
    }

    #[test]
    fn typed_motor_current_and_limit_conversions_round_trip() {
        let constant = REFLOAT_COMPAT_TORQUE_CONSTANT;
        let current = MotorCurrent::new(Current::from_amps(-12.0));
        let torque = constant.torque_from_motor_current(current);
        let limit = constant
            .torque_limit_from_current_limit(MotorCurrentLimit::new(Current::from_amps(-10.0)));

        assert_eq!(constant.motor_current_from_torque(torque), current);
        assert_f32_eq!(
            torque.clamped_to(limit).as_newton_meters(),
            -10.0 * REFLOAT_COMPAT_NEWTON_METERS_PER_AMP
        );
    }
}
