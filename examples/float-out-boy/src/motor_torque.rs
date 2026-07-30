use vescpkg_rs::prelude::{Current, SignedRatio};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::prelude::{FocMotorFluxLinkage, MotorPoleCount};

const REFLOAT_COMPAT_NEWTON_METERS_PER_AMP: f32 = 1.5 * 15.0 * 0.027;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct MotorTorque(f32);

impl Default for MotorTorque {
    fn default() -> Self {
        Self::ZERO
    }
}

impl MotorTorque {
    pub(crate) const ZERO: Self = Self(0.0);

    pub(crate) const fn from_newton_meters(newton_meters: f32) -> Self {
        Self(newton_meters)
    }

    pub(crate) const fn as_newton_meters(self) -> f32 {
        self.0
    }

    pub(crate) const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub(crate) const fn is_negative(self) -> bool {
        self.0 < 0.0
    }

    pub(crate) const fn signum(self) -> SignedRatio {
        SignedRatio::from_ratio_const(if self.is_negative() { -1.0 } else { 1.0 })
    }
}

impl core::ops::Add for MotorTorque {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Sub for MotorTorque {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl core::ops::Mul<f32> for MotorTorque {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl core::ops::Div<f32> for MotorTorque {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct MotorTorqueConstant(f32);

impl Default for MotorTorqueConstant {
    fn default() -> Self {
        Self::REFLOAT_COMPAT
    }
}

impl MotorTorqueConstant {
    pub(crate) const REFLOAT_COMPAT: Self = Self(REFLOAT_COMPAT_NEWTON_METERS_PER_AMP);

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn from_firmware_config(
        flux_linkage: FocMotorFluxLinkage,
        pole_count: Option<MotorPoleCount>,
    ) -> Self {
        let webers = flux_linkage.flux_linkage().as_webers();
        match pole_count {
            Some(poles) if webers > 0.001 => Self(1.5 * 0.5 * f32::from(poles.as_u16()) * webers),
            _ => Self::REFLOAT_COMPAT,
        }
    }

    pub(crate) const fn newton_meters_per_amp(self) -> f32 {
        self.0
    }

    pub(crate) fn torque_from_current(self, current: Current) -> MotorTorque {
        MotorTorque::from_newton_meters(current.as_amps() * self.newton_meters_per_amp())
    }

    #[cfg(test)]
    pub(crate) fn current_from_torque(self, torque: MotorTorque) -> Current {
        Current::from_amps(torque.as_newton_meters() / self.newton_meters_per_amp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs::prelude::FluxLinkage;

    fn flux(webers: f32) -> FocMotorFluxLinkage {
        FocMotorFluxLinkage::new(FluxLinkage::from_webers(webers))
    }

    #[test]
    fn refloat_compat_constant_matches_the_legacy_current_domain() {
        let torque =
            MotorTorqueConstant::REFLOAT_COMPAT.torque_from_current(Current::from_amps(30.0));

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
        let constant = MotorTorqueConstant::from_firmware_config(
            flux(0.004),
            MotorPoleCount::try_new(14).ok(),
        );

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
            MotorTorqueConstant::from_firmware_config(
                flux(0.001),
                MotorPoleCount::try_new(14).ok(),
            ),
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
}
