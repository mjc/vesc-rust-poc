use core::ffi::CStr;

use crate::ffi::{self, ExtensionDescriptor, LbmApi, LbmBindings, LbmCount, LbmValue, NativeImage};

const EXT_RUST_PROBE_NAME: &CStr = c"ext-rust-probe-v5";

#[cfg(not(test))]
const PACKAGE_EXTENSIONS: [ExtensionDescriptor; 1] =
    [ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, ext_rust_add)];

pub const PACKAGE_EXTENSION_NAMES: [&CStr; 1] = [EXT_RUST_PROBE_NAME];

pub struct PackageLifecycle<B = ffi::RealBindings> {
    api: LbmApi<B>,
}

impl<B: LbmBindings> PackageLifecycle<B> {
    pub fn new(bindings: B) -> Self {
        Self {
            api: LbmApi::new(bindings),
        }
    }

    pub fn register_extension(
        &self,
        descriptor: ExtensionDescriptor,
    ) -> Result<i32, ffi::RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| ffi::RegisterError::InvalidExtensionName)?;
        let result = self
            .api
            .register_extension(descriptor.name(), descriptor.handler());
        if result < 0 {
            Err(ffi::RegisterError::FirmwareRejected(result))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn register_extension_from_image(
        &self,
        image: NativeImage,
        descriptor: ExtensionDescriptor,
    ) -> Result<i32, ffi::RegisterError> {
        // Image-owned C strings are offsets until the VESC loader applies
        // `lib_info.base_addr`; validating here would dereference an invalid
        // pre-rebase address on device.
        let result = unsafe {
            self.api
                .register_extension_from_image(image, descriptor.name(), descriptor.handler())
        };
        if result < 0 {
            Err(ffi::RegisterError::FirmwareRejected(result))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn register_extensions_from_image(
        &self,
        image: NativeImage,
        descriptors: &[ExtensionDescriptor],
    ) -> Result<(), ffi::RegisterError> {
        for descriptor in descriptors {
            unsafe {
                self.register_extension_from_image(image, *descriptor)?;
            }
        }
        Ok(())
    }

    #[cfg(not(test))]
    pub fn register_extensions(&self, image: NativeImage) -> Result<(), ffi::RegisterError> {
        unsafe { self.register_extensions_from_image(image, &PACKAGE_EXTENSIONS) }
    }
}

#[cfg(not(test))]
pub fn init_package(info: *const ffi::LibInfo) -> bool {
    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };

    let lifecycle = PackageLifecycle::new(ffi::RealBindings);
    lifecycle
        .register_extensions(NativeImage::from_info(info))
        .is_ok()
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
        rust_add_extension_value, ExtensionDescriptor, LbmApi, LbmBindings, LbmCount, LbmValue,
        PackageLifecycle, EXT_RUST_PROBE_NAME, PACKAGE_EXTENSION_NAMES,
    };
    use crate::ffi;
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        last_name: Cell<usize>,
        last_handler: Cell<usize>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                last_name: Cell::new(0),
                last_handler: Cell::new(0),
            }
        }
    }

    impl LbmBindings for FakeBindings {
        unsafe fn add_extension(&self, name: *const c_char, handler: ffi::ExtensionHandler) -> i32 {
            self.add_calls.set(self.add_calls.get() + 1);
            self.last_name.set(name as usize);
            self.last_handler.set(handler as usize);
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
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(17));
        assert_eq!(lifecycle.api.bindings().add_calls.get(), 1);
        assert_eq!(
            EXT_RUST_PROBE_NAME.to_bytes_with_nul(),
            b"ext-rust-probe-v5\0"
        );
    }

    #[test]
    fn package_extension_table_lists_every_rust_owned_extension() {
        assert_eq!(PACKAGE_EXTENSION_NAMES, [EXT_RUST_PROBE_NAME]);
        assert!(PACKAGE_EXTENSION_NAMES
            .iter()
            .all(|name| name.to_bytes().starts_with(b"ext-")));
    }

    #[test]
    fn rejects_non_extension_names_before_calling_firmware() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(c"rust-probe-v5", stub_handler);

        assert!(matches!(
            descriptor.validate(),
            Err(crate::ffi::ExtensionNameError::MissingExtPrefix)
        ));
        assert_eq!(
            lifecycle.register_extension(descriptor),
            Err(ffi::RegisterError::InvalidExtensionName)
        );
        assert_eq!(lifecycle.api.bindings().add_calls.get(), 0);
    }

    #[test]
    fn registration_table_rebases_names_and_callbacks_from_the_native_image() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let image = ffi::NativeImage::new(0x2000);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(
            unsafe { lifecycle.register_extensions_from_image(image, &[descriptor]) },
            Ok(())
        );
        assert_eq!(
            lifecycle.api.bindings().last_name.get(),
            EXT_RUST_PROBE_NAME.as_ptr() as usize + 0x2000
        );
        assert_eq!(
            lifecycle.api.bindings().last_handler.get(),
            stub_handler as *const () as usize + 0x2000
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
