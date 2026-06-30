//! GPIO helpers built on the firmware abstract IO table slots.

use vescpkg_rs_sys::{VescPin, VescPinMode};

/// Abstract GPIO operations backed by firmware slots.
pub trait GpioBindings {
    /// Configure the pin mode.
    fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool;
    /// Drive the pin high or low.
    fn write(&self, pin: VescPin, level: bool) -> bool;
    /// Read the current pin state.
    fn read(&self, pin: VescPin) -> bool;
}

#[cfg(not(test))]
/// GPIO binding implementation that forwards to the live firmware ABI.
pub struct RealGpioBindings;

#[cfg(not(test))]
impl GpioBindings for RealGpioBindings {
    fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool {
        unsafe { vescpkg_rs_sys::raw::io_set_mode(pin, mode) }
    }

    fn write(&self, pin: VescPin, level: bool) -> bool {
        unsafe { vescpkg_rs_sys::raw::io_write(pin, i32::from(level)) }
    }

    fn read(&self, pin: VescPin) -> bool {
        unsafe { vescpkg_rs_sys::raw::io_read(pin) }
    }
}

/// High-level GPIO convenience API built on a binding implementation.
pub struct GpioApi<B> {
    bindings: B,
}

impl<B: GpioBindings> GpioApi<B> {
    /// Construct a new GPIO API wrapper.
    pub fn new(bindings: B) -> Self {
        Self { bindings }
    }

    /// Configure the pin mode.
    pub fn set_mode(&self, pin: VescPin, mode: VescPinMode) -> bool {
        self.bindings.set_mode(pin, mode)
    }

    /// Drive the pin high or low.
    pub fn write(&self, pin: VescPin, level: bool) -> bool {
        self.bindings.write(pin, level)
    }

    /// Read the current pin state.
    pub fn read(&self, pin: VescPin) -> bool {
        self.bindings.read(pin)
    }
}

#[cfg(any(test, feature = "test-support"))]
/// GPIO fake binding helpers exported for tests.
pub mod test_support {
    use super::GpioBindings;
    use core::cell::Cell;
    use vescpkg_rs_sys::{VescPin, VescPinMode};

    /// Fake GPIO binding implementation used by GPIO unit tests.
    pub struct FakeGpioBindings {
        /// Number of mode calls observed.
        pub mode_calls: Cell<usize>,
        /// Number of write calls observed.
        pub write_calls: Cell<usize>,
        /// Number of read calls observed.
        pub read_calls: Cell<usize>,
        /// Last pin passed to any GPIO call.
        pub last_pin: Cell<i32>,
        /// Last mode value passed to mode configuration.
        pub last_mode: Cell<i32>,
        /// Last output level passed to write.
        pub last_level: Cell<i32>,
    }

    impl Default for FakeGpioBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeGpioBindings {
        /// Creates a fake GPIO binding recorder with zeroed counters.
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
        use vescpkg_rs_sys::{VescPin, VescPinMode};

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
