//! Separate VESC Express native-library ABI foundation.
//!
//! Express is intentionally not projected into the STM32 `VescIf` table. This
//! module currently provides the pinned 32-bit table shape, version/magic
//! constants, scalar/function slot classification, and a fail-closed borrowed
//! loader. Typed callable wrappers can build on this boundary without mixing
//! slot order or target pointers with STM32.

mod container;
mod flat;
mod functions;
mod image;
mod lisp;
mod loader;
mod memory;
mod runtime;
mod sync;
mod table;
mod types;

pub use container::{
    ExpressNativeContainer, ExpressNativeContainerError, ExpressRelocation, ExpressRelocationIter,
};
pub use flat::{ExpressFlatValue, ExpressFlatValueError, ExpressLispMessageError};
pub use functions::*;
pub use image::{
    ExpressNativeImage, ExpressNativeImageError, ExpressNativeXipError, ExpressNativeXipImage,
};
pub use lisp::{ExpressLisp, ExpressLispSymbol, ExpressLispValue};
pub use loader::{ExpressCallError, ExpressInterface, ExpressLoadError};
pub use memory::{ExpressAllocation, ExpressAllocationError};
pub use runtime::ExpressRuntime;
pub use sync::{ExpressMutex, ExpressMutexGuard, ExpressSemaphore, ExpressSyncError};
pub use table::{ExpressSlot, ExpressSlotKind, ExpressTable, ExpressTableError, express_slot_kind};
pub use types::{
    EXPRESS_C_IF_VERSION, EXPRESS_IF_SLOT_COUNT, EXPRESS_IF_TABLE_BYTES, EXPRESS_NATIVE_LIB_MAGIC,
    EXPRESS_NATIVE_LIB_RELOC_MAGIC, EXPRESS_SYSTEM_TICK_RATE_HZ, ExpressAddress,
    ExpressFlatValueRaw, ExpressLibInfo, ExpressNativeLoadKind, ExpressTarget, ExpressWord,
};

/// Define the Express native-library image entrypoint for an initializer.
///
/// The caller receives a validated mutable view of the loader-owned
/// [`ExpressLibInfo`]. Returning `true` tells firmware that the library is
/// live; the initializer must set `stop_fun` before doing so. The generated
/// symbols are placed in the sections consumed by the Express loader. This
/// macro only defines the ABI entry seam; target-specific linker scripts and
/// package generation remain outside this crate.
#[macro_export]
macro_rules! express_native_start {
    ($start:path) => {
        #[used]
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".program_ptr")]
        pub static prog_ptr: u32 = 0;

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".init_fun")]
        pub extern "C" fn init(info: *mut $crate::express::ExpressLibInfo) -> bool {
            if info.is_null() {
                return false;
            }
            if !unsafe { $start(&mut *info) } {
                return false;
            }
            unsafe { (*info).stop_fun.is_some() }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn express_noop(_: *mut core::ffi::c_void) {}

    unsafe extern "C" fn express_lisp_handler(_: *mut u32, _: u32) -> u32 {
        0
    }

    fn express_start(info: &mut ExpressLibInfo) -> bool {
        info.stop_fun = Some(express_noop);
        info.arg = core::ptr::null_mut();
        info.base_addr != 0
    }

    express_native_start!(express_start);

    #[test]
    fn pinned_v1_shape_is_independent_from_stm32() {
        assert_eq!(EXPRESS_C_IF_VERSION, 1);
        assert_eq!(EXPRESS_IF_SLOT_COUNT, 80);
        assert_eq!(EXPRESS_IF_TABLE_BYTES, 320);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(core::mem::size_of::<ExpressFlatValueRaw>(), 12);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(core::mem::size_of::<ExpressFlatValueRaw>(), 16);
        assert_eq!(core::mem::offset_of!(ExpressFlatValueRaw, buf), 0);
        assert_eq!(
            core::mem::offset_of!(ExpressFlatValueRaw, buf_size),
            core::mem::size_of::<*mut u8>()
        );
        assert_eq!(
            core::mem::offset_of!(ExpressFlatValueRaw, buf_pos),
            core::mem::size_of::<*mut u8>() + 4
        );
        assert_eq!(express_slot_kind(0), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(38), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(42), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(43), Some(ExpressSlotKind::Function));
        assert_eq!(express_slot_kind(80), None);
        assert_eq!(ExpressSlot::IfVersion.index(), 0);
        assert_eq!(ExpressSlot::LbmAddExtension.index(), 1);
        assert_eq!(ExpressSlot::LbmEncSymNil.index(), 38);
        assert_eq!(ExpressSlot::SemReset.index(), 79);
        for (index, slot) in ExpressSlot::ALL.into_iter().enumerate() {
            assert_eq!(slot.index(), index);
            assert!(express_slot_kind(index).is_some());
        }
        assert_eq!(ExpressTarget::Esp32C3.interface_address(), 0x3FCD_BE00);
        assert_eq!(ExpressTarget::Esp32S3.interface_address(), 0x3FCE_8800);
        assert_eq!(ExpressTarget::Esp32C6.interface_address(), 0x4087_B800);
        assert_eq!(ExpressTarget::Esp32P4.interface_address(), 0x4FF3_A000);
        assert_eq!(ExpressTarget::Esp32C3.target_name(), "esp32c3");
        assert_eq!(
            ExpressTarget::Esp32S3.native_load_kind(),
            ExpressNativeLoadKind::Relocatable
        );
        assert_eq!(
            ExpressTarget::Esp32C3.native_load_kind(),
            ExpressNativeLoadKind::Xip
        );
        assert_eq!(
            ExpressTarget::from_target_name("esp32s3"),
            Some(ExpressTarget::Esp32S3)
        );
        assert_eq!(ExpressTarget::from_target_name("esp32"), None);
        assert_eq!(
            ExpressTarget::Esp32S3.sdkconfig_define(),
            "CONFIG_IDF_TARGET_ESP32S3"
        );
        assert_eq!(
            ExpressTarget::from_sdkconfig_define("CONFIG_IDF_TARGET_ESP32P4"),
            Some(ExpressTarget::Esp32P4)
        );
        assert_eq!(
            ExpressTarget::from_sdkconfig_define("CONFIG_IDF_TARGET_ESP32"),
            None
        );
        let pointer_size = core::mem::size_of::<usize>();
        assert_eq!(core::mem::offset_of!(ExpressLibInfo, stop_fun), 0);
        assert_eq!(core::mem::offset_of!(ExpressLibInfo, arg), pointer_size);
        assert_eq!(
            core::mem::offset_of!(ExpressLibInfo, base_addr),
            pointer_size * 2
        );
        assert_eq!(
            core::mem::size_of::<ExpressLibInfo>(),
            (pointer_size * 2 + 4).next_multiple_of(pointer_size)
        );
    }

    #[test]
    fn native_start_checks_info_and_forwards_loader_metadata() {
        assert!(!init(core::ptr::null_mut()));

        let mut info = ExpressLibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        assert!(init(&mut info));
        assert!(info.stop_fun.is_some());
    }

    #[test]
    fn loader_checks_version_before_exposing_slots() {
        assert_eq!(ExpressTable::load(&[]), Err(ExpressTableError::Empty));
        assert_eq!(
            ExpressTable::load(&[2]),
            Err(ExpressTableError::VersionMismatch {
                expected: EXPRESS_C_IF_VERSION,
                found: 2,
            })
        );

        let table = ExpressTable::load(&[EXPRESS_C_IF_VERSION, 0, 0x1234]).unwrap();
        assert_eq!(table.version(), EXPRESS_C_IF_VERSION);
        assert_eq!(table.len(), 3);
        assert_eq!(table.function_address(1), None);
        assert_eq!(table.function_address(2), Some(ExpressAddress::new(0x1234)));
        assert_eq!(
            table.function_address_at(ExpressSlot::LbmSetErrorReason),
            Some(ExpressAddress::new(0x1234))
        );
        assert_eq!(table.word(2), Some(ExpressWord::new(0x1234)));
        assert_eq!(
            table.word_at(ExpressSlot::LbmSetErrorReason),
            Some(ExpressWord::new(0x1234))
        );
        assert!(!table.is_complete());

        let interface = ExpressInterface::from_words(&[EXPRESS_C_IF_VERSION, 0, 0x1234]).unwrap();
        assert!(interface.has_slot(ExpressSlot::LbmSetErrorReason));
        assert!(!interface.has_slot(ExpressSlot::SleepMs));
        assert!(!interface.has_function(ExpressSlot::LbmAddExtension));
        assert_eq!(
            interface.function_address(ExpressSlot::LbmSetErrorReason),
            Some(ExpressAddress::new(0x1234))
        );
        let _raw_function = unsafe {
            interface
                .function::<unsafe extern "C" fn(u32)>(ExpressSlot::LbmSetErrorReason)
                .unwrap()
        };
        assert_eq!(
            unsafe { interface.function::<unsafe extern "C" fn()>(ExpressSlot::SleepMs) },
            Err(ExpressCallError {
                slot: ExpressSlot::SleepMs
            })
        );
        let runtime = unsafe { ExpressRuntime::from_interface(interface) };
        assert_eq!(
            runtime.system_time(),
            Err(ExpressCallError {
                slot: ExpressSlot::SystemTime
            })
        );
        assert!(matches!(
            ExpressMutex::new(runtime),
            Err(ExpressSyncError::Unavailable(ExpressCallError {
                slot: ExpressSlot::MutexCreate
            }))
        ));
        assert!(matches!(
            ExpressSemaphore::new(runtime),
            Err(ExpressSyncError::Unavailable(ExpressCallError {
                slot: ExpressSlot::SemCreate
            }))
        ));
        assert!(matches!(
            ExpressAllocation::new(runtime, 1),
            Err(ExpressAllocationError::Unavailable(ExpressCallError {
                slot: ExpressSlot::Malloc
            }))
        ));
        assert!(matches!(
            ExpressAllocation::new(runtime, 0),
            Err(ExpressAllocationError::ZeroSize)
        ));
        assert_eq!(
            unsafe { runtime.request_terminate(core::ptr::null_mut()) },
            Err(ExpressCallError {
                slot: ExpressSlot::RequestTerminate
            })
        );
        assert_eq!(
            unsafe { runtime.spawn(express_noop, 256, core::ptr::null(), core::ptr::null_mut()) },
            Err(ExpressCallError {
                slot: ExpressSlot::Spawn
            })
        );
        assert_eq!(
            unsafe { runtime.get_arg(0) },
            Err(ExpressCallError {
                slot: ExpressSlot::GetArg
            })
        );
        let lisp = unsafe { ExpressLisp::from_interface(interface) };
        assert_eq!(
            lisp.enc_i(1),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmEncI
            })
        );
        assert_eq!(
            lisp.is_number(ExpressLispValue::new(0)),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmIsNumber
            })
        );
        assert_eq!(
            lisp.symbol_nil(),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmEncSymNil
            })
        );
        assert_eq!(
            lisp.current_cid(),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmGetCurrentCid
            })
        );
        assert_eq!(
            unsafe { lisp.dec_str(ExpressLispValue::new(0)) },
            Err(ExpressCallError {
                slot: ExpressSlot::LbmDecStr
            })
        );
        assert_eq!(
            lisp.is_symbol_nil(ExpressLispSymbol::new(0)),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmIsSymbolNil
            })
        );
        assert_eq!(
            lisp.is_symbol_true(ExpressLispSymbol::new(0)),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmIsSymbolTrue
            })
        );
        assert_eq!(
            lisp.symbol_merror(),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmEncSymMerror
            })
        );
        assert!(matches!(
            lisp.start_flatten(16),
            Err(ExpressFlatValueError::Unavailable(ExpressCallError {
                slot: ExpressSlot::LbmStartFlatten
            }))
        ));
        assert_eq!(
            lisp.eval_is_paused(),
            Err(ExpressCallError {
                slot: ExpressSlot::LbmEvalIsPaused
            })
        );
        assert_eq!(
            unsafe { lisp.add_extension(core::ptr::null_mut(), express_lisp_handler) },
            Err(ExpressCallError {
                slot: ExpressSlot::LbmAddExtension
            })
        );
        assert_eq!(
            unsafe { lisp.create_byte_array(core::ptr::null_mut(), 1) },
            Err(ExpressCallError {
                slot: ExpressSlot::LbmCreateByteArray
            })
        );
    }

    #[test]
    fn appended_slots_are_optional_without_reinterpretation() {
        let mut words = [0; EXPRESS_IF_SLOT_COUNT];
        words[0] = EXPRESS_C_IF_VERSION;
        let table = ExpressTable::load(&words).unwrap();
        assert!(table.is_complete());
        assert_eq!(table.function_address(79), None);
        assert_eq!(table.word(81), None);
    }

    fn sample_relocatable_container(relocation: u32) -> [u8; 40] {
        let mut bytes = [0; 40];
        bytes[..4].copy_from_slice(&EXPRESS_NATIVE_LIB_RELOC_MAGIC.to_be_bytes());
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&4_u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&8_u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&0_u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&relocation.to_le_bytes());
        bytes[28..32].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        bytes[32..40].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe, 1, 2, 3, 4]);
        bytes
    }

    #[test]
    fn relocatable_container_exposes_checked_regions_and_relocations() {
        let bytes = sample_relocatable_container(0xc000_0000);
        let container = ExpressNativeContainer::parse(&bytes).unwrap();

        assert_eq!(container.version(), 2);
        assert_eq!(container.code_size(), 4);
        assert_eq!(container.data_size(), 8);
        assert_eq!(container.entry_offset(), 0);
        assert_eq!(container.relocation_count(), 1);
        assert_eq!(container.encoded_len(), bytes.len());
        assert_eq!(container.code(), &[0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(container.data(), &[0xca, 0xfe, 0xba, 0xbe, 1, 2, 3, 4]);

        let relocation = container.relocation(0).unwrap();
        assert!(relocation.patches_data());
        assert!(relocation.targets_code());
        assert_eq!(relocation.offset(), 0);
        assert!(container.relocation(1).is_none());
        assert_eq!(
            container.relocations().collect::<std::vec::Vec<_>>(),
            [relocation]
        );
    }

    #[test]
    fn relocatable_container_rejects_bad_header_and_region_metadata() {
        assert_eq!(
            ExpressNativeContainer::parse(&[0; 23]),
            Err(ExpressNativeContainerError::Truncated)
        );

        let mut bytes = sample_relocatable_container(0);
        bytes[..4].copy_from_slice(&EXPRESS_NATIVE_LIB_MAGIC.to_be_bytes());
        assert_eq!(
            ExpressNativeContainer::parse(&bytes),
            Err(ExpressNativeContainerError::InvalidMagic {
                found: EXPRESS_NATIVE_LIB_MAGIC
            })
        );

        let mut bytes = sample_relocatable_container(0);
        bytes[4..8].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            ExpressNativeContainer::parse(&bytes),
            Err(ExpressNativeContainerError::UnsupportedVersion { found: 1 })
        );

        let mut bytes = sample_relocatable_container(0);
        bytes[8..12].copy_from_slice(&3_u32.to_le_bytes());
        assert_eq!(
            ExpressNativeContainer::parse(&bytes),
            Err(ExpressNativeContainerError::InvalidCodeSize { found: 3 })
        );

        let mut bytes = sample_relocatable_container(0);
        bytes[16..20].copy_from_slice(&4_u32.to_le_bytes());
        assert_eq!(
            ExpressNativeContainer::parse(&bytes),
            Err(ExpressNativeContainerError::InvalidEntryOffset { found: 4 })
        );
    }

    #[test]
    fn relocatable_container_rejects_truncation_and_out_of_bounds_relocations() {
        let bytes = sample_relocatable_container(0);
        assert_eq!(
            ExpressNativeContainer::parse(&bytes[..39]),
            Err(ExpressNativeContainerError::Truncated)
        );

        let bytes = sample_relocatable_container(0x4000_0008);
        assert_eq!(
            ExpressNativeContainer::parse(&bytes),
            Err(ExpressNativeContainerError::InvalidRelocation {
                index: 0,
                offset: 8
            })
        );
    }

    fn sample_xip_image(magic: u32) -> [u8; 13] {
        let mut bytes = [0; 13];
        bytes[..4].copy_from_slice(&magic.to_be_bytes());
        bytes[8..].copy_from_slice(&[0x10, 0x20, 0x30, 0x40, 0x50]);
        bytes
    }

    #[test]
    fn target_selected_image_accepts_xip_and_relocatable_forms() {
        let xip_bytes = sample_xip_image(EXPRESS_NATIVE_LIB_MAGIC);
        let xip = ExpressNativeImage::parse(ExpressTarget::Esp32C3, &xip_bytes).unwrap();
        assert_eq!(xip.load_kind(), ExpressNativeLoadKind::Xip);
        assert_eq!(xip.encoded_len(), xip_bytes.len());
        assert_eq!(xip.entry_offset(), 8);
        let ExpressNativeImage::Xip(image) = xip else {
            panic!("C3 must select XIP");
        };
        assert_eq!(image.code(), &[0x10, 0x20, 0x30, 0x40, 0x50]);

        let reloc_bytes = sample_relocatable_container(0);
        let reloc = ExpressNativeImage::parse(ExpressTarget::Esp32S3, &reloc_bytes).unwrap();
        assert_eq!(reloc.load_kind(), ExpressNativeLoadKind::Relocatable);
        assert_eq!(reloc.encoded_len(), reloc_bytes.len());
        assert_eq!(reloc.entry_offset(), 0);
        assert!(matches!(reloc, ExpressNativeImage::Relocatable(_)));
    }

    #[test]
    fn target_selected_image_rejects_the_other_loader_format() {
        let reloc_bytes = sample_relocatable_container(0);
        assert_eq!(
            ExpressNativeImage::parse(ExpressTarget::Esp32C6, &reloc_bytes),
            Err(ExpressNativeImageError::Xip(
                ExpressNativeXipError::InvalidMagic {
                    found: EXPRESS_NATIVE_LIB_RELOC_MAGIC,
                }
            ))
        );

        let mut xip_bytes = [0; 24];
        xip_bytes[..13].copy_from_slice(&sample_xip_image(EXPRESS_NATIVE_LIB_MAGIC));
        assert_eq!(
            ExpressNativeImage::parse(ExpressTarget::Esp32S3, &xip_bytes),
            Err(ExpressNativeImageError::Relocatable(
                ExpressNativeContainerError::InvalidMagic {
                    found: EXPRESS_NATIVE_LIB_MAGIC,
                }
            ))
        );
    }

    #[test]
    fn xip_image_matches_loader_length_and_magic_checks() {
        assert_eq!(
            ExpressNativeXipImage::parse(&[0; 12]),
            Err(ExpressNativeXipError::Truncated)
        );
        let mut bytes = sample_xip_image(EXPRESS_NATIVE_LIB_MAGIC);
        bytes[..4].copy_from_slice(&EXPRESS_NATIVE_LIB_RELOC_MAGIC.to_be_bytes());
        assert_eq!(
            ExpressNativeXipImage::parse(&bytes),
            Err(ExpressNativeXipError::InvalidMagic {
                found: EXPRESS_NATIVE_LIB_RELOC_MAGIC,
            })
        );
    }

    #[cfg(not(target_pointer_width = "32"))]
    #[test]
    fn fixed_target_loader_fails_closed_on_wide_hosts() {
        assert_eq!(
            unsafe { ExpressInterface::from_target(ExpressTarget::Esp32C3) },
            Err(ExpressLoadError::UnsupportedPointerWidth)
        );
    }
}
