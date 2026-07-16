//! Typed GPIO access for package code.

use vescpkg_rs_sys::VescPin;

use crate::types::AdcVoltage;
use crate::units::Voltage;

/// A firmware analog-input pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnalogPin(u8);

impl AnalogPin {
    /// VESC's first external analog input.
    pub const ADC1: Self = Self(7);
    /// VESC's second external analog input.
    pub const ADC2: Self = Self(8);

    const fn firmware_pin(self) -> VescPin {
        VescPin(self.0 as i32)
    }
}

/// Firmware GPIO capability.
pub struct Gpio {
    #[cfg(test)]
    test: TestGpio,
}

impl Gpio {
    #[cfg(not(test))]
    pub(crate) const fn new() -> Self {
        Self {}
    }

    #[cfg(test)]
    fn test(analog_pair: (f32, f32)) -> Self {
        Self {
            test: TestGpio {
                analog_pair: core::cell::Cell::new(analog_pair),
                ..TestGpio::default()
            },
        }
    }

    /// Read two analog pins as typed firmware-scaled voltages.
    pub fn read_analog_pair(
        &self,
        first: AnalogPin,
        second: AnalogPin,
    ) -> (AdcVoltage, AdcVoltage) {
        let first = first.firmware_pin();
        let second = second.firmware_pin();
        #[cfg(test)]
        {
            self.test
                .analog_pair_calls
                .set(self.test.analog_pair_calls.get() + 1);
            self.test.last_pin.set(first.0);
            self.test.last_second_pin.set(second.0);
            let (first, second) = self.test.analog_pair.get();
            (
                AdcVoltage::new(Voltage::from_volts(first)),
                AdcVoltage::new(Voltage::from_volts(second)),
            )
        }
        #[cfg(not(test))]
        {
            let (first, second) = unsafe { crate::ffi::io_read_analog_pair(first, second) };
            (
                AdcVoltage::new(Voltage::from_volts(first)),
                AdcVoltage::new(Voltage::from_volts(second)),
            )
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct TestGpio {
    analog_pair_calls: core::cell::Cell<usize>,
    last_pin: core::cell::Cell<i32>,
    last_second_pin: core::cell::Cell<i32>,
    analog_pair: core::cell::Cell<(f32, f32)>,
}

#[cfg(test)]
mod tests {
    use super::{AnalogPin, Gpio};

    #[test]
    fn gpio_uses_one_semantic_capability() {
        let gpio = Gpio::test((1.2, 3.4));
        let (first, second) = gpio.read_analog_pair(AnalogPin::ADC1, AnalogPin::ADC2);
        assert_eq!(first.voltage().as_volts(), 1.2);
        assert_eq!(second.voltage().as_volts(), 3.4);
        assert_eq!(gpio.test.analog_pair_calls.get(), 1);
        assert_eq!(gpio.test.last_pin.get(), 7);
        assert_eq!(gpio.test.last_second_pin.get(), 8);
    }
}
