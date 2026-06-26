use core::ffi::CStr;

use crate::ffi::{self, LbmApi, LbmBindings, LbmCount, LbmValue, NativeImage};

const EXT_RUST_PROBE_NAME: &CStr = c"ext-rust-probe-v5";

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
        self.api.register_extension(EXT_RUST_PROBE_NAME, handler)
    }

    pub fn register_extensions_from_image(
        &self,
        image: NativeImage,
        handler: ffi::ExtensionHandler,
    ) -> i32 {
        self.api
            .register_extension_from_image(image, EXT_RUST_PROBE_NAME, handler)
    }

    #[cfg(not(test))]
    pub fn register_extensions(&self, image: NativeImage) -> i32 {
        self.register_extensions_from_image(image, ext_rust_add)
    }
}

#[cfg(not(test))]
pub fn init_package(info: *const ffi::LibInfo) {
    let Some(info) = (unsafe { info.as_ref() }) else {
        return;
    };

    let lifecycle = PackageLifecycle::new(ffi::RealBindings);
    let _ = lifecycle.register_extensions(NativeImage::from_info(info));
}

#[cfg(not(test))]
unsafe extern "C" fn ext_rust_add(args: *mut LbmValue, argn: LbmCount) -> LbmValue {
    rust_add_extension_value(&LbmApi::new(ffi::RealBindings), args, argn)
}

fn rust_add_extension_value<B: LbmBindings>(
    api: &LbmApi<B>,
    _args: *mut LbmValue,
    _argn: LbmCount,
) -> LbmValue {
    api.encode_i32(crate::rust_add(20, 22))
}

#[cfg(test)]
mod tests {
    use super::{
        rust_add_extension_value, LbmApi, LbmBindings, LbmCount, LbmValue, PackageLifecycle,
        EXT_RUST_PROBE_NAME,
    };
    use crate::ffi;
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
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
            true
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
        assert_eq!(
            EXT_RUST_PROBE_NAME.to_bytes_with_nul(),
            b"ext-rust-probe-v5\0"
        );
    }

    #[test]
    fn rust_add_extension_returns_a_constant_encoded_probe_value() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20), LbmValue(22)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(2)),
            LbmValue(42)
        );
    }

    #[test]
    fn rust_add_extension_does_not_depend_on_live_argument_shape() {
        let api = LbmApi::new(FakeBindings::new());
        let mut args = [LbmValue(20)];

        assert_eq!(
            rust_add_extension_value(&api, args.as_mut_ptr(), LbmCount(1)),
            LbmValue(42)
        );
        assert_eq!(
            rust_add_extension_value(&api, core::ptr::null_mut(), LbmCount(2)),
            LbmValue(42)
        );
    }
}
