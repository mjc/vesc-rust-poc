//! Core typed LispBM operations for the Express shared runtime.

use core::ffi::c_char;

use super::functions::{
    AddExtension, AddSymbolConst, CreateByteArray, GetSymbolByName, LbmBlockCtxFromExtension,
    LbmCar, LbmCdr, LbmCons, LbmContinueEval, LbmDecAsFloat, LbmDecAsI32, LbmDecAsU32, LbmDecChar,
    LbmDecStr, LbmDecSym, LbmEncChar, LbmEncFloat, LbmEncI, LbmEncI32, LbmEncSym, LbmEncU,
    LbmEncU32, LbmEvalIsPaused, LbmGetCurrentCid, LbmIsByteArray, LbmIsChar, LbmIsCons,
    LbmIsNumber, LbmIsSymbol, LbmIsSymbolNil, LbmIsSymbolTrue, LbmListDestructiveReverse,
    LbmPauseEvalWithGc, LbmSendMessage, LbmUnblockCtxUnboxed, SetErrorReason,
};
use super::{
    ExpressCallError, ExpressFlatValue, ExpressFlatValueError, ExpressInterface,
    ExpressLispMessageError, ExpressSlot,
};

/// Error returned when Express rejects a LispBM operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressLispOperationError {
    /// The firmware did not expose the requested operation.
    Unavailable(ExpressCallError),
    /// Firmware rejected the operation or its arguments.
    Rejected,
}

/// A typed Express LispBM value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExpressLispValue(u32);

impl ExpressLispValue {
    /// Construct a value from the firmware representation.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the firmware representation.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A typed Express LispBM symbol identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExpressLispSymbol(u32);

impl ExpressLispSymbol {
    /// Construct a symbol identifier from the firmware representation.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the firmware representation.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Checked core LispBM operations supplied by Express firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressLisp<'a> {
    interface: ExpressInterface<'a>,
}

impl<'a> ExpressLisp<'a> {
    /// Adopt a validated Express table as a LispBM provider.
    ///
    /// # Safety
    ///
    /// The table must be the live v1 Express firmware table on a matching
    /// target, with the pinned C signatures retained for its lifetime.
    pub const unsafe fn from_interface(interface: ExpressInterface<'a>) -> Self {
        Self { interface }
    }

    /// Encode a signed LispBM integer.
    pub fn enc_i(self, value: i32) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncI = unsafe { self.interface.function(ExpressSlot::LbmEncI) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode an unsigned LispBM integer.
    pub fn enc_u(self, value: u32) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncU = unsafe { self.interface.function(ExpressSlot::LbmEncU) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode a LispBM character.
    pub fn enc_char(self, value: u8) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncChar = unsafe { self.interface.function(ExpressSlot::LbmEncChar) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode a LispBM float.
    pub fn enc_float(self, value: f32) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncFloat = unsafe { self.interface.function(ExpressSlot::LbmEncFloat) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode a 32-bit signed LispBM value.
    pub fn enc_i32(self, value: i32) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncI32 = unsafe { self.interface.function(ExpressSlot::LbmEncI32) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode a 32-bit unsigned LispBM value.
    pub fn enc_u32(self, value: u32) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncU32 = unsafe { self.interface.function(ExpressSlot::LbmEncU32) }?;
        Ok(ExpressLispValue::new(unsafe { encode(value) }))
    }

    /// Encode a LispBM symbol identifier.
    pub fn enc_sym(self, symbol: ExpressLispSymbol) -> Result<ExpressLispValue, ExpressCallError> {
        let encode: LbmEncSym = unsafe { self.interface.function(ExpressSlot::LbmEncSym) }?;
        Ok(ExpressLispValue::new(unsafe { encode(symbol.raw()) }))
    }

    /// Decode a LispBM value as a float.
    pub fn dec_as_float(self, value: ExpressLispValue) -> Result<f32, ExpressCallError> {
        let decode: LbmDecAsFloat = unsafe { self.interface.function(ExpressSlot::LbmDecAsFloat) }?;
        Ok(unsafe { decode(value.raw()) })
    }

    /// Decode a LispBM value as an unsigned 32-bit integer.
    pub fn dec_as_u32(self, value: ExpressLispValue) -> Result<u32, ExpressCallError> {
        let decode: LbmDecAsU32 = unsafe { self.interface.function(ExpressSlot::LbmDecAsU32) }?;
        Ok(unsafe { decode(value.raw()) })
    }

    /// Decode a LispBM value as a signed 32-bit integer.
    pub fn dec_as_i32(self, value: ExpressLispValue) -> Result<i32, ExpressCallError> {
        let decode: LbmDecAsI32 = unsafe { self.interface.function(ExpressSlot::LbmDecAsI32) }?;
        Ok(unsafe { decode(value.raw()) })
    }

    /// Decode a LispBM value as a character.
    pub fn dec_char(self, value: ExpressLispValue) -> Result<u8, ExpressCallError> {
        let decode: LbmDecChar = unsafe { self.interface.function(ExpressSlot::LbmDecChar) }?;
        Ok(unsafe { decode(value.raw()) })
    }

    /// Decode a LispBM string value to the firmware-owned C string pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is owned by firmware and is only valid for the
    /// lifetime promised by the Express header. It must not be freed or
    /// written by the caller.
    pub unsafe fn dec_str(self, value: ExpressLispValue) -> Result<*mut c_char, ExpressCallError> {
        let decode: LbmDecStr = unsafe { self.interface.function(ExpressSlot::LbmDecStr) }?;
        Ok(unsafe { decode(value.raw()) })
    }

    /// Decode a LispBM value as a symbol identifier.
    pub fn dec_sym(self, value: ExpressLispValue) -> Result<ExpressLispSymbol, ExpressCallError> {
        let decode: LbmDecSym = unsafe { self.interface.function(ExpressSlot::LbmDecSym) }?;
        Ok(ExpressLispSymbol::new(unsafe { decode(value.raw()) }))
    }

    /// Construct a LispBM cons cell.
    pub fn cons(
        self,
        car: ExpressLispValue,
        cdr: ExpressLispValue,
    ) -> Result<ExpressLispValue, ExpressCallError> {
        let cons: LbmCons = unsafe { self.interface.function(ExpressSlot::LbmCons) }?;
        Ok(ExpressLispValue::new(unsafe { cons(car.raw(), cdr.raw()) }))
    }

    /// Return the car of a LispBM cons value.
    pub fn car(self, value: ExpressLispValue) -> Result<ExpressLispValue, ExpressCallError> {
        let car: LbmCar = unsafe { self.interface.function(ExpressSlot::LbmCar) }?;
        Ok(ExpressLispValue::new(unsafe { car(value.raw()) }))
    }

    /// Return the cdr of a LispBM cons value.
    pub fn cdr(self, value: ExpressLispValue) -> Result<ExpressLispValue, ExpressCallError> {
        let cdr: LbmCdr = unsafe { self.interface.function(ExpressSlot::LbmCdr) }?;
        Ok(ExpressLispValue::new(unsafe { cdr(value.raw()) }))
    }

    /// Reverse a LispBM list using firmware's destructive helper.
    pub fn list_destructive_reverse(
        self,
        value: ExpressLispValue,
    ) -> Result<ExpressLispValue, ExpressCallError> {
        let reverse: LbmListDestructiveReverse = unsafe {
            self.interface
                .function(ExpressSlot::LbmListDestructiveReverse)
        }?;
        Ok(ExpressLispValue::new(unsafe { reverse(value.raw()) }))
    }

    /// Return whether a value is a byte array.
    pub fn is_byte_array(self, value: ExpressLispValue) -> Result<bool, ExpressCallError> {
        let check: LbmIsByteArray =
            unsafe { self.interface.function(ExpressSlot::LbmIsByteArray) }?;
        Ok(unsafe { check(value.raw()) })
    }

    /// Return whether a value is a cons cell.
    pub fn is_cons(self, value: ExpressLispValue) -> Result<bool, ExpressCallError> {
        let check: LbmIsCons = unsafe { self.interface.function(ExpressSlot::LbmIsCons) }?;
        Ok(unsafe { check(value.raw()) })
    }

    /// Return whether a value is numeric.
    pub fn is_number(self, value: ExpressLispValue) -> Result<bool, ExpressCallError> {
        let check: LbmIsNumber = unsafe { self.interface.function(ExpressSlot::LbmIsNumber) }?;
        Ok(unsafe { check(value.raw()) })
    }

    /// Return whether a value is a character.
    pub fn is_char(self, value: ExpressLispValue) -> Result<bool, ExpressCallError> {
        let check: LbmIsChar = unsafe { self.interface.function(ExpressSlot::LbmIsChar) }?;
        Ok(unsafe { check(value.raw()) })
    }

    /// Return whether a value is a symbol.
    pub fn is_symbol(self, value: ExpressLispValue) -> Result<bool, ExpressCallError> {
        let check: LbmIsSymbol = unsafe { self.interface.function(ExpressSlot::LbmIsSymbol) }?;
        Ok(unsafe { check(value.raw()) })
    }

    /// Return whether a LispBM symbol identifier is `nil`.
    pub fn is_symbol_nil(self, symbol: ExpressLispSymbol) -> Result<bool, ExpressCallError> {
        let check: LbmIsSymbolNil =
            unsafe { self.interface.function(ExpressSlot::LbmIsSymbolNil) }?;
        Ok(unsafe { check(symbol.raw()) })
    }

    /// Return whether a LispBM symbol identifier is `true`.
    pub fn is_symbol_true(self, symbol: ExpressLispSymbol) -> Result<bool, ExpressCallError> {
        let check: LbmIsSymbolTrue =
            unsafe { self.interface.function(ExpressSlot::LbmIsSymbolTrue) }?;
        Ok(unsafe { check(symbol.raw()) })
    }

    fn symbol_constant(self, slot: ExpressSlot) -> Result<ExpressLispSymbol, ExpressCallError> {
        let value = self
            .interface
            .word(slot)
            .map(|word| word.get())
            .ok_or(ExpressCallError { slot })?;
        Ok(ExpressLispSymbol::new(value))
    }

    /// Return the firmware's `nil` symbol constant.
    pub fn symbol_nil(self) -> Result<ExpressLispSymbol, ExpressCallError> {
        self.symbol_constant(ExpressSlot::LbmEncSymNil)
    }

    /// Return the firmware's `true` symbol constant.
    pub fn symbol_true(self) -> Result<ExpressLispSymbol, ExpressCallError> {
        self.symbol_constant(ExpressSlot::LbmEncSymTrue)
    }

    /// Return the firmware's `terror` symbol constant.
    pub fn symbol_terror(self) -> Result<ExpressLispSymbol, ExpressCallError> {
        self.symbol_constant(ExpressSlot::LbmEncSymTerror)
    }

    /// Return the firmware's `eerror` symbol constant.
    pub fn symbol_eerror(self) -> Result<ExpressLispSymbol, ExpressCallError> {
        self.symbol_constant(ExpressSlot::LbmEncSymEerror)
    }

    /// Return the firmware's `merror` symbol constant.
    pub fn symbol_merror(self) -> Result<ExpressLispSymbol, ExpressCallError> {
        self.symbol_constant(ExpressSlot::LbmEncSymMerror)
    }

    /// Return the firmware's current LispBM context identifier.
    pub fn current_cid(self) -> Result<u32, ExpressCallError> {
        let current: LbmGetCurrentCid =
            unsafe { self.interface.function(ExpressSlot::LbmGetCurrentCid) }?;
        Ok(unsafe { current() })
    }

    /// Send a message to a LispBM context.
    pub fn send_message(self, cid: u32, value: ExpressLispValue) -> Result<i32, ExpressCallError> {
        let send: LbmSendMessage = unsafe { self.interface.function(ExpressSlot::LbmSendMessage) }?;
        Ok(unsafe { send(cid, value.raw()) })
    }

    /// Block the current extension context.
    pub fn block_context(self) -> Result<(), ExpressCallError> {
        let block: LbmBlockCtxFromExtension = unsafe {
            self.interface
                .function(ExpressSlot::LbmBlockCtxFromExtension)
        }?;
        unsafe { block() };
        Ok(())
    }

    /// Unblock a LispBM context with an unboxed value.
    pub fn unblock_context_unboxed(
        self,
        cid: u32,
        value: ExpressLispValue,
    ) -> Result<(), ExpressLispMessageError> {
        let unblock: LbmUnblockCtxUnboxed =
            unsafe { self.interface.function(ExpressSlot::LbmUnblockCtxUnboxed) }
                .map_err(ExpressLispMessageError::Unavailable)?;
        if unsafe { unblock(cid, value.raw()) } {
            Ok(())
        } else {
            Err(ExpressLispMessageError::Rejected)
        }
    }

    /// Start an ownership-scoped flattened LispBM value.
    pub fn start_flatten(
        self,
        buffer_size: usize,
    ) -> Result<ExpressFlatValue<'a>, ExpressFlatValueError> {
        ExpressFlatValue::start(self.interface, buffer_size)
    }

    /// Unblock a context with a finished flattened LispBM value.
    pub fn unblock_context_flat(
        self,
        cid: u32,
        mut value: ExpressFlatValue<'a>,
    ) -> Result<(), ExpressLispMessageError> {
        value.finish().map_err(|error| match error {
            ExpressFlatValueError::Unavailable(error) => {
                ExpressLispMessageError::Unavailable(error)
            }
            ExpressFlatValueError::Rejected => ExpressLispMessageError::Rejected,
        })?;
        let unblock: super::functions::LbmUnblockCtx =
            unsafe { self.interface.function(ExpressSlot::LbmUnblockCtx) }
                .map_err(ExpressLispMessageError::Unavailable)?;
        if unsafe { unblock(cid, &mut value.raw) } {
            value.relinquish();
            Ok(())
        } else {
            Err(ExpressLispMessageError::Rejected)
        }
    }

    /// Pause LispBM evaluation while retaining at least `num_free` words.
    pub fn pause_eval_with_gc(self, num_free: u32) -> Result<(), ExpressCallError> {
        let pause: LbmPauseEvalWithGc =
            unsafe { self.interface.function(ExpressSlot::LbmPauseEvalWithGc) }?;
        unsafe { pause(num_free) };
        Ok(())
    }

    /// Continue LispBM evaluation after a prior pause.
    pub fn continue_eval(self) -> Result<(), ExpressCallError> {
        let continue_eval: LbmContinueEval =
            unsafe { self.interface.function(ExpressSlot::LbmContinueEval) }?;
        unsafe { continue_eval() };
        Ok(())
    }

    /// Return whether LispBM evaluation is currently paused.
    pub fn eval_is_paused(self) -> Result<bool, ExpressCallError> {
        let is_paused: LbmEvalIsPaused =
            unsafe { self.interface.function(ExpressSlot::LbmEvalIsPaused) }?;
        Ok(unsafe { is_paused() })
    }

    /// Register a native LispBM extension with firmware.
    ///
    /// # Safety
    ///
    /// `name` must point to a writable, NUL-terminated C string and remain
    /// valid for the duration of the call. `handler` must use the exact
    /// Express extension callback ABI and obey firmware's callback rules.
    pub unsafe fn add_extension(
        self,
        name: *mut c_char,
        handler: super::functions::ExtensionHandler,
    ) -> Result<(), ExpressLispOperationError> {
        let add: AddExtension = unsafe { self.interface.function(ExpressSlot::LbmAddExtension) }
            .map_err(ExpressLispOperationError::Unavailable)?;
        if unsafe { add(name, handler) } {
            Ok(())
        } else {
            Err(ExpressLispOperationError::Rejected)
        }
    }

    /// Set the firmware-owned error reason for the current LispBM context.
    ///
    /// # Safety
    ///
    /// `reason` must point to a writable, NUL-terminated C string valid for
    /// the duration of the call.
    pub unsafe fn set_error_reason(self, reason: *mut c_char) -> Result<i32, ExpressCallError> {
        let set: SetErrorReason =
            unsafe { self.interface.function(ExpressSlot::LbmSetErrorReason) }?;
        Ok(unsafe { set(reason) })
    }

    /// Add a named symbol constant and write its identifier to `symbol`.
    ///
    /// # Safety
    ///
    /// `name` must be a valid NUL-terminated C string and `symbol` must be
    /// non-null and writable for the duration of the call.
    pub unsafe fn add_symbol_const(
        self,
        name: *const c_char,
        symbol: *mut u32,
    ) -> Result<i32, ExpressCallError> {
        let add: AddSymbolConst =
            unsafe { self.interface.function(ExpressSlot::LbmAddSymbolConst) }?;
        Ok(unsafe { add(name, symbol) })
    }

    /// Look up a symbol by name and write its identifier to `symbol`.
    ///
    /// # Safety
    ///
    /// `name` must be a valid NUL-terminated C string and `symbol` must be
    /// non-null and writable for the duration of the call.
    pub unsafe fn get_symbol_by_name(
        self,
        name: *const c_char,
        symbol: *mut u32,
    ) -> Result<i32, ExpressCallError> {
        let get: GetSymbolByName =
            unsafe { self.interface.function(ExpressSlot::LbmGetSymbolByName) }?;
        Ok(unsafe { get(name, symbol) })
    }

    /// Ask firmware to create a byte-array value in place.
    ///
    /// # Safety
    ///
    /// `value` must be non-null and writable for the duration of the call.
    pub unsafe fn create_byte_array(
        self,
        value: *mut ExpressLispValue,
        elements: u32,
    ) -> Result<(), ExpressLispOperationError> {
        let create: CreateByteArray =
            unsafe { self.interface.function(ExpressSlot::LbmCreateByteArray) }
                .map_err(ExpressLispOperationError::Unavailable)?;
        if unsafe { create(value.cast(), elements) } {
            Ok(())
        } else {
            Err(ExpressLispOperationError::Rejected)
        }
    }
}
