//! LispBM extension descriptor validation and registration errors.

use core::ffi::CStr;

use vescpkg_rs_sys::raw::LbmFlatValue;
use vescpkg_rs_sys::{ExtensionHandler, LbmValue};

const LBM_INT_TAG: u32 = 0x8;
const LBM_VALUE_SHIFT: u32 = 4;
#[cfg(any(test, not(target_arch = "arm")))]
const LBM_TRUE: u32 = 2 << LBM_VALUE_SHIFT;
const LBM_VALUE_TAG_MASK: u32 = (1 << LBM_VALUE_SHIFT) - 1;
const LBM_INT_MIN: i32 = -(1 << 27);
const LBM_INT_MAX: i32 = (1 << 27) - 1;

const fn encode_integer(value: i32) -> u32 {
    value.wrapping_shl(LBM_VALUE_SHIFT) as u32 | LBM_INT_TAG
}

const fn decode_integer(value: u32) -> i32 {
    (value as i32) >> LBM_VALUE_SHIFT
}

const fn is_integer(value: u32) -> bool {
    value & LBM_VALUE_TAG_MASK == LBM_INT_TAG
}

/// A LispBM value that can only be produced by the SDK's typed argument API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispValue(LbmValue);

/// A firmware symbol identifier suitable for encoding into a LispBM value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispSymbol(u32);

impl LispSymbol {
    /// Construct a symbol identifier returned by firmware symbol lookup.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the firmware symbol identifier.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Look up a firmware symbol by name without letting the borrowed name escape.
    #[cfg(not(test))]
    pub fn lookup(name: &CStr) -> Option<Self> {
        let mut symbol = 0;
        let result =
            unsafe { crate::ffi::lbm_get_symbol_by_name(name.as_ptr().cast_mut(), &mut symbol) };
        (result != 0).then_some(Self::new(symbol))
    }
}

/// A LispBM evaluator context identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispContextId(u32);

impl LispContextId {
    /// Construct a context identifier supplied by firmware.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the firmware context identifier.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Failure returned when firmware rejects a LispBM process message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LispMessageError {
    /// The target context did not accept the message.
    Rejected,
}

/// Process controls available while an extension callback is executing.
pub struct LispProcess;

/// A firmware-owned flattened LispBM value under construction.
///
/// The firmware exposes these constructors as optional ABI slots.
/// The buffer is reclaimed if the value is dropped without being accepted by
/// a context; a successful unblock transfers ownership to LispBM.
#[cfg_attr(not(test), must_use)]
#[cfg_attr(test, allow(dead_code))]
pub struct LispFlatValue {
    raw: LbmFlatValue,
    finished: bool,
}

#[cfg(not(test))]
impl LispFlatValue {
    /// Start a flattened value with a firmware-allocated buffer.
    pub fn try_new(buffer_size: usize) -> Option<Self> {
        let mut raw = LbmFlatValue {
            buf: core::ptr::null_mut(),
            buf_size: 0,
            buf_pos: 0,
        };
        (unsafe { crate::ffi::lbm_start_flatten(&mut raw, buffer_size) } == Some(true)).then_some(
            Self {
                raw,
                finished: false,
            },
        )
    }

    /// Append a cons marker.
    pub fn push_cons(&mut self) -> bool {
        !self.finished && unsafe { crate::ffi::f_cons(&mut self.raw) } == Some(true)
    }

    /// Append a symbol identifier.
    pub fn push_symbol(&mut self, symbol: LispSymbol) -> bool {
        !self.finished && unsafe { crate::ffi::f_sym(&mut self.raw, symbol.raw()) } == Some(true)
    }

    /// Append a signed 32-bit value.
    pub fn push_i32(&mut self, value: i32) -> bool {
        !self.finished && unsafe { crate::ffi::f_i32(&mut self.raw, value) } == Some(true)
    }

    /// Append an unsigned 32-bit value.
    pub fn push_u32(&mut self, value: u32) -> bool {
        !self.finished && unsafe { crate::ffi::f_u32(&mut self.raw, value) } == Some(true)
    }

    /// Append an `f32` value.
    pub fn push_float(&mut self, value: f32) -> bool {
        !self.finished && unsafe { crate::ffi::f_float(&mut self.raw, value) } == Some(true)
    }

    /// Append a byte value.
    pub fn push_byte(&mut self, value: u8) -> bool {
        !self.finished && unsafe { crate::ffi::f_b(&mut self.raw, value) } == Some(true)
    }

    /// Append a signed 64-bit value.
    pub fn push_i64(&mut self, value: i64) -> bool {
        !self.finished && unsafe { crate::ffi::f_i64(&mut self.raw, value) } == Some(true)
    }

    /// Append an unsigned 64-bit value.
    pub fn push_u64(&mut self, value: u64) -> bool {
        !self.finished && unsafe { crate::ffi::f_u64(&mut self.raw, value) } == Some(true)
    }

    /// Append a byte array copied by firmware into the flattened value.
    pub fn push_byte_array(&mut self, bytes: &[u8]) -> bool {
        let Ok(count) = u32::try_from(bytes.len()) else {
            return false;
        };
        !self.finished
            && unsafe { crate::ffi::f_lbm_array(&mut self.raw, count, bytes.as_ptr().cast_mut()) }
                == Some(true)
    }

    /// Finish the flattened value before passing it to LispBM.
    pub fn finish(&mut self) -> bool {
        if self.finished {
            return true;
        }
        self.finished = unsafe { crate::ffi::lbm_finish_flatten(&mut self.raw) == Some(true) };
        self.finished
    }
}

#[cfg(not(test))]
impl Drop for LispFlatValue {
    fn drop(&mut self) {
        if !self.raw.buf.is_null() {
            unsafe { crate::ffi::vesc_free(self.raw.buf.cast()) };
            self.raw.buf = core::ptr::null_mut();
        }
    }
}

impl LispProcess {
    /// Set the firmware-owned error reason for the current LispBM evaluation.
    #[cfg(not(test))]
    pub fn set_error_reason(reason: &CStr) -> i32 {
        unsafe { crate::ffi::lbm_set_error_reason(reason.as_ptr().cast_mut()) }
    }

    /// Return the context currently executing the extension callback.
    #[cfg(not(test))]
    pub fn current() -> LispContextId {
        LispContextId::new(unsafe { crate::ffi::lbm_get_current_cid() })
    }

    /// Block the current extension context until firmware unblocks it.
    #[cfg(not(test))]
    pub fn block_current() {
        unsafe { crate::ffi::lbm_block_ctx_from_extension() }
    }

    /// Unblock a context with an unboxed LispBM value.
    #[cfg(not(test))]
    pub fn unblock(context: LispContextId, value: LispValue) -> Result<(), LispMessageError> {
        match unsafe { crate::ffi::lbm_unblock_ctx_unboxed(context.raw(), value.raw()) } {
            Some(true) => Ok(()),
            Some(false) | None => Err(LispMessageError::Rejected),
        }
    }

    /// Unblock a context with a finished flattened value.
    #[cfg(not(test))]
    pub fn unblock_flat(
        context: LispContextId,
        mut value: LispFlatValue,
    ) -> Result<(), LispMessageError> {
        if !value.finish() {
            return Err(LispMessageError::Rejected);
        }
        let result = unsafe { crate::ffi::lbm_unblock_ctx(context.raw(), &mut value.raw) };
        if result == Some(true) {
            value.raw.buf = core::ptr::null_mut();
            Ok(())
        } else {
            Err(LispMessageError::Rejected)
        }
    }
}

impl LispValue {
    /// Convert any LispBM numeric value to an `f32`.
    #[cfg(not(test))]
    pub fn decode_number_as_f32(self) -> Option<f32> {
        crate::lifecycle_core::LbmApi::new(crate::bindings::RealBindings)
            .decode_number_as_f32(self.raw())
    }

    /// Widen a firmware numeric value to `f64` without adding a double ABI.
    #[cfg(not(test))]
    pub fn decode_number_as_f64(self) -> Option<f64> {
        self.decode_number_as_f32().map(f64::from)
    }

    /// Decode an `f32` only when the value is a non-integer LispBM number.
    #[cfg(not(test))]
    pub fn decode_f32_exact(self) -> Option<f32> {
        (!self.is_integer() && self.is_number())
            .then(|| unsafe { crate::ffi::lbm_dec_as_float(self.raw()) })
    }

    /// Decode an exact LispBM float widened to `f64`.
    #[cfg(not(test))]
    pub fn decode_f64_exact(self) -> Option<f64> {
        self.decode_f32_exact().map(f64::from)
    }

    /// Decode this value only when it is an immediate LispBM integer.
    pub fn decode_i32_exact(self) -> Option<i32> {
        is_integer(self.raw().0).then(|| decode_integer(self.raw().0))
    }

    /// Decode an immediate LispBM integer exactly when it is non-negative.
    #[must_use]
    pub fn decode_u32_exact(self) -> Option<u32> {
        self.decode_i32_exact()
            .and_then(|value| value.try_into().ok())
    }

    /// Decode an immediate LispBM integer exactly as an `i64`.
    ///
    /// The result is widened from the firmware's immediate payload; this does
    /// not claim to decode a wider flat value.
    pub fn decode_i64_exact(self) -> Option<i64> {
        self.decode_i32_exact().map(i64::from)
    }

    /// Decode an immediate non-negative LispBM integer exactly as a `u64`.
    pub fn decode_u64_exact(self) -> Option<u64> {
        self.decode_u32_exact().map(u64::from)
    }

    /// Convert a firmware-classified numeric value to an unsigned integer.
    #[cfg(not(test))]
    pub fn decode_number_as_u32(self) -> Option<u32> {
        self.is_number()
            .then(|| unsafe { crate::ffi::lbm_dec_as_u32(self.raw()) })
    }

    /// Convert a firmware-classified numeric value to a widened unsigned integer.
    ///
    /// The pinned VESC ABI exposes a 32-bit scalar decoder; wider LispBM values
    /// are constructed through [`LispFlatValue`] when needed.
    #[cfg(not(test))]
    pub fn decode_number_as_u64(self) -> Option<u64> {
        self.decode_number_as_u32().map(u64::from)
    }

    /// Encode an unsigned integer through the firmware's LispBM representation.
    #[cfg(not(test))]
    pub fn from_u32(value: u32) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_enc_u32(value) })
    }

    /// Encode a signed integer through the firmware's LispBM representation.
    #[cfg(not(test))]
    pub fn from_i32(value: i32) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_enc_i(value) })
    }

    /// Encode an `f32` through the firmware's LispBM representation.
    #[cfg(not(test))]
    pub fn from_f32(value: f32) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_enc_float(value) })
    }

    /// Encode a `f64` only when its value is exactly representable by the
    /// firmware's `f32` LispBM encoder.
    #[cfg(not(test))]
    pub fn from_f64(value: f64) -> Option<Self> {
        let narrowed = value as f32;
        (!value.is_nan() && f64::from(narrowed) == value).then(|| Self::from_f32(narrowed))
    }

    /// Decode a LispBM character value.
    #[cfg(not(test))]
    pub fn decode_char(self) -> Option<u8> {
        self.is_char()
            .then(|| unsafe { crate::ffi::lbm_dec_char(self.raw()) })
    }

    /// Encode a byte as a LispBM character value.
    #[cfg(not(test))]
    pub fn from_char(value: u8) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_enc_char(value) })
    }

    /// Return whether this value is an immediate LispBM integer.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        is_integer(self.raw().0)
    }

    /// Return whether firmware classifies this value as numeric.
    #[must_use]
    pub fn is_number(self) -> bool {
        unsafe { crate::ffi::lbm_is_number(self.raw()) }
    }

    /// Return whether firmware classifies this value as a character.
    #[must_use]
    pub fn is_char(self) -> bool {
        unsafe { crate::ffi::lbm_is_char(self.raw()) }
    }

    /// Return whether firmware classifies this value as a symbol.
    #[must_use]
    pub fn is_symbol(self) -> bool {
        unsafe { crate::ffi::lbm_is_symbol(self.raw()) }
    }

    /// Return whether firmware classifies this value as a cons cell.
    #[must_use]
    pub fn is_cons(self) -> bool {
        unsafe { crate::ffi::lbm_is_cons(self.raw()) }
    }

    /// Return whether firmware classifies this value as a byte array.
    #[must_use]
    pub fn is_byte_array(self) -> bool {
        unsafe { crate::ffi::lbm_is_byte_array(self.raw()) }
    }

    /// Return whether this value is an array in the pinned LispBM runtime.
    ///
    /// VESC's LispBM ABI exposes only byte arrays, so this is intentionally the
    /// same capability check as [`Self::is_byte_array`].
    #[must_use]
    pub fn is_array(self) -> bool {
        self.is_byte_array()
    }

    /// Return whether this value is the canonical LispBM nil value.
    #[must_use]
    pub fn is_nil(self) -> bool {
        self == Self::nil()
    }

    /// Return whether this value is the canonical LispBM true value.
    #[must_use]
    pub fn is_true(self) -> bool {
        self == Self::true_value()
    }

    /// Allocate a LispBM byte array through the firmware allocator.
    pub fn try_byte_array(len: usize) -> Option<Self> {
        let len = u32::try_from(len).ok()?;
        let mut value = LbmValue(0);
        unsafe { crate::ffi::lbm_create_byte_array(&mut value, len) }.then(|| Self::from_raw(value))
    }

    /// Borrow firmware-owned string bytes for the duration of a callback.
    ///
    /// The callback boundary prevents the returned `CStr` from escaping the
    /// evaluation that owns the LispBM storage.
    #[cfg(not(test))]
    pub fn with_str<R>(self, f: impl FnOnce(&CStr) -> R) -> Option<R> {
        if !self.is_byte_array() {
            return None;
        }
        let pointer = unsafe { crate::ffi::lbm_dec_str(self.raw()) };
        (!pointer.is_null()).then(|| {
            let value = unsafe { CStr::from_ptr(pointer) };
            f(value)
        })
    }

    /// Encode a firmware symbol identifier as a LispBM value.
    #[cfg(not(test))]
    pub fn from_symbol(symbol: LispSymbol) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_enc_sym(symbol.raw()) })
    }

    /// Decode a LispBM symbol identifier when this value is a symbol.
    #[cfg(not(test))]
    pub fn symbol_id(self) -> Option<LispSymbol> {
        self.is_symbol()
            .then(|| LispSymbol::new(unsafe { crate::ffi::lbm_dec_sym(self.raw()) }))
    }

    /// Send this value to a running LispBM context.
    #[cfg(not(test))]
    pub fn send_to(self, context: LispContextId) -> Result<(), LispMessageError> {
        (unsafe { crate::ffi::lbm_send_message(context.raw(), self.raw()) } == 1)
            .then_some(())
            .ok_or(LispMessageError::Rejected)
    }

    /// Construct a LispBM cons cell from two owned value handles.
    #[cfg(not(test))]
    pub fn cons(car: Self, cdr: Self) -> Self {
        Self::from_raw(unsafe { crate::ffi::lbm_cons(car.raw(), cdr.raw()) })
    }

    /// Read the head of a cons cell while preserving its firmware ownership.
    #[cfg(not(test))]
    pub fn car(self) -> Option<Self> {
        self.is_cons()
            .then(|| Self::from_raw(unsafe { crate::ffi::lbm_car(self.raw()) }))
    }

    /// Read the tail of a cons cell while preserving its firmware ownership.
    #[cfg(not(test))]
    pub fn cdr(self) -> Option<Self> {
        self.is_cons()
            .then(|| Self::from_raw(unsafe { crate::ffi::lbm_cdr(self.raw()) }))
    }

    /// Destructively reverse a firmware-owned list while retaining its handle.
    #[cfg(not(test))]
    pub fn reverse_list(self) -> Option<Self> {
        self.is_cons().then(|| {
            Self::from_raw(unsafe { crate::ffi::lbm_list_destructive_reverse(self.raw()) })
        })
    }

    /// Convert any LispBM numeric value to an `i32`.
    #[cfg(not(test))]
    pub fn decode_number_as_i32(self) -> Option<i32> {
        crate::lifecycle_core::LbmApi::new(crate::bindings::RealBindings)
            .decode_number_as_i32(self.raw())
    }

    /// Convert a firmware-classified numeric value to a widened signed integer.
    ///
    /// The pinned VESC ABI exposes a 32-bit scalar decoder; wider LispBM values
    /// are constructed through [`LispFlatValue`] when needed.
    #[cfg(not(test))]
    pub fn decode_number_as_i64(self) -> Option<i64> {
        self.decode_number_as_i32().map(i64::from)
    }

    /// Return LispBM true.
    pub fn true_value() -> Self {
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            Self::from_raw(
                crate::lifecycle_core::LbmApi::new(crate::bindings::RealBindings).encode_true(),
            )
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        {
            Self(LbmValue(LBM_TRUE))
        }
    }

    /// Return LispBM nil.
    pub fn nil() -> Self {
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            Self::from_raw(
                crate::lifecycle_core::LbmApi::new(crate::bindings::RealBindings).encode_nil(),
            )
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        {
            Self(LbmValue(0))
        }
    }

    /// Convert a Rust boolean to LispBM true or nil.
    pub fn boolean(value: bool) -> Self {
        if value {
            Self::true_value()
        } else {
            Self::nil()
        }
    }

    pub(crate) const fn raw(self) -> LbmValue {
        self.0
    }

    pub(crate) const fn from_raw(value: LbmValue) -> Self {
        Self(value)
    }
}

/// Error returned when an integer cannot use LispBM's immediate representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispIntegerError {
    value: i32,
}

impl LispIntegerError {
    /// Return the rejected integer.
    pub const fn value(self) -> i32 {
        self.value
    }
}

impl core::fmt::Display for LispIntegerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} is outside LispBM's immediate integer range",
            self.value
        )
    }
}

impl core::error::Error for LispIntegerError {}

impl TryFrom<i32> for LispValue {
    type Error = LispIntegerError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        (LBM_INT_MIN..=LBM_INT_MAX)
            .contains(&value)
            .then(|| Self(LbmValue(encode_integer(value))))
            .ok_or(LispIntegerError { value })
    }
}

/// Errors returned when extension registration fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(any(test, feature = "test-support", target_arch = "arm")),
    allow(dead_code)
)]
pub(crate) enum ExtensionRegistrationError {
    /// Firmware rejected the registration request.
    FirmwareRejected,
}

/// Result of registering a package's LispBM extension table.
///
/// VESC exposes extension insertion but not removal through `VESC_IF`, so a
/// batch can be partially registered. Once any entry succeeds, the SDK keeps
/// the native image loaded even if later package startup reports failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionRegistration {
    requested: usize,
    registered: usize,
}

impl ExtensionRegistration {
    #[cfg_attr(
        not(any(test, feature = "test-support", target_arch = "arm")),
        allow(dead_code)
    )]
    // Used by lifecycle extension registration on firmware and test-support builds.
    pub(crate) const fn new(requested: usize, registered: usize) -> Self {
        Self {
            requested,
            registered,
        }
    }

    /// Return how many extension handlers firmware accepted.
    #[must_use]
    pub const fn registered(self) -> usize {
        self.registered
    }

    /// Return whether firmware accepted every requested extension.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.registered == self.requested
    }
}

/// A static name assigned to a LispBM extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ExtensionName(&'static [u8]);

impl ExtensionName {
    /// Build a name from the terminated storage generated by `extension_name!`.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_terminated(name: &'static str) -> Option<Self> {
        let bytes = name.as_bytes();
        if bytes.len() < 5
            || bytes[0] != b'e'
            || bytes[1] != b'x'
            || bytes[2] != b't'
            || bytes[3] != b'-'
        {
            return None;
        }
        match CStr::from_bytes_with_nul(bytes) {
            Ok(_) => Some(Self(name.as_bytes())),
            Err(_) => None,
        }
    }

    #[cfg_attr(
        not(any(test, feature = "test-support", target_arch = "arm")),
        allow(dead_code)
    )]
    // Firmware/test-support registration needs the validated C name pointer.
    pub(crate) const fn as_cstr(self) -> &'static CStr {
        // SAFETY: the macro support hook validates the terminating NUL byte.
        unsafe { CStr::from_bytes_with_nul_unchecked(self.0) }
    }

    /// Return the validated Rust extension name without its ABI terminator.
    pub fn as_str(self) -> &'static str {
        let bytes = &self.0[..self.0.len() - 1];
        // SAFETY: the macro support hook accepts only valid C strings built
        // from UTF-8 Rust string literals.
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
}

/// Create a checked static LispBM extension name from a Rust string literal.
#[macro_export]
macro_rules! extension_name {
    ($name:literal) => {
        const {
            match $crate::ExtensionName::__from_terminated(concat!($name, "\0")) {
                Some(name) => name,
                None => panic!("extension name must begin with `ext-` and contain no NUL"),
            }
        }
    };
}

/// A validated extension registration request.
#[derive(Clone, Copy)]
pub struct ExtensionDescriptor {
    name: ExtensionName,
    #[cfg_attr(
        not(any(test, feature = "test-support", target_arch = "arm")),
        allow(dead_code)
    )]
    handler: ExtensionHandler,
    #[cfg_attr(
        not(any(test, feature = "test-support", target_arch = "arm")),
        allow(dead_code)
    )]
    state_type: Option<fn() -> core::any::TypeId>,
}

impl ExtensionDescriptor {
    pub(crate) fn from_handler(name: ExtensionName, handler: ExtensionHandler) -> Self {
        Self {
            name,
            handler,
            state_type: None,
        }
    }

    /// Build a descriptor for a stateless typed extension callback.
    #[inline(always)]
    pub fn typed<T: LbmExtension>(name: ExtensionName) -> Self {
        Self::from_handler(name, lbm_extension_handler::<T>)
    }

    /// Build a descriptor for a runtime-state-backed typed extension callback.
    #[inline(always)]
    pub fn stateful<T: StatefulLbmExtension>(name: ExtensionName) -> Self {
        Self {
            name,
            handler: stateful_lbm_extension_handler::<T>,
            state_type: Some(runtime_state_type::<T::State>),
        }
    }

    /// Return the descriptor name.
    pub const fn name(self) -> ExtensionName {
        self.name
    }

    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    /// Return the descriptor handler.
    pub(crate) const fn handler(self) -> ExtensionHandler {
        self.handler
    }

    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    pub(crate) fn state_type(self) -> Option<core::any::TypeId> {
        self.state_type.map(|state_type| state_type())
    }
}

fn runtime_state_type<T: 'static>() -> core::any::TypeId {
    core::any::TypeId::of::<T>()
}

/// Typed LispBM extension callback arguments.
pub struct LispArgs<'a> {
    values: &'a [LbmValue],
}

impl LispArgs<'static> {
    /// Construct an empty extension argument list.
    #[must_use]
    pub const fn empty() -> Self {
        Self { values: &[] }
    }
}

impl LispArgs<'_> {
    fn from_raw(args: *mut u32, arg_count: u32) -> Option<Self> {
        let len = usize::try_from(arg_count).ok()?;
        if len == 0 {
            return Some(Self { values: &[] });
        }
        unsafe { crate::firmware::lbm_args(args, arg_count).map(|values| Self { values }) }
    }

    /// Return the number of arguments supplied by LispBM.
    pub const fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether LispBM supplied no arguments.
    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return one argument by position.
    pub fn get(&self, index: usize) -> Option<LispValue> {
        self.values.get(index).copied().map(LispValue::from_raw)
    }
}

fn canonical_nil_raw() -> u32 {
    LispValue::nil().raw().0
}

/// Rust implementation for a LispBM extension callback.
pub trait LbmExtension {
    /// Handle one extension call.
    fn call(args: LispArgs<'_>) -> LispValue;
}

/// State-backed LispBM extension behavior for package authors.
pub trait StatefulLbmExtension {
    /// Package state installed by startup.
    type State: crate::PackageRuntimeState;

    /// Handle one extension call with the current package state.
    fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue;
}

/// Firmware ABI trampoline for a typed LispBM extension callback.
///
/// # Safety
///
/// `args` must be null with `arg_count == 0` or point to `arg_count` LispBM values that stay valid for
/// this call.
pub unsafe extern "C" fn lbm_extension_handler<T: LbmExtension>(
    args: *mut u32,
    arg_count: u32,
) -> u32 {
    let Some(args) = LispArgs::from_raw(args, arg_count) else {
        return canonical_nil_raw();
    };
    T::call(args).raw().0
}

/// Firmware ABI trampoline for a state-backed LispBM extension callback.
///
/// # Safety
///
/// `args` must be null with `arg_count == 0` or point to `arg_count` LispBM values that stay valid for
/// this call.
pub unsafe extern "C" fn stateful_lbm_extension_handler<T: StatefulLbmExtension>(
    args: *mut u32,
    arg_count: u32,
) -> u32 {
    let Some(args) = LispArgs::from_raw(args, arg_count) else {
        return canonical_nil_raw();
    };
    let nil = LispValue::nil();
    #[cfg(all(not(test), target_arch = "arm"))]
    let program = crate::firmware_package_program_address!(stateful_lbm_extension_handler::<T>);
    #[cfg(all(not(test), target_arch = "arm"))]
    // SAFETY: `program` is derived from this registered package handler, so
    // VESC returns the live `T::State` installed in this package's ARG slot.
    let result = unsafe { crate::firmware::__firmware_package_state_ptr::<T::State>(program) }
        .and_then(|state| {
            <T::State as crate::PackageRuntimeState>::runtime_store()
                .with_expected_mut(crate::runtime::ExpectedState::Exact(state), |state| {
                    T::call(state, args)
                })
        });
    #[cfg(any(test, not(target_arch = "arm")))]
    let result = <T::State as crate::PackageRuntimeState>::runtime_store()
        .with_mut(|state| T::call(state, args));
    result.unwrap_or(nil).raw().0
}

#[cfg(test)]
mod tests {
    use super::{LispArgs, LispValue, StatefulLbmExtension, stateful_lbm_extension_handler};
    use crate::{PackageRuntimeState, PackageStateStore};
    use std::boxed::Box;

    #[derive(Debug, PartialEq, Eq)]
    struct State {
        calls: u32,
    }

    struct TestExtension;

    struct IntegerExtension;

    struct EchoIntegerExtension;

    static SLOT: PackageStateStore<State> = PackageStateStore::new();

    impl PackageRuntimeState for State {
        fn runtime_store() -> &'static PackageStateStore<Self> {
            &SLOT
        }
    }

    impl StatefulLbmExtension for TestExtension {
        type State = State;

        fn call(state: &mut Self::State, args: LispArgs<'_>) -> LispValue {
            state.calls += 1;
            let _ = args;
            LispValue::true_value()
        }
    }

    impl super::LbmExtension for IntegerExtension {
        fn call(_args: LispArgs<'_>) -> LispValue {
            LispValue::try_from(42).unwrap()
        }
    }

    impl super::LbmExtension for EchoIntegerExtension {
        fn call(args: LispArgs<'_>) -> LispValue {
            args.get(0)
                .and_then(LispValue::decode_i32_exact)
                .and_then(|value| LispValue::try_from(value).ok())
                .unwrap_or_else(LispValue::nil)
        }
    }

    #[test]
    fn stateful_lbm_extension_handler_passes_package_state() {
        assert_eq!(
            unsafe { stateful_lbm_extension_handler::<TestExtension>(core::ptr::null_mut(), 0) },
            0
        );

        let state = Box::leak(Box::new(State { calls: 0 }));
        unsafe { SLOT.install(state) }.unwrap();

        assert_eq!(
            unsafe { stateful_lbm_extension_handler::<TestExtension>(core::ptr::null_mut(), 0) },
            super::LBM_TRUE
        );
        assert_eq!(*state, State { calls: 1 });

        SLOT.clear();
    }

    #[test]
    fn typed_lisp_integer_keeps_device_encoding_inside_the_sdk() {
        assert_eq!(
            unsafe { super::lbm_extension_handler::<IntegerExtension>(core::ptr::null_mut(), 0) },
            super::encode_integer(42),
        );
    }

    #[test]
    fn typed_lisp_integer_rejects_values_outside_the_immediate_payload() {
        assert!(LispValue::try_from(super::LBM_INT_MIN).is_ok());
        assert!(LispValue::try_from(super::LBM_INT_MAX).is_ok());
        assert!(LispValue::try_from(super::LBM_INT_MIN - 1).is_err());
        assert!(LispValue::try_from(super::LBM_INT_MAX + 1).is_err());
    }

    #[test]
    fn typed_lisp_args_decode_device_integer_encoding() {
        let mut args = [super::encode_integer(37)];
        assert_eq!(
            unsafe {
                super::lbm_extension_handler::<EchoIntegerExtension>(
                    args.as_mut_ptr().cast(),
                    args.len() as u32,
                )
            },
            super::encode_integer(37),
        );
    }

    #[test]
    fn typed_lisp_args_reject_non_integer_values() {
        let mut values = [0_u32];
        let args = super::LispArgs::from_raw(values.as_mut_ptr(), values.len() as u32)
            .expect("valid arguments");

        assert_eq!(args.get(0).and_then(LispValue::decode_i32_exact), None);
    }

    #[test]
    fn extension_handlers_return_canonical_nil_for_invalid_arguments() {
        let nil = LispValue::nil().raw().0;

        assert_eq!(
            unsafe {
                super::lbm_extension_handler::<EchoIntegerExtension>(core::ptr::null_mut(), 1)
            },
            nil
        );
        assert_eq!(
            unsafe {
                super::stateful_lbm_extension_handler::<TestExtension>(core::ptr::null_mut(), 1)
            },
            nil
        );
    }

    #[test]
    fn extension_name_exposes_rust_text_without_abi_terminator() {
        let name = crate::extension_name!("ext-example");

        assert_eq!(name.as_str(), "ext-example");
        assert_eq!(
            super::ExtensionName::__from_terminated("ext-example\0")
                .map(super::ExtensionName::as_str),
            Some("ext-example")
        );
        assert!(super::ExtensionName::__from_terminated("ext-example").is_none());
        assert!(super::ExtensionName::__from_terminated("ext\0-example\0").is_none());
    }
}
