use std::{
    mem::size_of,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    vec,
    vec::Vec,
};

use clang::{Clang, EntityKind, Index, Type, TypeKind};

use super::{VescIf, VescIfAbi};

const SCALAR_FIELDS: [&str; 5] = [
    "lbm_enc_sym_nil",
    "lbm_enc_sym_true",
    "lbm_enc_sym_terror",
    "lbm_enc_sym_eerror",
    "lbm_enc_sym_merror",
];

static LIBCLANG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn libclang_agrees_with_generated_vesc_if_inventory() {
    let _guard = LIBCLANG_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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
    let record_type = record.get_type().expect("typed vesc_c_if struct");
    let fields: Vec<_> = record
        .get_children()
        .into_iter()
        .filter(|entity| entity.get_kind() == EntityKind::FieldDecl)
        .collect();
    assert_eq!(fields.len(), VescIfAbi::ALL_SLOTS.len());
    let pointer_size = core::mem::size_of::<usize>();
    let expected_host_size = VescIfAbi::ALL_ENTRIES.iter().fold(0, |offset, entry| {
        let field_size = if entry.is_callable() { pointer_size } else { 4 };
        let aligned = (offset + field_size - 1) & !(field_size - 1);
        aligned + field_size
    });
    assert_eq!(core::mem::size_of::<VescIf>(), expected_host_size);

    for (index, (field, slot)) in fields.iter().zip(VescIfAbi::ALL_SLOTS).enumerate() {
        let name = field.get_name().expect("named vesc_c_if field");
        let ty = field.get_type().expect("typed vesc_c_if field");
        let byte_offset = record_type
            .get_offsetof(&name)
            .expect("laid-out vesc_c_if field")
            / 8;

        assert_eq!(name, slot.name(), "slot {index} name drifted");
        assert_eq!(
            byte_offset,
            slot.vesc32_byte_offset(),
            "VESC32 offset drifted for {name}"
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
    let _guard = LIBCLANG_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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

#[test]
fn concrete_abi_field_offsets_match_the_pinned_header() {
    let _guard = LIBCLANG_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
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

    let record_offsets = |name: &str| {
        let typedef = translation_unit
            .get_entity()
            .get_children()
            .into_iter()
            .find(|entity| {
                entity.get_kind() == EntityKind::TypedefDecl
                    && entity.get_name().as_deref() == Some(name)
            })
            .unwrap_or_else(|| panic!("find {name} typedef"));
        let record = typedef
            .get_typedef_underlying_type()
            .and_then(|ty| ty.get_declaration())
            .unwrap_or_else(|| panic!("resolve {name} record"));
        let record_type = record
            .get_type()
            .unwrap_or_else(|| panic!("type {name} record"));
        record
            .get_children()
            .into_iter()
            .filter(|field| field.get_kind() == EntityKind::FieldDecl)
            .map(|field| {
                let field_name = field.get_name().expect("named field");
                record_type
                    .get_offsetof(&field_name)
                    .unwrap_or_else(|_| panic!("offset in {name}"))
                    / 8
            })
            .collect::<Vec<_>>()
    };

    let expected = [
        (
            "can_status_msg",
            vec![
                core::mem::offset_of!(super::CanStatusMsg, id),
                core::mem::offset_of!(super::CanStatusMsg, rx_time),
                core::mem::offset_of!(super::CanStatusMsg, rpm),
                core::mem::offset_of!(super::CanStatusMsg, current),
                core::mem::offset_of!(super::CanStatusMsg, duty),
            ],
        ),
        (
            "can_status_msg_2",
            vec![
                core::mem::offset_of!(super::CanStatusMsg2, id),
                core::mem::offset_of!(super::CanStatusMsg2, rx_time),
                core::mem::offset_of!(super::CanStatusMsg2, amp_hours),
                core::mem::offset_of!(super::CanStatusMsg2, amp_hours_charged),
            ],
        ),
        (
            "can_status_msg_3",
            vec![
                core::mem::offset_of!(super::CanStatusMsg3, id),
                core::mem::offset_of!(super::CanStatusMsg3, rx_time),
                core::mem::offset_of!(super::CanStatusMsg3, watt_hours),
                core::mem::offset_of!(super::CanStatusMsg3, watt_hours_charged),
            ],
        ),
        (
            "can_status_msg_4",
            vec![
                core::mem::offset_of!(super::CanStatusMsg4, id),
                core::mem::offset_of!(super::CanStatusMsg4, rx_time),
                core::mem::offset_of!(super::CanStatusMsg4, temp_fet),
                core::mem::offset_of!(super::CanStatusMsg4, temp_motor),
                core::mem::offset_of!(super::CanStatusMsg4, current_in),
                core::mem::offset_of!(super::CanStatusMsg4, pid_pos_now),
            ],
        ),
        (
            "can_status_msg_5",
            vec![
                core::mem::offset_of!(super::CanStatusMsg5, id),
                core::mem::offset_of!(super::CanStatusMsg5, rx_time),
                core::mem::offset_of!(super::CanStatusMsg5, v_in),
                core::mem::offset_of!(super::CanStatusMsg5, tacho_value),
            ],
        ),
        (
            "can_status_msg_6",
            vec![
                core::mem::offset_of!(super::CanStatusMsg6, id),
                core::mem::offset_of!(super::CanStatusMsg6, rx_time),
                core::mem::offset_of!(super::CanStatusMsg6, adc_1),
                core::mem::offset_of!(super::CanStatusMsg6, adc_2),
                core::mem::offset_of!(super::CanStatusMsg6, adc_3),
                core::mem::offset_of!(super::CanStatusMsg6, ppm),
            ],
        ),
        (
            "gnss_data",
            vec![
                core::mem::offset_of!(super::GnssData, lat),
                core::mem::offset_of!(super::GnssData, lon),
                core::mem::offset_of!(super::GnssData, height),
                core::mem::offset_of!(super::GnssData, speed),
                core::mem::offset_of!(super::GnssData, hdop),
                core::mem::offset_of!(super::GnssData, ms_today),
                core::mem::offset_of!(super::GnssData, yy),
                core::mem::offset_of!(super::GnssData, mo),
                core::mem::offset_of!(super::GnssData, dd),
                core::mem::offset_of!(super::GnssData, last_update),
            ],
        ),
        (
            "ATTITUDE_INFO",
            vec![
                core::mem::offset_of!(super::AttitudeInfo, q0),
                core::mem::offset_of!(super::AttitudeInfo, q1),
                core::mem::offset_of!(super::AttitudeInfo, q2),
                core::mem::offset_of!(super::AttitudeInfo, q3),
                core::mem::offset_of!(super::AttitudeInfo, integralFBx),
                core::mem::offset_of!(super::AttitudeInfo, integralFBy),
                core::mem::offset_of!(super::AttitudeInfo, integralFBz),
                core::mem::offset_of!(super::AttitudeInfo, accMagP),
                core::mem::offset_of!(super::AttitudeInfo, initialUpdateDone),
                core::mem::offset_of!(super::AttitudeInfo, acc_confidence_decay),
                core::mem::offset_of!(super::AttitudeInfo, kp),
                core::mem::offset_of!(super::AttitudeInfo, ki),
                core::mem::offset_of!(super::AttitudeInfo, beta),
            ],
        ),
        (
            "remote_state",
            vec![
                core::mem::offset_of!(super::RemoteState, js_x),
                core::mem::offset_of!(super::RemoteState, js_y),
                core::mem::offset_of!(super::RemoteState, bt_c),
                core::mem::offset_of!(super::RemoteState, bt_z),
                core::mem::offset_of!(super::RemoteState, is_rev),
                core::mem::offset_of!(super::RemoteState, age_s),
            ],
        ),
        (
            "lbm_flat_value_t",
            vec![
                core::mem::offset_of!(super::LbmFlatValue, buf),
                core::mem::offset_of!(super::LbmFlatValue, buf_size),
                core::mem::offset_of!(super::LbmFlatValue, buf_pos),
            ],
        ),
        (
            "lbm_array_header_t",
            vec![
                core::mem::offset_of!(super::LbmArrayHeader, size),
                core::mem::offset_of!(super::LbmArrayHeader, data),
            ],
        ),
        (
            "PACKET_STATE_t",
            vec![
                core::mem::offset_of!(super::PacketState, send_func),
                core::mem::offset_of!(super::PacketState, process_func),
                core::mem::offset_of!(super::PacketState, rx_read_ptr),
                core::mem::offset_of!(super::PacketState, rx_write_ptr),
                core::mem::offset_of!(super::PacketState, bytes_left),
                core::mem::offset_of!(super::PacketState, rx_buffer),
                core::mem::offset_of!(super::PacketState, tx_buffer),
            ],
        ),
    ];

    for (name, rust_offsets) in expected {
        if !cfg!(target_arch = "arm")
            && matches!(
                name,
                "lbm_flat_value_t" | "lbm_array_header_t" | "PACKET_STATE_t"
            )
        {
            // The pinned C oracle is parsed for the STM32 target. Host
            // pointers are wider; those pointer-bearing records are checked
            // directly by the ARM build instead.
            continue;
        }
        assert_eq!(
            record_offsets(name),
            rust_offsets,
            "field offsets drifted for {name}"
        );
    }
}
