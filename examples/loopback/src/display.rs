//! Usage-shaped GPIO wiring for a small display-style bus.

use vescpkg_rs::{DigitalGpioLease, DigitalOutputLevel, DigitalPin, Gpio, GpioError, GpioMode};

/// A two-wire, exclusive display bus backed by package GPIO leases.
pub struct DisplayBus<'a> {
    data: DigitalGpioLease<'a>,
    clock: DigitalGpioLease<'a>,
}

/// Acquire the two package-owned pins used by the display probe.
///
/// The example deliberately uses named VESC pins rather than raw enum values;
/// a product package should choose pins that match its board wiring.
pub fn open_display_bus(gpio: &Gpio) -> Result<DisplayBus<'_>, GpioError> {
    let data = gpio.acquire_digital(DigitalPin::HW_1)?;
    let clock = match gpio.acquire_digital(DigitalPin::HW_2) {
        Ok(clock) => clock,
        Err(error) => return Err(error),
    };
    data.set_mode(GpioMode::OpenDrainPullUp)?;
    clock.set_mode(GpioMode::OpenDrainPullUp)?;
    Ok(DisplayBus { data, clock })
}

impl DisplayBus<'_> {
    /// Release the bus lines to their idle-high state.
    pub fn idle(&self) -> Result<(), GpioError> {
        self.data.write(DigitalOutputLevel::High)?;
        self.clock.write(DigitalOutputLevel::High)
    }

    /// Emit one clock pulse for a display command byte.
    pub fn pulse_clock(&self) -> Result<(), GpioError> {
        self.clock.write(DigitalOutputLevel::Low)?;
        self.clock.write(DigitalOutputLevel::High)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::open_display_bus;

    #[test]
    fn display_bus_uses_exclusive_gpio_leases() {
        let firmware = vescpkg_rs::test_support::FirmwareTest::new();
        let bus = open_display_bus(firmware.gpio()).expect("display bus leases");
        bus.idle().expect("idle levels");
        bus.pulse_clock().expect("clock pulse");
        drop(bus);
        open_display_bus(firmware.gpio()).expect("leases released on drop");
    }
}
