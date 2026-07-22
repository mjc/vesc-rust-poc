//! Typed GPIO access for package code.

use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

use vescpkg_rs_sys::VescPin;
#[cfg(not(test))]
use vescpkg_rs_sys::VescPinMode;

use crate::types::AdcVoltage;
use crate::units::Voltage;

fn adc_voltage_from_firmware(raw: f32) -> AdcVoltage {
    AdcVoltage::new(Voltage::from_volts(raw))
}

static GPIO_LEASES: AtomicU32 = AtomicU32::new(0);

/// Firmware GPIO configuration modes from `VESC_PIN_MODE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioMode {
    /// Digital input without an internal pull resistor.
    Input,
    /// Digital input with an internal pull-up.
    InputPullUp,
    /// Digital input with an internal pull-down.
    InputPullDown,
    /// Push-pull digital output.
    Output,
    /// Open-drain output without a pull resistor.
    OpenDrain,
    /// Open-drain output with an internal pull-up.
    OpenDrainPullUp,
    /// Open-drain output with an internal pull-down.
    OpenDrainPullDown,
    /// Analog input.
    Analog,
}

impl GpioMode {
    #[cfg(not(test))]
    const fn firmware_mode(self) -> VescPinMode {
        VescPinMode(match self {
            Self::Input => 0,
            Self::InputPullUp => 1,
            Self::InputPullDown => 2,
            Self::Output => 3,
            Self::OpenDrain => 4,
            Self::OpenDrainPullUp => 5,
            Self::OpenDrainPullDown => 6,
            Self::Analog => 7,
        })
    }

    const fn accepts_digital_read(self) -> bool {
        matches!(self, Self::Input | Self::InputPullUp | Self::InputPullDown)
    }

    const fn accepts_write(self) -> bool {
        matches!(
            self,
            Self::Output | Self::OpenDrain | Self::OpenDrainPullUp | Self::OpenDrainPullDown
        )
    }
}

/// Error returned by leased GPIO operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioError {
    /// Another package-owned lease already holds the pin.
    Busy,
    /// The operation does not match the currently configured mode.
    WrongMode,
    /// Firmware rejected a mode transition or digital operation.
    FirmwareRejected,
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

    /// Acquire exclusive ownership of a digital pin.
    pub fn acquire_digital(&self, pin: DigitalPin) -> Result<DigitalGpioLease<'_>, GpioError> {
        let token = claim(pin.0.0)?;
        Ok(DigitalGpioLease {
            gpio: self,
            pin,
            token,
            mode: Cell::new(None),
        })
    }

    /// Acquire exclusive ownership of an analog pin.
    pub fn acquire_analog(&self, pin: AnalogPin) -> Result<AnalogGpioLease<'_>, GpioError> {
        let token = claim(pin.0)?;
        Ok(AnalogGpioLease {
            gpio: self,
            pin,
            token,
            mode: Cell::new(None),
        })
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
            unsafe { crate::ffi::io_set_mode(pin.0, VescPinMode(DIGITAL_OUTPUT_MODE)) }
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

/// An exclusive, non-cloneable digital GPIO lease.
pub struct DigitalGpioLease<'a> {
    gpio: &'a Gpio,
    pin: DigitalPin,
    token: u32,
    mode: Cell<Option<GpioMode>>,
}

impl DigitalGpioLease<'_> {
    /// Change the firmware mode for the leased pin.
    pub fn set_mode(&self, mode: GpioMode) -> Result<(), GpioError> {
        if mode == GpioMode::Analog {
            return Err(GpioError::WrongMode);
        }
        if !set_mode(self.gpio, self.pin.0, mode) {
            return Err(GpioError::FirmwareRejected);
        }
        self.mode.set(Some(mode));
        Ok(())
    }

    /// Read a configured digital input.
    pub fn read(&self) -> Result<bool, GpioError> {
        if !self.mode.get().is_some_and(GpioMode::accepts_digital_read) {
            return Err(GpioError::WrongMode);
        }
        Ok(read(self.gpio, self.pin.0))
    }

    /// Drive a configured digital output.
    pub fn write(&self, level: DigitalOutputLevel) -> Result<(), GpioError> {
        if !self.mode.get().is_some_and(GpioMode::accepts_write) {
            return Err(GpioError::WrongMode);
        }
        if write(self.gpio, self.pin.0, level) {
            Ok(())
        } else {
            Err(GpioError::FirmwareRejected)
        }
    }
}

impl Drop for DigitalGpioLease<'_> {
    fn drop(&mut self) {
        GPIO_LEASES.fetch_and(!self.token, Ordering::Release);
    }
}

/// An exclusive, non-cloneable analog GPIO lease.
pub struct AnalogGpioLease<'a> {
    gpio: &'a Gpio,
    pin: AnalogPin,
    token: u32,
    mode: Cell<Option<GpioMode>>,
}

impl AnalogGpioLease<'_> {
    /// Configure the leased pin for analog input.
    pub fn set_mode(&self, mode: GpioMode) -> Result<(), GpioError> {
        if mode != GpioMode::Analog {
            return Err(GpioError::WrongMode);
        }
        if !set_mode(self.gpio, self.pin.firmware_pin(), mode) {
            return Err(GpioError::FirmwareRejected);
        }
        self.mode.set(Some(mode));
        Ok(())
    }

    /// Read the typed analog voltage after selecting analog mode.
    pub fn read(&self) -> Result<AdcVoltage, GpioError> {
        if self.mode.get() != Some(GpioMode::Analog) {
            return Err(GpioError::WrongMode);
        }
        Ok(self.gpio.read_analog(self.pin))
    }
}

impl Drop for AnalogGpioLease<'_> {
    fn drop(&mut self) {
        GPIO_LEASES.fetch_and(!self.token, Ordering::Release);
    }
}

fn claim(pin: i32) -> Result<u32, GpioError> {
    let Some(token) = (pin >= 0)
        .then_some(1_u32.checked_shl(pin as u32))
        .flatten()
    else {
        return Err(GpioError::FirmwareRejected);
    };
    GPIO_LEASES
        .fetch_update(Ordering::Acquire, Ordering::Relaxed, |used| {
            (used & token == 0).then_some(used | token)
        })
        .map(|_| token)
        .map_err(|_| GpioError::Busy)
}

#[cfg(all(feature = "test-support", not(test)))]
pub(crate) fn reset_leases() {
    GPIO_LEASES.store(0, Ordering::Release);
}

fn set_mode(_gpio: &Gpio, pin: VescPin, mode: GpioMode) -> bool {
    #[cfg(test)]
    {
        let _ = (_gpio, pin, mode);
        true
    }
    #[cfg(not(test))]
    {
        unsafe { crate::ffi::io_set_mode(pin, mode.firmware_mode()) }
    }
}

fn read(_gpio: &Gpio, pin: VescPin) -> bool {
    #[cfg(test)]
    {
        let _ = _gpio;
        let _ = pin;
        false
    }
    #[cfg(not(test))]
    {
        unsafe { crate::ffi::io_read(pin) }
    }
}

fn write(_gpio: &Gpio, pin: VescPin, level: DigitalOutputLevel) -> bool {
    #[cfg(test)]
    {
        let _ = (_gpio, pin, level);
        true
    }
    #[cfg(not(test))]
    {
        unsafe { crate::ffi::io_write(pin, level.firmware_level()) }
    }
}

#[cfg(not(test))]
// VESC's `io_set_mode` ABI value for a push-pull digital output.
const DIGITAL_OUTPUT_MODE: i32 = 3;

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

        assert_eq!(DigitalOutputLevel::Low.firmware_level(), 0);
        assert_eq!(DigitalOutputLevel::High.firmware_level(), 1);

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
