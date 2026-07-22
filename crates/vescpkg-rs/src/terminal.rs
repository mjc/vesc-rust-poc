//! Exclusive terminal command registration with scoped argument views.

use core::ffi::{CStr, c_char};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

static TERMINAL_OWNED: AtomicBool = AtomicBool::new(false);

/// Failure returned by terminal registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalError {
    /// The terminal callback slots are unavailable.
    Unavailable,
    /// Another package currently owns the terminal callback.
    Busy,
}

/// Safe callback behavior for one terminal command.
pub trait TerminalHandler {
    /// Handle the command's scoped argument iterator.
    fn run(args: TerminalArgs<'_>);
}

/// Scoped terminal argument iterator.
pub struct TerminalArgs<'a> {
    argv: *const *const c_char,
    index: usize,
    length: usize,
    _lifetime: PhantomData<&'a CStr>,
}

impl<'a> Iterator for TerminalArgs<'a> {
    type Item = &'a CStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.length || self.argv.is_null() {
            return None;
        }
        let pointer = unsafe { *self.argv.add(self.index) };
        self.index += 1;
        if pointer.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(pointer) })
    }
}

/// Optional terminal capability handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Terminal;

/// Exclusive terminal callback registration.
pub struct TerminalRegistration<'a, H: TerminalHandler> {
    _handler: PhantomData<H>,
    _borrowed_strings: PhantomData<&'a CStr>,
}

impl Terminal {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Register one command while retaining its metadata and callback owner.
    pub fn register<'a, H: TerminalHandler>(
        &'a self,
        command: &'a CStr,
        help: &'a CStr,
        arg_names: &'a CStr,
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
        Ok(TerminalRegistration {
            _handler: PhantomData,
            _borrowed_strings: PhantomData,
        })
    }
}

impl<H: TerminalHandler> Drop for TerminalRegistration<'_, H> {
    fn drop(&mut self) {
        let _ = unsafe { crate::ffi::terminal_unregister_callback(callback::<H>) };
        TERMINAL_OWNED.store(false, Ordering::Release);
    }
}

unsafe extern "C" fn callback<H: TerminalHandler>(arg_count: i32, argv: *const *const c_char) {
    if arg_count < 0 {
        return;
    }
    H::run(TerminalArgs {
        argv,
        index: 0,
        length: arg_count as usize,
        _lifetime: PhantomData,
    });
}

impl crate::Firmware {
    /// Return the optional terminal capability handle.
    pub fn terminal(&self) -> Terminal {
        Terminal::new()
    }
}

#[cfg(all(feature = "test-support", not(test)))]
impl crate::test_support::FirmwareTest {
    /// Return the optional terminal capability handle.
    pub fn terminal(&self) -> Terminal {
        Terminal::new()
    }
}
