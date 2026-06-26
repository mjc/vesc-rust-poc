use core::ffi::CStr;

use crate::ffi::{self, LbmApi, LbmBindings, LbmCount, LbmValue};

const EXT_RUST_ADD_NAME: &CStr = c"ext-rust-add";

pub struct PackageLifecycle<B = ffi::RealBindings> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn register_extensions_with(&self, handler: ffi::ExtensionHandler) -> i32 {
        self.api.register_extension(EXT_RUST_ADD_NAME, handler)
    }

    #[cfg(not(test))]
    pub fn register_extensions(&self) -> i32 {
        self.register_extensions_with(ext_rust_add)
    }
}

#[cfg(not(test))]
pub fn init_package() {
    let lifecycle = PackageLifecycle::new(ffi::RealBindings);
    let _ = lifecycle.register_extensions();
}

#[cfg(not(test))]
unsafe extern "C" fn ext_rust_add(args: *mut LbmValue, argn: LbmCount) -> LbmValue {
    rust_add_extension_value(&LbmApi::new(ffi::RealBindings), args, argn)
}

fn rust_add_extension_value<B: LbmBindings>(
    api: &LbmApi<B>,
    args: *mut LbmValue,
    argn: LbmCount,
) -> LbmValue {
    if argn.0 != 2 || args.is_null() {
        return api.encode_eval_error();
    }

    let first = unsafe { *args };
    let second = unsafe { *args.add(1) };
    if !api.is_number(first) || !api.is_number(second) {
        return api.encode_eval_error();
    }

    let a = api.decode_i32(first);
    let b = api.decode_i32(second);
    api.encode_i32(crate::rust_add(a, b))
}

#[cfg(test)]
mod tests {
    use super::{
        rust_add_extension_value, LbmApi, LbmBindings, LbmCount, LbmValue, PackageLifecycle,
    };
    use crate::ffi;
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        number_result: bool,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                number_result: true,
            }
        }

        fn with_number_result(number_result: bool) -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                number_result,
            }
        }
    }

    impl LbmBindings for FakeBindings {
        unsafe fn add_extension(
            &self,
            _name: *const c_char,
            _handler: ffi::ExtensionHandler,
        ) -> i32 {
            self.add_calls.set(self.add_calls.get() + 1);
            17
        }

        unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
            self.decode_calls.set(self.decode_calls.get() + 1);
            value.0 as i32
        }

        unsafe fn encode_i32(&self, value: i32) -> LbmValue {
            LbmValue(value as u32)
        }

        unsafe fn is_number(&self, _value: LbmValue) -> bool {
            self.number_result
        }

        unsafe fn encode_eval_error(&self) -> LbmValue {
            LbmValue(0xeeee_eeee)
        }
    }

    unsafe extern "C" fn stub_handler(_args: *mut LbmValue, _count: super::LbmCount) -> LbmValue {
        LbmValue(0)
    }

    #[test]
    fn registers_the_rust_extension_through_the_lifecycle_helper() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);

        assert_eq!(lifecycle.register_extensions_with(stub_handler), 17);
    }

    #[test]
    fn rust_add_extension_adds_checked_numeric_arguments() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20), LbmValue(22)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(2)),
            LbmValue(42)
        );
    }

    #[test]
    fn rust_add_extension_returns_eval_error_for_bad_arguments() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(1)),
            LbmValue(0xeeee_eeee)
        );
        assert_eq!(
            rust_add_extension_value(&api, core::ptr::null_mut(), LbmCount(2)),
            LbmValue(0xeeee_eeee)
        );

        let api = LbmApi::new(FakeBindings::with_number_result(false));
        let mut args = [LbmValue(20), LbmValue(22)];
        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(2)),
            LbmValue(0xeeee_eeee)
        );
    }
}
