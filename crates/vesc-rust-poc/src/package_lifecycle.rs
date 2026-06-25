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
    if argn.0 != 2 {
        return LbmValue(0);
    }

    let a = ffi::raw::lbm_dec_as_i32(*args);
    let b = ffi::raw::lbm_dec_as_i32(*args.add(1));
    ffi::raw::lbm_enc_i(crate::rust_add(a, b))
}

#[cfg(test)]
mod tests {
    use super::{LbmBindings, LbmValue, PackageLifecycle};
    use crate::ffi;
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
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
            value.0 as i32
        }

        unsafe fn encode_i32(&self, value: i32) -> LbmValue {
            LbmValue(value as usize)
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
}
