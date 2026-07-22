//! C ABI function-pointer signatures shared by the Express runtime surface.

use core::ffi::{c_char, c_void};

/// Express system tick value (`systime_t`).
pub type Systime = u32;
/// Express LispBM context identifier (`lbm_cid`).
pub type LbmCid = u32;
/// Express LispBM value (`lbm_value`).
pub type LbmValue = u32;
/// Express LispBM unsigned integer (`lbm_uint`).
pub type LbmUint = u32;
/// Express LispBM signed integer (`lbm_int`).
pub type LbmInt = i32;
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
/// LispBM extension handler ABI accepted by `lbm_add_extension`.
pub type ExtensionHandler = unsafe extern "C" fn(*mut LbmValue, LbmValue) -> LbmValue;
/// `lbm_add_extension` function-pointer ABI.
pub type AddExtension = unsafe extern "C" fn(*mut c_char, ExtensionHandler) -> bool;
/// `lbm_set_error_reason` function-pointer ABI.
pub type SetErrorReason = unsafe extern "C" fn(*mut c_char) -> i32;
/// `lbm_add_symbol_const` function-pointer ABI.
pub type AddSymbolConst = unsafe extern "C" fn(*mut c_char, *mut LbmUint) -> i32;
/// `lbm_get_symbol_by_name` function-pointer ABI.
pub type GetSymbolByName = unsafe extern "C" fn(*mut c_char, *mut LbmUint) -> i32;
/// `lbm_create_byte_array` function-pointer ABI.
pub type CreateByteArray = unsafe extern "C" fn(*mut LbmValue, LbmUint) -> bool;
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

/// `lbm_cons` function-pointer ABI.
pub type LbmCons = unsafe extern "C" fn(LbmValue, LbmValue) -> LbmValue;
/// `lbm_car` function-pointer ABI.
pub type LbmCar = unsafe extern "C" fn(LbmValue) -> LbmValue;
/// `lbm_cdr` function-pointer ABI.
pub type LbmCdr = unsafe extern "C" fn(LbmValue) -> LbmValue;
/// `lbm_list_destructive_reverse` function-pointer ABI.
pub type LbmListDestructiveReverse = unsafe extern "C" fn(LbmValue) -> LbmValue;
/// `lbm_enc_i` function-pointer ABI.
pub type LbmEncI = unsafe extern "C" fn(LbmInt) -> LbmValue;
/// `lbm_enc_u` function-pointer ABI.
pub type LbmEncU = unsafe extern "C" fn(LbmUint) -> LbmValue;
/// `lbm_enc_char` function-pointer ABI.
pub type LbmEncChar = unsafe extern "C" fn(u8) -> LbmValue;
/// `lbm_enc_float` function-pointer ABI.
pub type LbmEncFloat = unsafe extern "C" fn(f32) -> LbmValue;
/// `lbm_enc_u32` function-pointer ABI.
pub type LbmEncU32 = unsafe extern "C" fn(u32) -> LbmValue;
/// `lbm_enc_i32` function-pointer ABI.
pub type LbmEncI32 = unsafe extern "C" fn(i32) -> LbmValue;
/// `lbm_enc_sym` function-pointer ABI.
pub type LbmEncSym = unsafe extern "C" fn(LbmUint) -> LbmValue;
/// `lbm_dec_as_float` function-pointer ABI.
pub type LbmDecAsFloat = unsafe extern "C" fn(LbmValue) -> f32;
/// `lbm_dec_as_u32` function-pointer ABI.
pub type LbmDecAsU32 = unsafe extern "C" fn(LbmValue) -> u32;
/// `lbm_dec_as_i32` function-pointer ABI.
pub type LbmDecAsI32 = unsafe extern "C" fn(LbmValue) -> i32;
/// `lbm_dec_char` function-pointer ABI.
pub type LbmDecChar = unsafe extern "C" fn(LbmValue) -> u8;
/// `lbm_dec_sym` function-pointer ABI.
pub type LbmDecSym = unsafe extern "C" fn(LbmValue) -> LbmUint;
/// `lbm_is_byte_array` function-pointer ABI.
pub type LbmIsByteArray = unsafe extern "C" fn(LbmValue) -> bool;
/// `lbm_is_cons` function-pointer ABI.
pub type LbmIsCons = unsafe extern "C" fn(LbmValue) -> bool;
/// `lbm_is_number` function-pointer ABI.
pub type LbmIsNumber = unsafe extern "C" fn(LbmValue) -> bool;
/// `lbm_is_char` function-pointer ABI.
pub type LbmIsChar = unsafe extern "C" fn(LbmValue) -> bool;
/// `lbm_is_symbol` function-pointer ABI.
pub type LbmIsSymbol = unsafe extern "C" fn(LbmValue) -> bool;
/// `lbm_block_ctx_from_extension` function-pointer ABI.
pub type LbmBlockCtxFromExtension = unsafe extern "C" fn();
/// `lbm_unblock_ctx_unboxed` function-pointer ABI.
pub type LbmUnblockCtxUnboxed = unsafe extern "C" fn(LbmCid, LbmValue) -> bool;
/// `lbm_get_current_cid` function-pointer ABI.
pub type LbmGetCurrentCid = unsafe extern "C" fn() -> LbmCid;
/// `lbm_send_message` function-pointer ABI.
pub type LbmSendMessage = unsafe extern "C" fn(LbmCid, LbmValue) -> i32;
/// `lbm_pause_eval_with_gc` function-pointer ABI.
pub type LbmPauseEvalWithGc = unsafe extern "C" fn(u32);
/// `lbm_continue_eval` function-pointer ABI.
pub type LbmContinueEval = unsafe extern "C" fn();
/// `lbm_eval_is_paused` function-pointer ABI.
pub type LbmEvalIsPaused = unsafe extern "C" fn() -> bool;
