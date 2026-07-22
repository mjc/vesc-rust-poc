//! Separate VESC Express native-library ABI foundation.
//!
//! Express is intentionally not projected into the STM32 `VescIf` table. This
//! module currently provides the pinned 32-bit table shape, version/magic
//! constants, scalar/function slot classification, and a fail-closed borrowed
//! loader. Typed callable wrappers can build on this boundary without mixing
//! slot order or target pointers with STM32.

mod functions;
mod loader;
mod memory;
mod runtime;
mod sync;
mod table;
mod types;

pub use functions::*;
pub use loader::{ExpressCallError, ExpressInterface, ExpressLoadError};
pub use memory::{ExpressAllocation, ExpressAllocationError};
pub use runtime::ExpressRuntime;
pub use sync::{ExpressMutex, ExpressMutexGuard, ExpressSemaphore, ExpressSyncError};
pub use table::{ExpressSlot, ExpressSlotKind, ExpressTable, ExpressTableError, express_slot_kind};
pub use types::{
    EXPRESS_C_IF_VERSION, EXPRESS_IF_SLOT_COUNT, EXPRESS_IF_TABLE_BYTES, EXPRESS_NATIVE_LIB_MAGIC,
    EXPRESS_NATIVE_LIB_RELOC_MAGIC, EXPRESS_SYSTEM_TICK_RATE_HZ, ExpressAddress, ExpressTarget,
    ExpressWord,
};

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn express_noop(_: *mut core::ffi::c_void) {}

    #[test]
    fn pinned_v1_shape_is_independent_from_stm32() {
        assert_eq!(EXPRESS_C_IF_VERSION, 1);
        assert_eq!(EXPRESS_IF_SLOT_COUNT, 80);
        assert_eq!(EXPRESS_IF_TABLE_BYTES, 320);
        assert_eq!(express_slot_kind(0), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(38), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(42), Some(ExpressSlotKind::Scalar));
        assert_eq!(express_slot_kind(43), Some(ExpressSlotKind::Function));
        assert_eq!(express_slot_kind(80), None);
        assert_eq!(ExpressSlot::IfVersion.index(), 0);
        assert_eq!(ExpressSlot::LbmAddExtension.index(), 1);
        assert_eq!(ExpressSlot::LbmEncSymNil.index(), 38);
        assert_eq!(ExpressSlot::SemReset.index(), 79);
        assert_eq!(ExpressTarget::Esp32C3.interface_address(), 0x3FCD_BE00);
        assert_eq!(ExpressTarget::Esp32S3.interface_address(), 0x3FCE_8800);
        assert_eq!(ExpressTarget::Esp32C6.interface_address(), 0x4087_B800);
        assert_eq!(ExpressTarget::Esp32P4.interface_address(), 0x4FF3_A000);
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
}
