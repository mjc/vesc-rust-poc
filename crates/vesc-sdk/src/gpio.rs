//! GPIO helpers built on the firmware abstract IO table slots.

use vesc_ffi::{VescPin, VescPinMode};

pub trait GpioBindings {
    fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool;
    fn write(&self, pin: VescPin, level: bool) -> bool;
    fn read(&self, pin: VescPin) -> bool;
}

#[cfg(not(test))]
pub struct RealGpioBindings;

#[cfg(not(test))]
impl GpioBindings for RealGpioBindings {
    fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool {
        unsafe { vesc_ffi::raw::io_set_mode(pin, mode) }
    }

    fn write(&self, pin: VescPin, level: bool) -> bool {
        unsafe { vesc_ffi::raw::io_write(pin, i32::from(level)) }
    }

    fn read(&self, pin: VescPin) -> bool {
        unsafe { vesc_ffi::raw::io_read(pin) }
    }
}

pub struct GpioApi<B> {
    bindings: B,
}

impl<B: GpioBindings> GpioApi<B> {
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    pub fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool {
        self.bindings.set_mode(pin, mode)
    }

    pub fn write(&self, pin: VescPin, level: bool) -> bool {
        self.bindings.write(pin, level)
    }

    pub fn read(&self, pin: VescPin) -> bool {
        self.bindings.read(pin)
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use super::GpioBindings;
    use core::cell::Cell;
    use vesc_ffi::{VescPin, VescPinMode};

    pub struct FakeGpioBindings {
        pub mode_calls: Cell<usize>,
        pub write_calls: Cell<usize>,
        pub read_calls: Cell<usize>,
        pub last_pin: Cell<i32>,
        pub last_mode: Cell<i32>,
        pub last_level: Cell<i32>,
    }

    impl Default for FakeGpioBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeGpioBindings {
        pub fn new() -> Self {
            Self {
                mode_calls: Cell::new(0),
                write_calls: Cell::new(0),
                read_calls: Cell::new(0),
                last_pin: Cell::new(0),
                last_mode: Cell::new(0),
                last_level: Cell::new(0),
            }
        }
    }

    impl GpioBindings for FakeGpioBindings {
        fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool {
            self.mode_calls.set(self.mode_calls.get() + 1);
            self.last_pin.set(pin.0);
            self.last_mode.set(mode.0);
            true
        }

        fn write(&self, pin: VescPin, level: bool) -> bool {
            self.write_calls.set(self.write_calls.get() + 1);
            self.last_pin.set(pin.0);
            self.last_level.set(i32::from(level));
            true
        }

        fn read(&self, pin: VescPin) -> bool {
            self.read_calls.set(self.read_calls.get() + 1);
            self.last_pin.set(pin.0);
            false
        }
    }

    #[cfg(test)]
    mod tests {
        use super::FakeGpioBindings;
        use crate::GpioApi;
        use vesc_ffi::{VescPin, VescPinMode};

        #[test]
        fn gpio_api_forwards_through_bindings() {
            let bindings = FakeGpioBindings::new();
            let api = GpioApi::new(bindings);
            let pin = VescPin(42);
            let mode = VescPinMode(1);

            assert!(api.set_mode(pin, mode));
            assert!(api.write(pin, true));
            assert!(!api.read(pin));
        }
    }
}
