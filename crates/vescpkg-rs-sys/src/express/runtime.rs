//! Constructor-gated shared Express runtime operations.

use super::functions::{
    ShouldTerminate, SleepMs, SleepTicks, SleepUs, SystemTime, SystemTimeTicks, ThreadSetPriority,
    TimerSecondsElapsedSince, TimerSleep, TimerTimeNow, TsToAgeS,
};
use super::{ExpressCallError, ExpressInterface, ExpressSlot, ExpressTarget};

/// Shared Express clock, sleep, and thread-control facade.
///
/// STM32-only motor, CAN, pad, and peripheral operations are intentionally not
/// present on this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressRuntime<'a> {
    interface: ExpressInterface<'a>,
}

impl<'a> ExpressRuntime<'a> {
    /// Adopt a validated Express table as the shared runtime provider.
    ///
    /// # Safety
    ///
    /// The table must be the live v1 Express firmware table on a matching
    /// 32-bit target. Each function slot must retain the exact C ABI declared
    /// by the pinned Express header for the lifetime of this runtime.
    pub const unsafe fn from_interface(interface: ExpressInterface<'a>) -> Self {
        Self { interface }
    }

    /// Load the fixed target table and adopt it as the runtime provider.
    ///
    /// # Safety
    ///
    /// See [`ExpressInterface::from_target`].
    pub unsafe fn from_target(target: ExpressTarget) -> Result<Self, super::ExpressLoadError> {
        let interface = unsafe { ExpressInterface::from_target(target)? };
        Ok(Self { interface })
    }

    /// Return the underlying validated Express interface view.
    pub const fn interface(self) -> ExpressInterface<'a> {
        self.interface
    }

    /// Sleep for whole milliseconds.
    pub fn sleep_ms(self, milliseconds: u32) -> Result<(), ExpressCallError> {
        let sleep: SleepMs = unsafe { self.interface.function(ExpressSlot::SleepMs) }?;
        unsafe { sleep(milliseconds) };
        Ok(())
    }

    /// Sleep for whole microseconds.
    pub fn sleep_us(self, microseconds: u32) -> Result<(), ExpressCallError> {
        let sleep: SleepUs = unsafe { self.interface.function(ExpressSlot::SleepUs) }?;
        unsafe { sleep(microseconds) };
        Ok(())
    }

    /// Return firmware uptime in seconds.
    pub fn system_time(self) -> Result<f32, ExpressCallError> {
        let clock: SystemTime = unsafe { self.interface.function(ExpressSlot::SystemTime) }?;
        Ok(unsafe { clock() })
    }

    /// Convert a firmware timestamp to age in seconds.
    pub fn timestamp_age(self, timestamp: u32) -> Result<f32, ExpressCallError> {
        let age: TsToAgeS = unsafe { self.interface.function(ExpressSlot::TsToAgeS) }?;
        Ok(unsafe { age(timestamp) })
    }

    /// Return firmware uptime in system ticks.
    pub fn system_time_ticks(self) -> Result<u32, ExpressCallError> {
        let ticks: SystemTimeTicks =
            unsafe { self.interface.function(ExpressSlot::SystemTimeTicks) }?;
        Ok(unsafe { ticks() })
    }

    /// Sleep for firmware system ticks.
    pub fn sleep_ticks(self, ticks: u32) -> Result<(), ExpressCallError> {
        let sleep: SleepTicks = unsafe { self.interface.function(ExpressSlot::SleepTicks) }?;
        unsafe { sleep(ticks) };
        Ok(())
    }

    /// Return the high-resolution timer value.
    pub fn timer_time_now(self) -> Result<u32, ExpressCallError> {
        let now: TimerTimeNow = unsafe { self.interface.function(ExpressSlot::TimerTimeNow) }?;
        Ok(unsafe { now() })
    }

    /// Return elapsed high-resolution timer seconds.
    pub fn timer_seconds_elapsed_since(self, timestamp: u32) -> Result<f32, ExpressCallError> {
        let elapsed: TimerSecondsElapsedSince = unsafe {
            self.interface
                .function(ExpressSlot::TimerSecondsElapsedSince)
        }?;
        Ok(unsafe { elapsed(timestamp) })
    }

    /// Busy-sleep for a fractional number of seconds.
    pub fn timer_sleep(self, seconds: f32) -> Result<(), ExpressCallError> {
        let sleep: TimerSleep = unsafe { self.interface.function(ExpressSlot::TimerSleep) }?;
        unsafe { sleep(seconds) };
        Ok(())
    }

    /// Return whether the current thread was asked to terminate.
    pub fn should_terminate(self) -> Result<bool, ExpressCallError> {
        let should_terminate: ShouldTerminate =
            unsafe { self.interface.function(ExpressSlot::ShouldTerminate) }?;
        Ok(unsafe { should_terminate() })
    }

    /// Set the priority of the current thread.
    pub fn thread_set_priority(self, priority: i32) -> Result<(), ExpressCallError> {
        let set_priority: ThreadSetPriority =
            unsafe { self.interface.function(ExpressSlot::ThreadSetPriority) }?;
        unsafe { set_priority(priority) };
        Ok(())
    }
}
