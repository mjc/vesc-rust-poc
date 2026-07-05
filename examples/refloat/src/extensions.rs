//! LispBM extensions required by Refloat's package loader.
//!
//! Refloat `v1.2.1` (`0ef6e99d8701`) defines `ext_set_fw_version` in
//! `src/main.c:2305-2313`, `ext_bms` in `src/main.c:2315-2331`, and registers
//! both names in `src/main.c:2458-2459`. The Lisp loader calls them immediately
//! after native load in `lisp/package.lisp:4-17`.

use core::ffi::CStr;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use vescpkg_rs::ffi;

const EXT_SET_FW_VERSION_NAME: &CStr = c"ext-set-fw-version";
const EXT_BMS_NAME: &CStr = c"ext-bms";
const PACKAGE_EXTENSION_COUNT: usize = 2;

static FW_VERSION_MAJOR: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_MINOR: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_BETA: AtomicI32 = AtomicI32::new(0);
static FW_VERSION_RECORDED: AtomicBool = AtomicBool::new(false);

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
#[unsafe(no_mangle)]
pub extern "C" fn ext_set_fw_version(args: *mut u32, argn: u32) -> u32 {
    if argn > 2 {
        let Some(args) = vescpkg_rs::lbm_args(args, argn) else {
            return 0;
        };
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            let lbm = vescpkg_rs::LbmApi::new(vescpkg_rs::RealBindings);
            record_refloat_firmware_version(args, |value| lbm.decode_i32(value));
        }
        #[cfg(any(test, not(target_arch = "arm")))]
        record_refloat_firmware_version(args, |value| value.0 as i32);
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    {
        vescpkg_rs::LbmApi::new(vescpkg_rs::RealBindings)
            .encode_true()
            .0
    }
    #[cfg(any(test, not(target_arch = "arm")))]
    {
        1
    }
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
    // `/Users/mjc/projects/refloat/src/main.c:2306-2310`; shorter calls still
    // return true at `/Users/mjc/projects/refloat/src/main.c:2311`.
    let [Some(major), Some(minor), Some(beta)] = [args.first(), args.get(1), args.get(2)] else {
        return;
    };

    FW_VERSION_MAJOR.store(decode_i32(*major), Ordering::Relaxed);
    FW_VERSION_MINOR.store(decode_i32(*minor), Ordering::Relaxed);
    FW_VERSION_BETA.store(decode_i32(*beta), Ordering::Relaxed);
    FW_VERSION_RECORDED.store(true, Ordering::Release);
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
#[unsafe(no_mangle)]
pub extern "C" fn ext_bms(_args: *mut u32, _argn: u32) -> u32 {
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        vescpkg_rs::LbmApi::new(vescpkg_rs::RealBindings)
            .encode_nil()
            .0
    }
    #[cfg(any(test, not(target_arch = "arm")))]
    {
        0
    }
}

/// Return the native extension descriptors required by upstream `package.lisp`.
pub fn package_extension_descriptors() -> [ffi::ExtensionDescriptor; PACKAGE_EXTENSION_COUNT] {
    [
        ffi::ExtensionDescriptor::new(EXT_SET_FW_VERSION_NAME, ext_set_fw_version as _),
        ffi::ExtensionDescriptor::new(EXT_BMS_NAME, ext_bms as _),
    ]
}

/// Register Refloat loader extensions through a package lifecycle.
///
/// Upstream registers these LispBM names and handlers at
/// `/Users/mjc/projects/refloat/src/main.c:2458-2459`.
///
/// This host/test helper models image-relative descriptor pointers. The target
/// registration below uses PC-relative runtime addresses instead, matching the
/// known-good ARM registration path.
pub fn register_refloat_loader_extensions_with<B: vescpkg_rs::LbmBindings>(
    info: &ffi::LibInfo,
    lifecycle: &vescpkg_rs::PackageLifecycle<B>,
) -> bool {
    lifecycle
        .register_extensions_from_image(
            vescpkg_rs::ffi::NativeImage::from_info(info),
            package_extension_descriptors(),
        )
        .is_ok()
}

/// Register Refloat's loader extensions with runtime names and handlers.
///
/// Upstream reaches this after custom config and app-data setup in
/// `/Users/mjc/projects/refloat/src/main.c:2456-2459`; Rust package init
/// reaches this after state install and runtime thread startup.
///
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_loader_extensions(info: *mut ffi::LibInfo) -> bool {
    let Some(info) = vescpkg_rs::loader_info_mut(info) else {
        return false;
    };
    let lifecycle = vescpkg_rs::PackageLifecycle::new(vescpkg_rs::RealBindings);
    register_refloat_loader_extensions_with(info, &lifecycle)
}

#[cfg(test)]
mod tests {
    use super::{
        EXT_BMS_NAME, EXT_SET_FW_VERSION_NAME, PACKAGE_EXTENSION_NAMES, RefloatFirmwareVersion,
        package_extension_descriptors, record_refloat_firmware_version,
        recorded_refloat_firmware_version, register_refloat_loader_extensions_with,
        reset_refloat_firmware_version,
    };
    use core::cell::Cell;
    use core::ffi::{CStr, c_char};
    use vescpkg_rs::ffi::{ExtensionHandler, LbmValue, LibInfo};
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
    fn refloat_loader_extension_registration_rebases_handlers_from_loaded_image() {
        let lifecycle = PackageLifecycle::new(RecordingLbmBindings::accepting());
        let info = LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x1000,
        };
        let descriptors = package_extension_descriptors();

        assert!(register_refloat_loader_extensions_with(&info, &lifecycle));

        let bindings = lifecycle.bindings();
        assert_eq!(bindings.add_calls.get(), 2);
        assert_eq!(
            bindings.name_addr(0),
            descriptors[0].name().as_ptr() as usize + 0x1000
        );
        assert_eq!(
            bindings.name_addr(1),
            descriptors[1].name().as_ptr() as usize + 0x1000
        );
        assert_eq!(
            bindings.handler_addr(0),
            descriptors[0].handler() as usize + 0x1000
        );
        assert_eq!(
            bindings.handler_addr(1),
            descriptors[1].handler() as usize + 0x1000
        );
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
        handlers: Cell<[usize; 2]>,
    }

    impl RecordingLbmBindings {
        fn accepting() -> Self {
            Self {
                add_calls: Cell::new(0),
                names: Cell::new([0; 2]),
                handlers: Cell::new([0; 2]),
            }
        }

        fn name(&self, index: usize) -> &CStr {
            let names = self.names.get();
            unsafe { CStr::from_ptr(names[index] as *const c_char) }
        }

        fn name_addr(&self, index: usize) -> usize {
            self.names.get()[index]
        }

        fn handler_addr(&self, index: usize) -> usize {
            self.handlers.get()[index]
        }
    }

    impl LbmBindings for RecordingLbmBindings {
        unsafe fn add_extension(&self, name: *const c_char, handler: ExtensionHandler) -> bool {
            let index = self.add_calls.get();
            let index = index.min(1);
            let mut names = self.names.get();
            names[index] = name as usize;
            self.names.set(names);
            let mut handlers = self.handlers.get();
            handlers[index] = handler as usize;
            self.handlers.set(handlers);
            self.add_calls.set(self.add_calls.get() + 1);
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

        fn encode_true(&self) -> LbmValue {
            LbmValue(1)
        }

        fn encode_nil(&self) -> LbmValue {
            LbmValue(0)
        }
    }
}
