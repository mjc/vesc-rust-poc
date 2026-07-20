//! Typed GPIO access for package code.

use vescpkg_rs_sys::VescPin;

use crate::types::AdcVoltage;
use crate::units::Voltage;

fn adc_voltage_from_firmware(raw: f32) -> AdcVoltage {
    AdcVoltage::new(Voltage::from_volts(raw))
}

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

    /// Read one analog pin as a typed firmware-scaled voltage.
    pub fn read_analog(&self, pin: AnalogPin) -> AdcVoltage {
        let pin = pin.firmware_pin();
        #[cfg(test)]
        {
            self.test
                .analog_pair_calls
                .set(self.test.analog_pair_calls.get() + 1);
            self.test.last_pin.set(pin.0);
            let (first, second) = self.test.analog_pair.get();
            adc_voltage_from_firmware(match pin.0 {
                7 => first,
                8 => second,
                _ => unreachable!("AnalogPin only exposes VESC ADC inputs"),
            })
        }
        #[cfg(not(test))]
        {
            adc_voltage_from_firmware(unsafe { crate::ffi::io_read_analog(pin) })
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct TestGpio {
    analog_pair_calls: core::cell::Cell<usize>,
    last_pin: core::cell::Cell<i32>,
    analog_pair: core::cell::Cell<(f32, f32)>,
}

#[cfg(test)]
mod tests {
    use super::{AnalogPin, Gpio};

    #[test]
    fn gpio_uses_one_semantic_capability() {
        let gpio = Gpio::test((1.2, 3.4));
        assert_eq!(gpio.read_analog(AnalogPin::ADC1).voltage().as_volts(), 1.2);
        assert_eq!(gpio.read_analog(AnalogPin::ADC2).voltage().as_volts(), 3.4);
        assert_eq!(gpio.test.analog_pair_calls.get(), 2);
        assert_eq!(gpio.test.last_pin.get(), 8);
    }
}
