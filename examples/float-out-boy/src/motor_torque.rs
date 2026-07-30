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

    pub(crate) fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
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

    pub(crate) fn current_from_torque(self, torque: MotorTorque) -> Current {
        Current::from_amps(torque.as_newton_meters() / self.newton_meters_per_amp())
    }
}

#[cfg(test)]
mod tests;
