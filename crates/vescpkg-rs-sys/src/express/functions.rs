//! C ABI function-pointer signatures shared by the Express runtime surface.

use core::ffi::{c_char, c_void};

/// Express system tick value (`systime_t`).
pub type Systime = u32;
/// Express LispBM context identifier (`lbm_cid`).
pub type LbmCid = u32;
/// Firmware-owned thread handle (`lib_thread`).
pub type LibThread = *mut c_void;
/// Firmware-owned mutex handle (`lib_mutex`).
pub type LibMutex = *mut c_void;
/// Firmware-owned semaphore handle (`lib_semaphore`).
pub type LibSemaphore = *mut c_void;

/// `sleep_ms` function-pointer ABI.
pub type SleepMs = unsafe extern "C" fn(u32);
/// `sleep_us` function-pointer ABI.
pub type SleepUs = unsafe extern "C" fn(u32);
/// `system_time` function-pointer ABI.
pub type SystemTime = unsafe extern "C" fn() -> f32;
/// `ts_to_age_s` function-pointer ABI.
pub type TsToAgeS = unsafe extern "C" fn(Systime) -> f32;
/// `system_time_ticks` function-pointer ABI.
pub type SystemTimeTicks = unsafe extern "C" fn() -> Systime;
/// `sleep_ticks` function-pointer ABI.
pub type SleepTicks = unsafe extern "C" fn(Systime);
/// `timer_time_now` function-pointer ABI.
pub type TimerTimeNow = unsafe extern "C" fn() -> u32;
/// `timer_seconds_elapsed_since` function-pointer ABI.
pub type TimerSecondsElapsedSince = unsafe extern "C" fn(u32) -> f32;
/// `timer_sleep` function-pointer ABI.
pub type TimerSleep = unsafe extern "C" fn(f32);
/// `malloc` function-pointer ABI.
pub type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
/// `free` function-pointer ABI.
pub type Free = unsafe extern "C" fn(*mut c_void);
/// Callback ABI accepted by `spawn`.
pub type SpawnFunction = unsafe extern "C" fn(*mut c_void);
/// `spawn` function-pointer ABI.
pub type Spawn =
    unsafe extern "C" fn(SpawnFunction, usize, *const c_char, *mut c_void) -> LibThread;
/// `request_terminate` function-pointer ABI.
pub type RequestTerminate = unsafe extern "C" fn(LibThread);
/// `should_terminate` function-pointer ABI.
pub type ShouldTerminate = unsafe extern "C" fn() -> bool;
/// `thread_set_priority` function-pointer ABI.
pub type ThreadSetPriority = unsafe extern "C" fn(i32);
/// `get_arg` function-pointer ABI.
pub type GetArg = unsafe extern "C" fn(u32) -> *mut *mut c_void;
/// `mutex_create` function-pointer ABI.
pub type MutexCreate = unsafe extern "C" fn() -> LibMutex;
/// `mutex_lock` function-pointer ABI.
pub type MutexLock = unsafe extern "C" fn(LibMutex);
/// `mutex_unlock` function-pointer ABI.
pub type MutexUnlock = unsafe extern "C" fn(LibMutex);
/// `sem_create` function-pointer ABI.
pub type SemaphoreCreate = unsafe extern "C" fn() -> LibSemaphore;
/// `sem_wait` function-pointer ABI.
pub type SemaphoreWait = unsafe extern "C" fn(LibSemaphore);
/// `sem_signal` function-pointer ABI.
pub type SemaphoreSignal = unsafe extern "C" fn(LibSemaphore);
/// `sem_wait_to` function-pointer ABI.
pub type SemaphoreWaitTo = unsafe extern "C" fn(LibSemaphore, Systime) -> bool;
/// `sem_reset` function-pointer ABI.
pub type SemaphoreReset = unsafe extern "C" fn(LibSemaphore);
