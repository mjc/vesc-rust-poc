use std::{path::PathBuf, vec::Vec};

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
