use core::ffi::CStr;

use crate::ffi::{self};

#[cfg(test)]
use crate::ffi::{LbmApi, LbmBindings, LbmCount, LbmValue};

const EXT_RUST_PROBE_DIAG_NAME: &CStr = c"ext-rust-probe-diag-v4";
#[cfg(test)]
const EXT_RUST_PROBE_NAME: &CStr = c"ext-c-probe-v12";
const LBM_INT_TAG: u32 = 0x8;
#[cfg(test)]
const LBM_TAG_MASK: u32 = 0xf;
const LBM_VALUE_SHIFT: u32 = 4;

pub const PACKAGE_EXTENSION_NAMES: [&CStr; 1] = [EXT_RUST_PROBE_DIAG_NAME];

#[cfg(not(test))]
#[no_mangle]
/// Device probe: returns encoded LispBM integer 42 without calling firmware `lbm_enc_i`.
///
/// Host tests use the `#[cfg(test)]` build, which exercises argument validation through
/// `LbmApi` instead. Keep the device path minimal so PIC/staticlib codegen stays stable.
pub unsafe extern "C" fn ext_rust_probe_v12(_args: *mut u32, _argn: u32) -> u32 {
    encode_lbm_i32_raw(42)
}

#[cfg(test)]
#[no_mangle]
/// # Safety
///
/// `args` must point to at least `argn` initialized LispBM values when `argn > 0`.
pub unsafe extern "C" fn ext_rust_probe_v12(args: *mut u32, argn: u32) -> u32 {
    rust_probe_extension(
        &ffi::LbmApi::new(ffi::RealBindings),
        args.cast(),
        ffi::LbmCount(argn),
    )
    .0
}

#[cfg(test)]
fn rust_probe_extension<B: ffi::LbmBindings>(
    api: &ffi::LbmApi<B>,
    args: *mut ffi::LbmValue,
    argn: ffi::LbmCount,
) -> ffi::LbmValue {
    if argn.0 != 1 {
        return api.encode_eval_error();
    }

    let value = unsafe { *args };
    if value.0 & LBM_TAG_MASK != LBM_INT_TAG {
        return api.encode_eval_error();
    }

    let decoded = (value.0 as i32) >> LBM_VALUE_SHIFT;
    encode_lbm_i32(decoded.wrapping_mul(3))
}

fn encode_lbm_i32_raw(value: i32) -> u32 {
    value.wrapping_shl(LBM_VALUE_SHIFT) as u32 | LBM_INT_TAG
}

#[cfg(test)]
fn encode_lbm_i32(value: i32) -> ffi::LbmValue {
    ffi::LbmValue(encode_lbm_i32_raw(value))
}

#[cfg(test)]
pub fn rust_probe_descriptor() -> ffi::ExtensionDescriptor {
    ffi::ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, ext_rust_probe_v12)
}

pub fn rust_probe_diag_descriptor() -> ffi::ExtensionDescriptor {
    ffi::ExtensionDescriptor::new(EXT_RUST_PROBE_DIAG_NAME, ext_rust_probe_v12)
}

pub fn package_extension_descriptors() -> [ffi::ExtensionDescriptor; 1] {
    [rust_probe_diag_descriptor()]
}

pub fn register_loader_extensions<B: ffi::LbmBindings>(
    info: &ffi::LibInfo,
    lifecycle: &ffi::PackageLifecycle<B>,
) -> Result<(), ffi::RegisterError> {
    let image = ffi::NativeImage::from_info(info);
    lifecycle.register_extensions_from_image(image, package_extension_descriptors())
}

#[cfg(test)]
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
        register_loader_extensions, rust_add_extension_value, LbmApi, LbmBindings, LbmCount,
        LbmValue, EXT_RUST_PROBE_DIAG_NAME, EXT_RUST_PROBE_NAME, PACKAGE_EXTENSION_NAMES,
    };
    use crate::ffi::{self, ExtensionDescriptor, PackageLifecycle};
    use core::cell::Cell;
    use core::ffi::c_char;

    struct FakeBindings {
        add_calls: Cell<usize>,
        decode_calls: Cell<usize>,
        last_name: Cell<usize>,
        last_handler: Cell<usize>,
        add_result: Cell<bool>,
    }

    impl FakeBindings {
        fn new() -> Self {
            Self {
                add_calls: Cell::new(0),
                decode_calls: Cell::new(0),
                last_name: Cell::new(0),
                last_handler: Cell::new(0),
                add_result: Cell::new(true),
            }
        }

        fn rejecting() -> Self {
            Self {
                add_result: Cell::new(false),
                ..Self::new()
            }
        }
    }

    impl LbmBindings for FakeBindings {
        unsafe fn add_extension(
            &self,
            name: *const c_char,
            handler: ffi::ExtensionHandler,
        ) -> bool {
            self.add_calls.set(self.add_calls.get() + 1);
            self.last_name.set(name as usize);
            self.last_handler.set(handler as usize);
            self.add_result.get()
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

    unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
        0
    }

    #[test]
    fn register_loader_extensions_registers_every_descriptor_from_image() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert_eq!(register_loader_extensions(&info, &lifecycle), Ok(()));
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    }

    #[test]
    fn register_extension_from_image_rebases_handler_before_firmware_call() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let handler_offset = 0x31_usize;
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, unsafe {
            core::mem::transmute::<usize, ffi::ExtensionHandler>(handler_offset)
        });
        let image = ffi::NativeImage::new(0x2000);

        assert_eq!(
            lifecycle.register_extension_from_image(image, descriptor),
            Ok(())
        );
        assert_eq!(lifecycle.bindings().last_handler.get(), 0x2031);
    }

    #[test]
    fn registers_the_rust_extension_through_the_lifecycle_helper() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
        assert_eq!(
            EXT_RUST_PROBE_NAME.to_bytes_with_nul(),
            b"ext-c-probe-v12\0"
        );
    }

    #[test]
    fn package_extension_table_lists_every_rust_owned_extension() {
        assert_eq!(PACKAGE_EXTENSION_NAMES, [EXT_RUST_PROBE_DIAG_NAME]);
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
        assert_eq!(lifecycle.bindings().add_calls.get(), 0);
    }

    #[test]
    fn rejects_firmware_extension_registration_false() {
        let bindings = FakeBindings::rejecting();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(
            lifecycle.register_extension(descriptor),
            Err(ffi::RegisterError::FirmwareRejected)
        );
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    }

    #[test]
    fn repeated_registration_reports_each_firmware_result() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        assert_eq!(
            lifecycle.bindings().last_name.get(),
            EXT_RUST_PROBE_NAME.as_ptr() as usize
        );
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            stub_handler as *const () as usize
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
