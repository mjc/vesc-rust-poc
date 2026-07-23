//! Usage-shaped GPIO wiring for a small display-style bus.

use vescpkg_rs::{DigitalGpioLease, DigitalOutputLevel, DigitalPin, Gpio, GpioError, GpioMode};

/// Width of the SSD1306-compatible framebuffer used by the official example.
pub const SSD1306_WIDTH: usize = 128;
/// Height of the SSD1306-compatible framebuffer used by the official example.
pub const SSD1306_HEIGHT: usize = 64;
/// Serialized framebuffer size, including the one-byte command prefix used by VESC.
pub const SSD1306_FRAME_BYTES: usize = 1 + (SSD1306_WIDTH * SSD1306_HEIGHT / 8);

/// A bounded, allocation-free SSD1306 framebuffer.
///
/// The layout matches the vendored VESC `examples/ssd1306` port: byte zero is
/// the command prefix and the remaining bytes are page-organized display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ssd1306Frame {
    bytes: [u8; SSD1306_FRAME_BYTES],
}

impl Default for Ssd1306Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Ssd1306Frame {
    /// Construct an empty framebuffer with the VESC command prefix initialized.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; SSD1306_FRAME_BYTES],
        }
    }

    /// Clear all pixels while retaining the command prefix byte.
    pub fn clear(&mut self) {
        self.bytes[1..].fill(0);
    }

    /// Return the serialized frame suitable for a transport callback.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SSD1306_FRAME_BYTES] {
        &self.bytes
    }

    /// Set one pixel when it lies within the 128×64 display bounds.
    pub fn set_pixel(&mut self, x: i16, y: i16) {
        usize::try_from(x)
            .ok()
            .zip(usize::try_from(y).ok())
            .filter(|&(x, y)| x < SSD1306_WIDTH && y < SSD1306_HEIGHT)
            .map(|(x, y)| {
                let byte = 1 + x + (y / 8) * SSD1306_WIDTH;
                self.bytes[byte] |= 1 << (y % 8);
            });
    }

    /// Draw a clipped line using the same integer algorithm as the official example.
    pub fn draw_line(&mut self, start: (i16, i16), end: (i16, i16)) {
        let (mut x0, mut y0) = (i32::from(start.0), i32::from(start.1));
        let (x1, y1) = (i32::from(end.0), i32::from(end.1));
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.set_pixel(x0 as i16, y0 as i16);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled = error * 2;
            if doubled >= dy {
                if x0 == x1 {
                    break;
                }
                error += dy;
                x0 += sx;
            }
            if doubled <= dx {
                if y0 == y1 {
                    break;
                }
                error += dx;
                y0 += sy;
            }
        }
    }
}

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
    let clock = gpio.acquire_digital(DigitalPin::HW_2)?;
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
    use super::{SSD1306_FRAME_BYTES, Ssd1306Frame, open_display_bus};

    #[test]
    fn ssd1306_frame_matches_official_page_layout() {
        let mut frame = Ssd1306Frame::new();
        frame.set_pixel(0, 0);
        frame.set_pixel(127, 63);

        assert_eq!(frame.as_bytes()[0], 0);
        assert_eq!(frame.as_bytes()[1], 1);
        assert_eq!(frame.as_bytes()[1 + 127 + 7 * 128], 0x80);
        assert_eq!(frame.as_bytes().len(), SSD1306_FRAME_BYTES);
    }

    #[test]
    fn ssd1306_frame_clips_and_draws_lines_without_allocation() {
        let mut frame = Ssd1306Frame::new();
        frame.draw_line((-2, 0), (2, 0));

        assert_eq!(frame.as_bytes()[1], 1);
        assert_eq!(frame.as_bytes()[3], 1);
        frame.clear();
        assert!(frame.as_bytes()[1..].iter().all(|byte| *byte == 0));
    }

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
