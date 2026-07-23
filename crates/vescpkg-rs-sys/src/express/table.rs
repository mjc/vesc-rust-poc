//! Express interface-table shape and slot classification.

use super::types::{EXPRESS_C_IF_VERSION, EXPRESS_IF_SLOT_COUNT, ExpressAddress, ExpressWord};

/// Whether a pinned Express slot is a scalar word or a nullable function slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressSlotKind {
    /// Interface version or LispBM symbol constant stored inline.
    Scalar,
    /// Function pointer represented as a target word and allowed to be null on
    /// older firmware.
    Function,
}

/// A named slot in the pinned Express v1 table.
///
/// The discriminants are part of the Express ABI and intentionally do not
/// share the STM32 slot manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(missing_docs)]
pub enum ExpressSlot {
    IfVersion = 0,
    LbmAddExtension,
    LbmSetErrorReason,
    LbmAddSymbolConst,
    LbmGetSymbolByName,
    LbmBlockCtxFromExtension,
    LbmUnblockCtx,
    LbmUnblockCtxUnboxed,
    LbmGetCurrentCid,
    LbmSendMessage,
    LbmPauseEvalWithGc,
    LbmContinueEval,
    LbmEvalIsPaused,
    LbmCons,
    LbmCar,
    LbmCdr,
    LbmListDestructiveReverse,
    LbmCreateByteArray,
    LbmEncI,
    LbmEncU,
    LbmEncChar,
    LbmEncFloat,
    LbmEncU32,
    LbmEncI32,
    LbmEncSym,
    LbmDecAsFloat,
    LbmDecAsU32,
    LbmDecAsI32,
    LbmDecChar,
    LbmDecStr,
    LbmDecSym,
    LbmIsByteArray,
    LbmIsCons,
    LbmIsNumber,
    LbmIsChar,
    LbmIsSymbol,
    LbmIsSymbolNil,
    LbmIsSymbolTrue,
    LbmEncSymNil,
    LbmEncSymTrue,
    LbmEncSymTerror,
    LbmEncSymEerror,
    LbmEncSymMerror,
    LbmStartFlatten,
    LbmFinishFlatten,
    FCons,
    FSym,
    FI,
    FB,
    FI32,
    FU32,
    FFloat,
    FI64,
    FU64,
    FLbmArray,
    SleepMs,
    SleepUs,
    SystemTime,
    TsToAgeS,
    SystemTimeTicks,
    SleepTicks,
    TimerTimeNow,
    TimerSecondsElapsedSince,
    TimerSleep,
    Printf,
    Malloc,
    Free,
    Spawn,
    RequestTerminate,
    ShouldTerminate,
    ThreadSetPriority,
    GetArg,
    MutexCreate,
    MutexLock,
    MutexUnlock,
    SemCreate,
    SemWait,
    SemSignal,
    SemWaitTo,
    SemReset,
}

impl ExpressSlot {
    /// Every v1 slot in source order.
    pub const ALL: [Self; EXPRESS_IF_SLOT_COUNT] = [
        Self::IfVersion,
        Self::LbmAddExtension,
        Self::LbmSetErrorReason,
        Self::LbmAddSymbolConst,
        Self::LbmGetSymbolByName,
        Self::LbmBlockCtxFromExtension,
        Self::LbmUnblockCtx,
        Self::LbmUnblockCtxUnboxed,
        Self::LbmGetCurrentCid,
        Self::LbmSendMessage,
        Self::LbmPauseEvalWithGc,
        Self::LbmContinueEval,
        Self::LbmEvalIsPaused,
        Self::LbmCons,
        Self::LbmCar,
        Self::LbmCdr,
        Self::LbmListDestructiveReverse,
        Self::LbmCreateByteArray,
        Self::LbmEncI,
        Self::LbmEncU,
        Self::LbmEncChar,
        Self::LbmEncFloat,
        Self::LbmEncU32,
        Self::LbmEncI32,
        Self::LbmEncSym,
        Self::LbmDecAsFloat,
        Self::LbmDecAsU32,
        Self::LbmDecAsI32,
        Self::LbmDecChar,
        Self::LbmDecStr,
        Self::LbmDecSym,
        Self::LbmIsByteArray,
        Self::LbmIsCons,
        Self::LbmIsNumber,
        Self::LbmIsChar,
        Self::LbmIsSymbol,
        Self::LbmIsSymbolNil,
        Self::LbmIsSymbolTrue,
        Self::LbmEncSymNil,
        Self::LbmEncSymTrue,
        Self::LbmEncSymTerror,
        Self::LbmEncSymEerror,
        Self::LbmEncSymMerror,
        Self::LbmStartFlatten,
        Self::LbmFinishFlatten,
        Self::FCons,
        Self::FSym,
        Self::FI,
        Self::FB,
        Self::FI32,
        Self::FU32,
        Self::FFloat,
        Self::FI64,
        Self::FU64,
        Self::FLbmArray,
        Self::SleepMs,
        Self::SleepUs,
        Self::SystemTime,
        Self::TsToAgeS,
        Self::SystemTimeTicks,
        Self::SleepTicks,
        Self::TimerTimeNow,
        Self::TimerSecondsElapsedSince,
        Self::TimerSleep,
        Self::Printf,
        Self::Malloc,
        Self::Free,
        Self::Spawn,
        Self::RequestTerminate,
        Self::ShouldTerminate,
        Self::ThreadSetPriority,
        Self::GetArg,
        Self::MutexCreate,
        Self::MutexLock,
        Self::MutexUnlock,
        Self::SemCreate,
        Self::SemWait,
        Self::SemSignal,
        Self::SemWaitTo,
        Self::SemReset,
    ];

    /// Return the ABI index of this named slot.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Return the originating C declaration name for this slot.
    pub const fn name(self) -> &'static str {
        match self {
            Self::IfVersion => "if_version",
            Self::LbmAddExtension => "lbm_add_extension",
            Self::LbmSetErrorReason => "lbm_set_error_reason",
            Self::LbmAddSymbolConst => "lbm_add_symbol_const",
            Self::LbmGetSymbolByName => "lbm_get_symbol_by_name",
            Self::LbmBlockCtxFromExtension => "lbm_block_ctx_from_extension",
            Self::LbmUnblockCtx => "lbm_unblock_ctx",
            Self::LbmUnblockCtxUnboxed => "lbm_unblock_ctx_unboxed",
            Self::LbmGetCurrentCid => "lbm_get_current_cid",
            Self::LbmSendMessage => "lbm_send_message",
            Self::LbmPauseEvalWithGc => "lbm_pause_eval_with_gc",
            Self::LbmContinueEval => "lbm_continue_eval",
            Self::LbmEvalIsPaused => "lbm_eval_is_paused",
            Self::LbmCons => "lbm_cons",
            Self::LbmCar => "lbm_car",
            Self::LbmCdr => "lbm_cdr",
            Self::LbmListDestructiveReverse => "lbm_list_destructive_reverse",
            Self::LbmCreateByteArray => "lbm_create_byte_array",
            Self::LbmEncI => "lbm_enc_i",
            Self::LbmEncU => "lbm_enc_u",
            Self::LbmEncChar => "lbm_enc_char",
            Self::LbmEncFloat => "lbm_enc_float",
            Self::LbmEncU32 => "lbm_enc_u32",
            Self::LbmEncI32 => "lbm_enc_i32",
            Self::LbmEncSym => "lbm_enc_sym",
            Self::LbmDecAsFloat => "lbm_dec_as_float",
            Self::LbmDecAsU32 => "lbm_dec_as_u32",
            Self::LbmDecAsI32 => "lbm_dec_as_i32",
            Self::LbmDecChar => "lbm_dec_char",
            Self::LbmDecStr => "lbm_dec_str",
            Self::LbmDecSym => "lbm_dec_sym",
            Self::LbmIsByteArray => "lbm_is_byte_array",
            Self::LbmIsCons => "lbm_is_cons",
            Self::LbmIsNumber => "lbm_is_number",
            Self::LbmIsChar => "lbm_is_char",
            Self::LbmIsSymbol => "lbm_is_symbol",
            Self::LbmIsSymbolNil => "lbm_is_symbol_nil",
            Self::LbmIsSymbolTrue => "lbm_is_symbol_true",
            Self::LbmEncSymNil => "lbm_enc_sym_nil",
            Self::LbmEncSymTrue => "lbm_enc_sym_true",
            Self::LbmEncSymTerror => "lbm_enc_sym_terror",
            Self::LbmEncSymEerror => "lbm_enc_sym_eerror",
            Self::LbmEncSymMerror => "lbm_enc_sym_merror",
            Self::LbmStartFlatten => "lbm_start_flatten",
            Self::LbmFinishFlatten => "lbm_finish_flatten",
            Self::FCons => "f_cons",
            Self::FSym => "f_sym",
            Self::FI => "f_i",
            Self::FB => "f_b",
            Self::FI32 => "f_i32",
            Self::FU32 => "f_u32",
            Self::FFloat => "f_float",
            Self::FI64 => "f_i64",
            Self::FU64 => "f_u64",
            Self::FLbmArray => "f_lbm_array",
            Self::SleepMs => "sleep_ms",
            Self::SleepUs => "sleep_us",
            Self::SystemTime => "system_time",
            Self::TsToAgeS => "ts_to_age_s",
            Self::SystemTimeTicks => "system_time_ticks",
            Self::SleepTicks => "sleep_ticks",
            Self::TimerTimeNow => "timer_time_now",
            Self::TimerSecondsElapsedSince => "timer_seconds_elapsed_since",
            Self::TimerSleep => "timer_sleep",
            Self::Printf => "printf",
            Self::Malloc => "malloc",
            Self::Free => "free",
            Self::Spawn => "spawn",
            Self::RequestTerminate => "request_terminate",
            Self::ShouldTerminate => "should_terminate",
            Self::ThreadSetPriority => "thread_set_priority",
            Self::GetArg => "get_arg",
            Self::MutexCreate => "mutex_create",
            Self::MutexLock => "mutex_lock",
            Self::MutexUnlock => "mutex_unlock",
            Self::SemCreate => "sem_create",
            Self::SemWait => "sem_wait",
            Self::SemSignal => "sem_signal",
            Self::SemWaitTo => "sem_wait_to",
            Self::SemReset => "sem_reset",
        }
    }

    /// Return the pinned ABI kind for this slot.
    pub const fn kind(self) -> ExpressSlotKind {
        match self {
            Self::IfVersion
            | Self::LbmEncSymNil
            | Self::LbmEncSymTrue
            | Self::LbmEncSymTerror
            | Self::LbmEncSymEerror
            | Self::LbmEncSymMerror => ExpressSlotKind::Scalar,
            _ => ExpressSlotKind::Function,
        }
    }

    /// Return whether this slot contains a nullable function pointer.
    pub const fn is_callable(self) -> bool {
        matches!(self.kind(), ExpressSlotKind::Function)
    }
}

/// Return the pinned kind of an Express slot, if it is in the v1 table.
pub const fn express_slot_kind(index: usize) -> Option<ExpressSlotKind> {
    match index {
        0 | 38..=42 => Some(ExpressSlotKind::Scalar),
        1..=37 | 43..=79 => Some(ExpressSlotKind::Function),
        _ => None,
    }
}

/// Borrowed Express table after its breaking layout version has been checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressTable<'a> {
    words: &'a [u32],
}

impl<'a> ExpressTable<'a> {
    /// Validate the first slot and borrow all words supplied by firmware.
    ///
    /// A shorter table is valid for an older Express firmware: appended
    /// function slots are absent rather than shifted or reinterpreted.
    pub const fn load(words: &'a [u32]) -> Result<Self, ExpressTableError> {
        if words.is_empty() {
            return Err(ExpressTableError::Empty);
        }
        if words[0] != EXPRESS_C_IF_VERSION {
            return Err(ExpressTableError::VersionMismatch {
                expected: EXPRESS_C_IF_VERSION,
                found: words[0],
            });
        }
        Ok(Self { words })
    }

    /// Return the validated interface version.
    pub const fn version(self) -> u32 {
        self.words[0]
    }

    /// Return the number of words exposed by this firmware table.
    pub const fn len(self) -> usize {
        self.words.len()
    }

    /// Return whether the firmware exposed no table words.
    pub const fn is_empty(self) -> bool {
        self.words.is_empty()
    }

    /// Return a raw word when the firmware exposes that appended slot.
    pub fn word(self, index: usize) -> Option<ExpressWord> {
        self.words.get(index).map(|word| ExpressWord::new(*word))
    }

    /// Return a raw word from a named Express slot.
    pub fn word_at(self, slot: ExpressSlot) -> Option<ExpressWord> {
        self.word(slot.index())
    }

    /// Return a non-null function address without converting it to a host
    /// pointer or making a call through an unverified ABI.
    pub fn function_address(self, index: usize) -> Option<ExpressAddress> {
        if !matches!(express_slot_kind(index), Some(ExpressSlotKind::Function)) {
            return None;
        }
        match self.words.get(index) {
            Some(0) | None => None,
            Some(word) => Some(ExpressAddress::new(*word)),
        }
    }

    /// Return a named Express function slot when firmware exposes it and it is
    /// non-null.
    pub fn function_address_at(self, slot: ExpressSlot) -> Option<ExpressAddress> {
        self.function_address(slot.index())
    }

    /// Return whether all slots in the pinned v1 table are present.
    pub const fn is_complete(self) -> bool {
        self.words.len() >= EXPRESS_IF_SLOT_COUNT
    }
}

/// Error returned before any Express table slot is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressTableError {
    /// The firmware did not provide the version slot.
    Empty,
    /// The breaking interface version is not supported by this crate.
    VersionMismatch {
        /// Version expected by this table loader.
        expected: u32,
        /// Version found in the firmware table.
        found: u32,
    },
}
