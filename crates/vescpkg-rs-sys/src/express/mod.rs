//! Separate VESC Express native-library ABI foundation.
//!
//! Express is intentionally not projected into the STM32 `VescIf` table. This
//! module currently provides the pinned 32-bit table shape, version/magic
//! constants, scalar/function slot classification, and a fail-closed borrowed
//! loader. Typed callable wrappers can build on this boundary without mixing
//! slot order or target pointers with STM32.

mod table;
mod types;

pub use table::{ExpressSlotKind, ExpressTable, ExpressTableError, express_slot_kind};
pub use types::{
    EXPRESS_C_IF_VERSION, EXPRESS_IF_SLOT_COUNT, EXPRESS_IF_TABLE_BYTES, EXPRESS_NATIVE_LIB_MAGIC,
    EXPRESS_NATIVE_LIB_RELOC_MAGIC, EXPRESS_SYSTEM_TICK_RATE_HZ, ExpressAddress, ExpressWord,
};

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(table.word(2), Some(ExpressWord::new(0x1234)));
        assert!(!table.is_complete());
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
