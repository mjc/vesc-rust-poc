//! Electrical unit newtypes and obvious dimensional arithmetic.

use core::ops::{Div, Mul};

use crate::battery::Energy;
use crate::scalar_unit;
use crate::time::{SystemTicks, system_ticks_as_secs_f32};

scalar_unit!(Voltage, from_volts, as_volts, "volts");
scalar_unit!(Current, from_amps, as_amps, "amps");
scalar_unit!(Power, from_watts, as_watts, "watts");
scalar_unit!(Resistance, from_ohms, as_ohms, "ohms");
scalar_unit!(Inductance, from_henries, as_henries, "henries");
scalar_unit!(FluxLinkage, from_webers, as_webers, "webers");
scalar_unit!(
    MagneticFluxDensity,
    from_microteslas,
    as_microteslas,
    "microteslas"
);

/// Positive number of series-connected battery cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BatteryCellCount(u16);

impl BatteryCellCount {
    /// Create a checked non-zero battery cell count.
    pub const fn try_new(count: u16) -> Result<Self, BatteryCellCountError> {
        if count == 0 {
            Err(BatteryCellCountError { value: count })
        } else {
            Ok(Self(count))
        }
    }

    /// Encode the count for a firmware boundary.
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// Error returned when the battery cell count is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryCellCountError {
    value: u16,
}

impl BatteryCellCountError {
    /// Return the rejected count.
    pub const fn value(self) -> u16 {
        self.value
    }
}

impl Mul<BatteryCellCount> for Voltage {
    type Output = Voltage;

    fn mul(self, rhs: BatteryCellCount) -> Self::Output {
        Voltage::from_volts(self.as_volts() * f32::from(rhs.0))
    }
}

impl Mul<Voltage> for BatteryCellCount {
    type Output = Voltage;

    fn mul(self, rhs: Voltage) -> Self::Output {
        rhs * self
    }
}

impl Mul<Current> for Voltage {
    type Output = Power;

    fn mul(self, rhs: Current) -> Self::Output {
        Power::from_watts(self.as_volts() * rhs.as_amps())
    }
}

impl Mul<Voltage> for Current {
    type Output = Power;

    fn mul(self, rhs: Voltage) -> Self::Output {
        rhs * self
    }
}

impl Div<Voltage> for Power {
    type Output = Current;

    fn div(self, rhs: Voltage) -> Self::Output {
        Current::from_amps(self.as_watts() / rhs.as_volts())
    }
}

impl Div<Current> for Power {
    type Output = Voltage;

    fn div(self, rhs: Current) -> Self::Output {
        Voltage::from_volts(self.as_watts() / rhs.as_amps())
    }
}

impl Div<Current> for Voltage {
    type Output = Resistance;

    fn div(self, rhs: Current) -> Self::Output {
        Resistance::from_ohms(self.as_volts() / rhs.as_amps())
    }
}

impl Mul<Resistance> for Current {
    type Output = Voltage;

    fn mul(self, rhs: Resistance) -> Self::Output {
        Voltage::from_volts(self.as_amps() * rhs.as_ohms())
    }
}

impl Mul<Current> for Resistance {
    type Output = Voltage;

    fn mul(self, rhs: Current) -> Self::Output {
        rhs * self
    }
}

impl Mul<SystemTicks> for Power {
    type Output = Energy;

    fn mul(self, rhs: SystemTicks) -> Self::Output {
        Energy::from_joules(self.as_watts() * system_ticks_as_secs_f32(rhs))
    }
}
