//! LispBM extensions required by Refloat's package loader.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) defines `ext_set_fw_version` in
//! `src/main.c:2305-2313`, `ext_bms` in `src/main.c:2315-2331`, and registers
//! both names in `src/main.c:2458-2459`. The Lisp loader calls them immediately
//! after native load in `lisp/package.lisp:4-17`.

use core::ffi::CStr;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use vescpkg_rs::ffi;

#[cfg(all(not(test), target_arch = "arm"))]
use core::ffi::c_char;

const EXT_SET_FW_VERSION_NAME: &CStr = c"ext-set-fw-version";
const EXT_BMS_NAME: &CStr = c"ext-bms";
const PACKAGE_EXTENSION_COUNT: usize = 2;

static FW_VERSION_MAJOR: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_MINOR: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_BETA: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_RECORDED: AtomicBool = AtomicBool::new(false);

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
/// Upstream stores these components into `Data` at `src/main.c:2305-2311`.
/// The loader-only Rust candidate has no upstream `Data` allocation/`ARG`
/// install from `src/main.c:2419-2432`, so it stores only this narrow state.
///
/// # Safety
///
/// `args` and `argn` are supplied by LispBM and must follow the firmware
/// extension ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ext_set_fw_version(args: *mut u32, argn: u32) -> u32 {
    if argn > 2 && !args.is_null() {
        let args =
            unsafe { core::slice::from_raw_parts(args.cast::<ffi::LbmValue>(), argn as usize) };
        record_refloat_firmware_version(args, |value| unsafe { ffi::raw::lbm_dec_as_i32(value) });
    }
    unsafe { ffi::raw::lbm_enc_sym_true().0 }
}

/// Firmware version captured from Refloat's loader extension call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefloatFirmwareVersion {
    major: i32,
    minor: i32,
    beta: i32,
}

impl RefloatFirmwareVersion {
    /// Create a captured firmware-version tuple.
    pub const fn new(major: i32, minor: i32, beta: i32) -> Self {
        Self { major, minor, beta }
    }
}

fn record_refloat_firmware_version(
    args: &[ffi::LbmValue],
    decode_i32: impl Fn(ffi::LbmValue) -> i32,
) {
    // Refloat v1.2.1 only updates version state when `argn > 2` at
    // `src/main.c:2306-2310`; shorter calls still return true at `src/main.c:2311`.
    if args.len() > 2 {
        FW_VERSION_MAJOR.store(decode_i32(args[0]), Ordering::Relaxed);
        FW_VERSION_MINOR.store(decode_i32(args[1]), Ordering::Relaxed);
        FW_VERSION_BETA.store(decode_i32(args[2]), Ordering::Relaxed);
        FW_VERSION_RECORDED.store(true, Ordering::Release);
    }
}

/// Return the firmware version captured from `ext-set-fw-version`, if any.
pub fn recorded_refloat_firmware_version() -> Option<RefloatFirmwareVersion> {
    FW_VERSION_RECORDED
        .load(Ordering::Acquire)
        .then(|| RefloatFirmwareVersion {
            major: FW_VERSION_MAJOR.load(Ordering::Relaxed),
            minor: FW_VERSION_MINOR.load(Ordering::Relaxed),
            beta: FW_VERSION_BETA.load(Ordering::Relaxed),
        })
}

#[cfg(test)]
fn reset_refloat_firmware_version() {
    FW_VERSION_MAJOR.store(0, Ordering::Relaxed);
    FW_VERSION_MINOR.store(0, Ordering::Relaxed);
    FW_VERSION_BETA.store(0, Ordering::Relaxed);
    FW_VERSION_RECORDED.store(false, Ordering::Release);
}

/// Called from Refloat's Lisp loader and BMS polling loop.
///
/// Returns nil for now, matching a startup config with BMS integration disabled.
/// Upstream returns `d->float_conf.bms.enabled` at `src/main.c:2319-2331`; this
/// is an intentional containment divergence while the upstream EEPROM-backed
/// `Data.float_conf` state from `src/main.c:1190-1194` is not installed.
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
/// Upstream registers the same names after custom config and app-data setup in
/// `src/main.c:2456-2459`; Rust package init reaches this after state install
/// and runtime thread startup.
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
        EXT_BMS_NAME, EXT_SET_FW_VERSION_NAME, PACKAGE_EXTENSION_NAMES, RefloatFirmwareVersion,
        package_extension_descriptors, record_refloat_firmware_version,
        recorded_refloat_firmware_version, reset_refloat_firmware_version,
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

    #[test]
    fn ext_set_fw_version_records_three_decoded_components() {
        reset_refloat_firmware_version();

        // Refloat v1.2.1 stores firmware version only when `argn > 2` at
        // `src/main.c:2306-2310`; shorter calls still return true at
        // `src/main.c:2311`.
        record_refloat_firmware_version(&[LbmValue(6), LbmValue(5)], |value| value.0 as i32);
        assert_eq!(recorded_refloat_firmware_version(), None);

        record_refloat_firmware_version(&[LbmValue(6), LbmValue(2), LbmValue(0)], |value| {
            value.0 as i32
        });
        assert_eq!(
            recorded_refloat_firmware_version(),
            Some(RefloatFirmwareVersion::new(6, 2, 0))
        );
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
