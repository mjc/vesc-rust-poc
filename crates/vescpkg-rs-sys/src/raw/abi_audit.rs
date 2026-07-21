use std::{mem::size_of, path::PathBuf, vec::Vec};

use clang::{Clang, EntityKind, Index, Type, TypeKind};

use super::{VescIf, VescIfAbi};

const SCALAR_FIELDS: [&str; 5] = [
    "lbm_enc_sym_nil",
    "lbm_enc_sym_true",
    "lbm_enc_sym_terror",
    "lbm_enc_sym_eerror",
    "lbm_enc_sym_merror",
];

#[test]
fn libclang_agrees_with_generated_vesc_if_inventory() {
    let header = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(VescIfAbi::SOURCE_HEADER);
    let clang = Clang::new().expect("load libclang; enter the Nix dev shell");
    let index = Index::new(&clang, false, false);
    let translation_unit = index
        .parser(&header)
        .arguments(&[
            "-DIS_VESC_LIB",
            "-target",
            "arm-none-eabi",
            "-ffreestanding",
        ])
        .parse()
        .expect("parse pinned vesc_c_if.h with libclang");
    let typedef = translation_unit
        .get_entity()
        .get_children()
        .into_iter()
        .find(|entity| {
            entity.get_kind() == EntityKind::TypedefDecl
                && entity.get_name().as_deref() == Some("vesc_c_if")
        })
        .expect("find vesc_c_if typedef");
    let record = typedef
        .get_typedef_underlying_type()
        .and_then(|ty| ty.get_declaration())
        .expect("resolve vesc_c_if struct");
    let fields: Vec<_> = record
        .get_children()
        .into_iter()
        .filter(|entity| entity.get_kind() == EntityKind::FieldDecl)
        .collect();
    let rust_offsets = crate::c_vesc_if::rust_field_offsets!(VescIf);

    assert_eq!(fields.len(), VescIfAbi::ALL_SLOTS.len());
    assert_eq!(rust_offsets.len(), fields.len());
    assert_eq!(
        core::mem::size_of::<VescIf>() / core::mem::size_of::<usize>(),
        fields.len(),
        "Rust VescIf must contain one pointer-sized word per C field"
    );

    for (index, (field, slot)) in fields.iter().zip(VescIfAbi::ALL_SLOTS).enumerate() {
        let name = field.get_name().expect("named vesc_c_if field");
        let ty = field.get_type().expect("typed vesc_c_if field");
        let byte_offset = field
            .get_offset_of_field()
            .expect("laid-out vesc_c_if field")
            / 8;

        assert_eq!(name, slot.name(), "slot {index} name drifted");
        assert_eq!(
            byte_offset,
            slot.vesc32_byte_offset(),
            "VESC32 offset drifted for {name}"
        );
        assert_eq!(
            (rust_offsets[index] / core::mem::size_of::<usize>()) * 4,
            byte_offset,
            "Rust VescIf offset drifted for {name}"
        );
        assert_eq!(ty.get_sizeof().expect("sized vesc_c_if field"), 4, "{name}");
        assert_eq!(
            is_function_pointer(ty),
            !SCALAR_FIELDS.contains(&name.as_str()),
            "declaration shape drifted for {name}"
        );
    }
}

fn is_function_pointer(ty: Type<'_>) -> bool {
    let canonical = ty.get_canonical_type();
    canonical.get_kind() == TypeKind::Pointer
        && canonical.get_pointee_type().is_some_and(|pointee| {
            pointee.get_canonical_type().get_kind() == TypeKind::FunctionPrototype
        })
}

#[test]
fn concrete_abi_type_sizes_match_the_pinned_header() {
    let header = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(VescIfAbi::SOURCE_HEADER);
    let clang = Clang::new().expect("load libclang; enter the Nix dev shell");
    let index = Index::new(&clang, false, false);
    let translation_unit = index
        .parser(&header)
        .arguments(&[
            "-DIS_VESC_LIB",
            "-target",
            "arm-none-eabi",
            "-ffreestanding",
        ])
        .parse()
        .expect("parse pinned vesc_c_if.h with libclang");

    let expected = [
        ("eeprom_var", size_of::<super::EepromVar>()),
        ("can_status_msg", size_of::<super::CanStatusMsg>()),
        ("can_status_msg_2", size_of::<super::CanStatusMsg2>()),
        ("can_status_msg_3", size_of::<super::CanStatusMsg3>()),
        ("can_status_msg_4", size_of::<super::CanStatusMsg4>()),
        ("can_status_msg_5", size_of::<super::CanStatusMsg5>()),
        ("can_status_msg_6", size_of::<super::CanStatusMsg6>()),
        ("gnss_data", size_of::<super::GnssData>()),
        ("ATTITUDE_INFO", size_of::<super::AttitudeInfo>()),
        ("remote_state", size_of::<super::RemoteState>()),
    ];

    for (name, rust_size) in expected {
        let typedef = translation_unit
            .get_entity()
            .get_children()
            .into_iter()
            .find(|entity| {
                entity.get_kind() == EntityKind::TypedefDecl
                    && entity.get_name().as_deref() == Some(name)
            })
            .unwrap_or_else(|| panic!("find {name} typedef"));
        let c_size = typedef
            .get_typedef_underlying_type()
            .and_then(|ty| ty.get_sizeof().ok())
            .unwrap_or_else(|| panic!("size {name} typedef"));
        assert_eq!(c_size, rust_size, "layout size drifted for {name}");
    }

    #[cfg(target_arch = "arm")]
    {
        assert_eq!(size_of::<super::LbmFlatValue>(), 12);
        assert_eq!(size_of::<super::LbmArrayHeader>(), 12);
        assert_eq!(size_of::<super::PacketState>(), 1060);
    }
}
