//! Typed GPIO access for package code.

use vescpkg_rs_sys::VescPin;
#[cfg(not(test))]
use vescpkg_rs_sys::VescPinMode;

use crate::types::AdcVoltage;
use crate::units::Voltage;

fn adc_voltage_from_firmware(raw: f32) -> AdcVoltage {
    AdcVoltage::new(Voltage::from_volts(raw))
}

/// A firmware analog-input pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnalogPin(i32);

impl AnalogPin {
    /// VESC's first external analog input.
    pub const ADC1: Self = Self(7);
    /// VESC's second external analog input.
    pub const ADC2: Self = Self(8);

    const fn firmware_pin(self) -> VescPin {
        VescPin(self.0)
    }
}

/// A firmware digital GPIO pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct DigitalPin(VescPin);

impl DigitalPin {
    /// VESC's Servo/PPM pin.
    pub const PPM: Self = Self(VescPin(12));
}

/// Digital GPIO output level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitalOutputLevel {
    /// Logic-low output.
    Low,
    /// Logic-high output.
    High,
}

impl DigitalOutputLevel {
    #[cfg(not(test))]
    const fn firmware_level(self) -> i32 {
        match self {
            Self::Low => 0,
            Self::High => 1,
        }
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

    /// Configure a digital pin as a push-pull output.
    pub fn configure_output(&self, pin: DigitalPin) -> bool {
        #[cfg(test)]
        {
            self.test
                .output_mode_calls
                .set(self.test.output_mode_calls.get() + 1);
            self.test.last_pin.set(pin.0.0);
            true
        }
        #[cfg(not(test))]
        {
            unsafe { crate::ffi::io_set_mode(pin.0, VescPinMode(3)) }
        }
    }

    /// Drive a configured digital output pin.
    pub fn write(&self, pin: DigitalPin, level: DigitalOutputLevel) -> bool {
        #[cfg(test)]
        {
            self.test.write_calls.set(self.test.write_calls.get() + 1);
            self.test.last_pin.set(pin.0.0);
            self.test.last_output_level.set(Some(level));
            true
        }
        #[cfg(not(test))]
        {
            unsafe { crate::ffi::io_write(pin.0, level.firmware_level()) }
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct TestGpio {
    analog_pair_calls: core::cell::Cell<usize>,
    output_mode_calls: core::cell::Cell<usize>,
    write_calls: core::cell::Cell<usize>,
    last_pin: core::cell::Cell<i32>,
    last_output_level: core::cell::Cell<Option<DigitalOutputLevel>>,
    analog_pair: core::cell::Cell<(f32, f32)>,
}

#[cfg(test)]
mod tests {
    use super::{AnalogPin, DigitalOutputLevel, DigitalPin, Gpio};

    #[test]
    fn gpio_uses_one_semantic_capability() {
        let gpio = Gpio::test((1.2, 3.4));
        assert_eq!(gpio.read_analog(AnalogPin::ADC1).voltage().as_volts(), 1.2);
        assert_eq!(gpio.read_analog(AnalogPin::ADC2).voltage().as_volts(), 3.4);
        assert_eq!(gpio.test.analog_pair_calls.get(), 2);
        assert_eq!(gpio.test.last_pin.get(), 8);
    }

    #[test]
    fn digital_gpio_uses_typed_ppm_pin_and_output_level() {
        let gpio = Gpio::test((0.0, 0.0));

        assert!(gpio.configure_output(DigitalPin::PPM));
        assert!(gpio.write(DigitalPin::PPM, DigitalOutputLevel::High));

        assert_eq!(gpio.test.output_mode_calls.get(), 1);
        assert_eq!(gpio.test.write_calls.get(), 1);
        assert_eq!(gpio.test.last_pin.get(), 12);
        assert_eq!(
            gpio.test.last_output_level.get(),
            Some(DigitalOutputLevel::High)
        );
    }
}
