//! Exclusive terminal command registration with scoped argument views.
#![allow(
    clippy::missing_errors_doc,
    reason = "error variants document failures"
)]

use core::ffi::{CStr, c_char};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

static TERMINAL_OWNED: AtomicBool = AtomicBool::new(false);
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn disable_callback_dispatch() {
    TERMINAL_ACTIVE.store(false, Ordering::Release);
}

/// Failure returned by terminal registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalError {
    /// The terminal callback slots are unavailable.
    Unavailable,
    /// Another package currently owns the terminal callback.
    Busy,
}

impl_error!(TerminalError {
    Unavailable => "terminal callback slot is unavailable",
    Busy => "terminal callback slot is already owned",
});

/// Safe callback behavior for one terminal command.
pub trait TerminalHandler {
    /// Handle the command's scoped argument iterator.
    fn run(args: TerminalArgs<'_>);
}

/// Scoped terminal argument iterator that reports invalid UTF-8 per argument.
pub struct TerminalArgs<'a> {
    argv: *const *const c_char,
    index: usize,
    length: usize,
    _lifetime: PhantomData<&'a str>,
}

impl<'a> Iterator for TerminalArgs<'a> {
    type Item = Result<&'a str, core::str::Utf8Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.length || self.argv.is_null() {
            return None;
        }
        let pointer = unsafe { *self.argv.add(self.index) };
        self.index += 1;
        if pointer.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(pointer) }.to_str())
    }
}

/// Optional terminal capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Terminal;

/// Exclusive terminal callback registration.
pub struct TerminalRegistration<'a, H: TerminalHandler> {
    _handler: PhantomData<H>,
    _borrowed_strings: PhantomData<crate::FirmwareStr<'a>>,
}

impl Terminal {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Register one command while retaining its metadata and callback owner.
    pub fn register<'a, H: TerminalHandler>(
        &'a self,
        command: crate::FirmwareStr<'a>,
        help: crate::FirmwareStr<'a>,
        arg_names: crate::FirmwareStr<'a>,
    ) -> Result<TerminalRegistration<'a, H>, TerminalError> {
        if TERMINAL_OWNED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(TerminalError::Busy);
        }
        let registered = unsafe {
            crate::ffi::terminal_register_command_callback(
                command.as_ptr(),
                help.as_ptr(),
                arg_names.as_ptr(),
                callback::<H>,
            )
        };
        if !registered {
            TERMINAL_OWNED.store(false, Ordering::Release);
            return Err(TerminalError::Unavailable);
        }
        TERMINAL_ACTIVE.store(true, Ordering::Release);
        Ok(TerminalRegistration {
            _handler: PhantomData,
            _borrowed_strings: PhantomData,
        })
    }
}

impl<H: TerminalHandler> Drop for TerminalRegistration<'_, H> {
    fn drop(&mut self) {
        TERMINAL_ACTIVE.store(false, Ordering::Release);
        // A failed unregister leaves the provider callback live, so fail closed
        // instead of allowing a later registration to replace it.
        if unsafe { crate::ffi::terminal_unregister_callback(callback::<H>) } {
            TERMINAL_OWNED.store(false, Ordering::Release);
        }
    }
}

unsafe extern "C" fn callback<H: TerminalHandler>(arg_count: i32, argv: *const *const c_char) {
    if !TERMINAL_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    if arg_count < 0 {
        return;
    }
    let Ok(length) = usize::try_from(arg_count) else {
        return;
    };
    H::run(TerminalArgs {
        argv,
        index: 0,
        length,
        _lifetime: PhantomData,
    });
}

impl crate::Firmware {
    /// Return the optional terminal capability handle.
    #[must_use]
    pub fn terminal(&self) -> Terminal {
        Terminal::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional terminal capability handle.
    #[must_use]
    pub fn terminal(&self) -> Terminal {
        Terminal::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{TERMINAL_ACTIVE, TerminalArgs, TerminalHandler, callback};
    use core::ffi::c_char;
    use core::marker::PhantomData;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    struct Handler;

    impl TerminalHandler for Handler {
        fn run(_args: TerminalArgs<'_>) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn late_terminal_callback_after_drop_fails_closed() {
        CALLS.store(0, Ordering::Relaxed);
        TERMINAL_ACTIVE.store(true, Ordering::Release);
        unsafe { callback::<Handler>(0, core::ptr::null()) };
        TERMINAL_ACTIVE.store(false, Ordering::Release);
        unsafe { callback::<Handler>(0, core::ptr::null()) };
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn invalid_utf8_argument_does_not_hide_later_arguments() {
        let invalid = [0xff_u8, 0];
        let valid = b"valid\0";
        let argument_pointers = [
            invalid.as_ptr().cast::<c_char>(),
            valid.as_ptr().cast::<c_char>(),
        ];
        let mut terminal_args = TerminalArgs {
            argv: argument_pointers.as_ptr(),
            index: 0,
            length: argument_pointers.len(),
            _lifetime: PhantomData,
        };

        assert!(terminal_args.next().unwrap().is_err());
        assert_eq!(terminal_args.next().unwrap().unwrap(), "valid");
    }
}
