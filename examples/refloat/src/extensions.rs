//! LispBM extensions required by Refloat's package loader.

use core::ffi::CStr;

use vescpkg_rs::ffi;

#[cfg(all(not(test), target_arch = "arm"))]
use core::ffi::c_char;

const EXT_SET_FW_VERSION_NAME: &CStr = c"ext-set-fw-version";
const EXT_BMS_NAME: &CStr = c"ext-bms";
const PACKAGE_EXTENSION_COUNT: usize = 2;

#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(link_section = ".text.refloat_ext_names")]
static EXT_SET_FW_VERSION_NAME_BYTES: [u8; 19] = *b"ext-set-fw-version\0";

#[cfg(all(not(test), target_arch = "arm"))]
#[used]
#[unsafe(link_section = ".text.refloat_ext_names")]
static EXT_BMS_NAME_BYTES: [u8; 8] = *b"ext-bms\0";

/// Extension names exported by the Refloat package loader.
pub const PACKAGE_EXTENSION_NAMES: [&CStr; PACKAGE_EXTENSION_COUNT] =
    [EXT_SET_FW_VERSION_NAME, EXT_BMS_NAME];

const _: () = assert!(PACKAGE_EXTENSION_COUNT == 2);

/// Called from Refloat's Lisp loader to pass firmware version components.
///
/// # Safety
///
/// `args` and `argn` are supplied by LispBM and must follow the firmware
/// extension ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ext_set_fw_version(_args: *mut u32, _argn: u32) -> u32 {
    unsafe { ffi::raw::lbm_enc_sym_true().0 }
}

/// Called from Refloat's Lisp loader and BMS polling loop.
///
/// Returns nil for now, matching a startup config with BMS integration disabled.
///
/// # Safety
///
/// `args` and `argn` are supplied by LispBM and must follow the firmware
/// extension ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ext_bms(_args: *mut u32, _argn: u32) -> u32 {
    unsafe { ffi::raw::lbm_enc_sym_nil().0 }
}

/// Return the native extension descriptors required by upstream `package.lisp`.
pub fn package_extension_descriptors() -> [ffi::ExtensionDescriptor; PACKAGE_EXTENSION_COUNT] {
    [
        ffi::ExtensionDescriptor::new(EXT_SET_FW_VERSION_NAME, ext_set_fw_version),
        ffi::ExtensionDescriptor::new(EXT_BMS_NAME, ext_bms),
    ]
}

/// Register Refloat's loader extensions with image-rebased native handlers.
///
/// # Safety
///
/// `info` must describe the loaded native image that owns every descriptor
/// handler. The registered handlers must remain valid while firmware may call
/// the LispBM extensions.
#[cfg(all(not(test), target_arch = "arm"))]
pub unsafe fn register_refloat_loader_extensions(_info: *mut ffi::LibInfo) -> bool {
    unsafe {
        ffi::raw::lbm_add_extension(
            runtime_ext_set_fw_version_name(),
            runtime_ext_set_fw_version_handler(),
        ) && ffi::raw::lbm_add_extension(runtime_ext_bms_name(), runtime_ext_bms_handler())
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_ext_set_fw_version_name() -> *const c_char {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {name}",
            address = out(reg) address,
            name = sym EXT_SET_FW_VERSION_NAME_BYTES,
            options(nomem, nostack, preserves_flags),
        );
    }
    address as *const c_char
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_ext_bms_name() -> *const c_char {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {name}",
            address = out(reg) address,
            name = sym EXT_BMS_NAME_BYTES,
            options(nomem, nostack, preserves_flags),
        );
    }
    address as *const c_char
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_ext_set_fw_version_handler() -> ffi::ExtensionHandler {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {handler}",
            address = out(reg) address,
            handler = sym ext_set_fw_version,
            options(nomem, nostack, preserves_flags),
        );
        core::mem::transmute::<usize, ffi::ExtensionHandler>(address | 1)
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
fn runtime_ext_bms_handler() -> ffi::ExtensionHandler {
    let address: usize;
    unsafe {
        core::arch::asm!(
            "adr.w {address}, {handler}",
            address = out(reg) address,
            handler = sym ext_bms,
            options(nomem, nostack, preserves_flags),
        );
        core::mem::transmute::<usize, ffi::ExtensionHandler>(address | 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EXT_BMS_NAME, EXT_SET_FW_VERSION_NAME, PACKAGE_EXTENSION_NAMES,
        package_extension_descriptors,
    };
    use core::cell::Cell;
    use core::ffi::{CStr, c_char};
    use vescpkg_rs::ffi::{ExtensionHandler, LbmValue};
    use vescpkg_rs::{LbmBindings, PackageLifecycle};

    #[test]
    fn extension_table_lists_official_refloat_loader_extensions() {
        let descriptors = package_extension_descriptors();

        assert_eq!(
            PACKAGE_EXTENSION_NAMES,
            [EXT_SET_FW_VERSION_NAME, EXT_BMS_NAME]
        );
        assert_eq!(descriptors[0].name(), EXT_SET_FW_VERSION_NAME);
        assert_eq!(descriptors[1].name(), EXT_BMS_NAME);
    }

    #[test]
    fn package_lifecycle_registers_official_refloat_loader_extensions() {
        let lifecycle = PackageLifecycle::new(RecordingLbmBindings::accepting());

        for descriptor in package_extension_descriptors() {
            assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        }

        let bindings = lifecycle.bindings();
        assert_eq!(bindings.add_calls.get(), 2);
        assert_eq!(bindings.name(0), EXT_SET_FW_VERSION_NAME);
        assert_eq!(bindings.name(1), EXT_BMS_NAME);
    }

    struct RecordingLbmBindings {
        add_calls: Cell<usize>,
        names: Cell<[usize; 2]>,
    }

    impl RecordingLbmBindings {
        fn accepting() -> Self {
            Self {
                add_calls: Cell::new(0),
                names: Cell::new([0; 2]),
            }
        }

        fn name(&self, index: usize) -> &CStr {
            let names = self.names.get();
            unsafe { CStr::from_ptr(names[index] as *const c_char) }
        }
    }

    impl LbmBindings for RecordingLbmBindings {
        unsafe fn add_extension(&self, name: *const c_char, _handler: ExtensionHandler) -> bool {
            let index = self.add_calls.get();
            let mut names = self.names.get();
            names[index.min(1)] = name as usize;
            self.names.set(names);
            self.add_calls.set(index + 1);
            true
        }

        unsafe fn decode_i32(&self, value: LbmValue) -> i32 {
            value.0 as i32
        }

        unsafe fn encode_i32(&self, value: i32) -> LbmValue {
            LbmValue(value as u32)
        }

        unsafe fn is_number(&self, _value: LbmValue) -> bool {
            true
        }

        unsafe fn encode_eval_error(&self) -> LbmValue {
            LbmValue(0)
        }
    }
}
