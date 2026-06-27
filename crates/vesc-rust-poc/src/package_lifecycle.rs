use core::ffi::CStr;

use crate::ffi::{self, ExtensionDescriptor, LbmApi, LbmBindings};

#[cfg(test)]
use crate::ffi::{LbmCount, LbmValue};

const EXT_RUST_PROBE_DIAG_NAME: &CStr = c"ext-rust-probe-diag-v4";
#[cfg(test)]
const EXT_RUST_PROBE_NAME: &CStr = c"ext-c-probe-v12";
#[cfg(test)]
const LBM_INT_TAG: u32 = 0x8;
#[cfg(test)]
const LBM_TAG_MASK: u32 = 0xf;
#[cfg(test)]
const LBM_VALUE_SHIFT: u32 = 4;

pub const PACKAGE_EXTENSION_NAMES: [&CStr; 1] = [EXT_RUST_PROBE_DIAG_NAME];

#[cfg(not(test))]
core::arch::global_asm!(
    r#"
    .section .program_ptr,"aw",%progbits
    .global prog_ptr
    .type prog_ptr, %object
    .balign 4
prog_ptr:
    .word   0
    .size prog_ptr, . - prog_ptr

    .section .init_fun,"ax",%progbits
    .thumb
    .thumb_func
    .global init
    .type init, %function
init:
    push    {{r3, lr}}
    adr     r3, prog_ptr
    ldr     r3, [r3]
    bl      package_lib_init
    cbz     r0, .Linit_done
    ldr     r3, .Linit_vesc_if_base
    adr     r1, ext_rust_probe_v12
    adr     r0, .Linit_rust_probe_name
    ldr     r3, [r3, #0]
    blx     r3
    movs    r0, #1
.Linit_done:
    pop     {{r3, pc}}

    .balign 4
.Linit_vesc_if_base:
    .word   0x1000f800

    .balign 4
.Linit_rust_probe_name:
    .asciz  "ext-rust-probe-diag-v4"
    .size init, . - init
"#
);

#[cfg(not(test))]
unsafe extern "C" {
    pub fn ext_rust_probe_v12(args: *mut u32, argn: u32) -> u32;
}

#[cfg(not(test))]
core::arch::global_asm!(
    r#"
    .section .text.ext_rust_probe_v12,"ax",%progbits
    .thumb
    .thumb_func
    .global ext_rust_probe_v12
    .type ext_rust_probe_v12, %function
ext_rust_probe_v12:
    cmp     r1, #1
    push    {{r4, r5, r6, lr}}
    ldr     r4, .Lvesc_if_base_probe
    mov     r5, r0
    bne     .Lprobe_eerror
    ldr     r3, [r4, #124]
    ldr     r0, [r0, #0]
    blx     r3
    cbz     r0, .Lprobe_eerror
    ldr     r3, [r4, #100]
    ldr     r0, [r5, #0]
    ldr     r6, [r4, #64]
    blx     r3
    mov     r3, r6
    add.w   r0, r0, r0, lsl #1
    pop     {{r4, r5, r6, lr}}
    bx      r3
.Lprobe_eerror:
    ldr.w   r0, [r4, #148]
    pop     {{r4, r5, r6, pc}}

    .balign 4
.Lvesc_if_base_probe:
    .word   0x1000f800
    .size ext_rust_probe_v12, . - ext_rust_probe_v12
"#
);

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

#[cfg(test)]
fn encode_lbm_i32(value: i32) -> ffi::LbmValue {
    ffi::LbmValue(value.wrapping_shl(LBM_VALUE_SHIFT) as u32 | LBM_INT_TAG)
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
    ) -> Result<(), ffi::RegisterError> {
        let descriptor = descriptor
            .validate()
            .map_err(|_| ffi::RegisterError::InvalidExtensionName)?;
        if self
            .api
            .register_extension(descriptor.name(), descriptor.handler())
        {
            Ok(())
        } else {
            Err(ffi::RegisterError::FirmwareRejected)
        }
    }
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
        rust_add_extension_value, ExtensionDescriptor, LbmApi, LbmBindings, LbmCount, LbmValue,
        PackageLifecycle, EXT_RUST_PROBE_DIAG_NAME, EXT_RUST_PROBE_NAME, PACKAGE_EXTENSION_NAMES,
    };
    use crate::ffi;
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
    fn registers_the_rust_extension_through_the_lifecycle_helper() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        assert_eq!(lifecycle.api.bindings().add_calls.get(), 1);
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
        assert_eq!(lifecycle.api.bindings().add_calls.get(), 0);
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
        assert_eq!(lifecycle.api.bindings().add_calls.get(), 1);
    }

    #[test]
    fn repeated_registration_reports_each_firmware_result() {
        let bindings = FakeBindings::new();
        let lifecycle = PackageLifecycle::new(bindings);
        let descriptor = ExtensionDescriptor::new(EXT_RUST_PROBE_NAME, stub_handler);

        assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
        assert_eq!(
            lifecycle.api.bindings().last_name.get(),
            EXT_RUST_PROBE_NAME.as_ptr() as usize
        );
        assert_eq!(
            lifecycle.api.bindings().last_handler.get(),
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
