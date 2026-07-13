//! Typed GPIO access for package code.

use vescpkg_rs_sys::VescPin;

use crate::units::Voltage;

/// A firmware analog-input pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnalogPin(u8);

impl AnalogPin {
    /// Name an analog pin by its VESC firmware pin number.
    pub const fn new(number: u8) -> Self {
        Self(number)
    }

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
    pub fn read_analog_pair(&self, first: AnalogPin, second: AnalogPin) -> (Voltage, Voltage) {
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
            (Voltage::from_volts(first), Voltage::from_volts(second))
        }
        #[cfg(not(test))]
        {
            let (first, second) = unsafe { crate::ffi::io_read_analog_pair(first, second) };
            (Voltage::from_volts(first), Voltage::from_volts(second))
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
        let (first, second) = gpio.read_analog_pair(AnalogPin::new(42), AnalogPin::new(43));
        assert_eq!(first.as_volts(), 1.2);
        assert_eq!(second.as_volts(), 3.4);
        assert_eq!(gpio.test.analog_pair_calls.get(), 1);
        assert_eq!(gpio.test.last_pin.get(), 42);
        assert_eq!(gpio.test.last_second_pin.get(), 43);
    }
}
