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

    /// Resolve a firmware ADC enum value without exposing the raw pin.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            7 => Some(Self::ADC1),
            8 => Some(Self::ADC2),
            _ => None,
        }
    }

    const fn firmware_pin(self) -> VescPin {
        VescPin(self.0)
    }
}

/// A firmware digital GPIO pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct DigitalPin(VescPin);

impl DigitalPin {
    /// VESC communication receive pin.
    pub const COMM_RX: Self = Self(VescPin(0));
    /// VESC communication transmit pin.
    pub const COMM_TX: Self = Self(VescPin(1));
    /// VESC SWD data pin.
    pub const SWDIO: Self = Self(VescPin(2));
    /// VESC SWD clock pin.
    pub const SWCLK: Self = Self(VescPin(3));
    /// VESC Hall sensor 1 pin.
    pub const HALL1: Self = Self(VescPin(4));
    /// VESC Hall sensor 2 pin.
    pub const HALL2: Self = Self(VescPin(5));
    /// VESC Hall sensor 3 pin.
    pub const HALL3: Self = Self(VescPin(6));
    /// VESC Hall sensor 4 pin.
    pub const HALL4: Self = Self(VescPin(9));
    /// VESC Hall sensor 5 pin.
    pub const HALL5: Self = Self(VescPin(10));
    /// VESC Hall sensor 6 pin.
    pub const HALL6: Self = Self(VescPin(11));
    /// VESC's Servo/PPM pin.
    pub const PPM: Self = Self(VescPin(12));
    /// VESC hardware pin 1.
    pub const HW_1: Self = Self(VescPin(13));
    /// VESC hardware pin 2.
    pub const HW_2: Self = Self(VescPin(14));

    /// Resolve a pinned VESC digital-pin enum value without exposing the raw pin.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::COMM_RX),
            1 => Some(Self::COMM_TX),
            2 => Some(Self::SWDIO),
            3 => Some(Self::SWCLK),
            4 => Some(Self::HALL1),
            5 => Some(Self::HALL2),
            6 => Some(Self::HALL3),
            9 => Some(Self::HALL4),
            10 => Some(Self::HALL5),
            11 => Some(Self::HALL6),
            12 => Some(Self::PPM),
            13 => Some(Self::HW_1),
            14 => Some(Self::HW_2),
            _ => None,
        }
    }

    pub(crate) const fn raw(self) -> VescPin {
        self.0
    }
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

#[allow(clippy::used_underscore_binding)]
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

#[allow(clippy::used_underscore_binding)]
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

#[allow(clippy::used_underscore_binding)]
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
    use super::{AnalogPin, DigitalOutputLevel, DigitalPin, Gpio, VescPin};

    #[test]
    fn digital_pin_constants_match_the_pinned_vesc_enum() {
        let pins = [
            (DigitalPin::COMM_RX, 0),
            (DigitalPin::COMM_TX, 1),
            (DigitalPin::SWDIO, 2),
            (DigitalPin::SWCLK, 3),
            (DigitalPin::HALL1, 4),
            (DigitalPin::HALL2, 5),
            (DigitalPin::HALL3, 6),
            (DigitalPin::HALL4, 9),
            (DigitalPin::HALL5, 10),
            (DigitalPin::HALL6, 11),
            (DigitalPin::PPM, 12),
            (DigitalPin::HW_1, 13),
            (DigitalPin::HW_2, 14),
        ];
        for (pin, raw) in pins {
            assert_eq!(pin, DigitalPin(VescPin(raw)));
        }
    }

    #[test]
    fn pin_lookup_rejects_unassigned_enum_values() {
        assert_eq!(DigitalPin::from_raw(12), Some(DigitalPin::PPM));
        assert_eq!(DigitalPin::from_raw(7), None);
        assert_eq!(AnalogPin::from_raw(8), Some(AnalogPin::ADC2));
        assert_eq!(AnalogPin::from_raw(12), None);
    }

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
