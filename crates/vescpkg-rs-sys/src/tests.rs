#[allow(unused_imports)]
use crate::{
    AbiError, AppDataLen, AppDataPacket, CanBaud, CanControllerId, CanFrameLen, CanPayload,
    CanStatusIndex, CfgFloat, CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult,
    ConfigXmlBytes, EepromAddress, EepromVar, FirmwareNonNull, FirmwarePtr, GpioPin, GpioPortPtr,
    HalfDuplex, HardwareType, ImageOffset, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol,
    LbmFloat, LbmInt, LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo,
    LibInfoAbi, LoaderBaseAddress, MallocLen, MotorIndex, MutablePacket, MutexHandle,
    NativeAddress, NativeImage, NvmAddress, NvmBytes, NvmLen, OwnedFirmwareAllocation,
    PlotAxisName, PlotGraphIndex, PlotGraphName, PlotPoint, ProgramAddress, ReplyPacket,
    SemaphoreHandle, StackSizeBytes, Stm32AbiRevision, SystemTicks, ThreadHandle, ThreadName,
    UartBaudRate, UartWriteLen, VescIfAbi, VescIfCapabilities, VescIfSlot, VescIfSlotKind,
    VescIfSubsystem, VescPin, VescPinMode,
};
use core::ffi::{CStr, c_char};
use std::vec;

#[test]
fn native_image_rebases_image_data_offsets() {
    let image = NativeImage::new(0x2000);

    assert_eq!(image.rebase_addr(0x61), 0x2061);
    assert_eq!(image.base_addr(), NativeAddress(0x2000));
    assert_eq!(
        image.rebase_offset(ImageOffset::new(0x61)),
        NativeAddress(0x2061)
    );
    assert_eq!(image.rebase_ptr(0x1df as *const c_char) as usize, 0x21df);
}

#[test]
fn native_image_resolves_relative_and_loaded_code_addresses() {
    let image = NativeImage::new(0x2000);

    assert_eq!(image.resolve_addr(0x61), 0x2061);
    assert_eq!(image.resolve_addr(0x2061), 0x2061);
}

#[test]
fn native_image_from_info_uses_loader_base_addr() {
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x3000,
    };

    assert_eq!(
        NativeImage::from_info(&info).base_addr(),
        NativeAddress(0x3000)
    );
}

#[test]
fn lib_info_abi_constants_match_the_vesc_native_loader_layout() {
    assert_eq!(LibInfoAbi::STOP_FUN_OFFSET, 0);
    assert_eq!(LibInfoAbi::ARG_OFFSET, 4);
    assert_eq!(LibInfoAbi::BASE_ADDR_OFFSET, 8);
    assert_eq!(LibInfoAbi::SIZE, 12);
    assert_eq!(LibInfoAbi::ALIGN, 4);
}

#[test]
fn lib_info_abi_exposes_a_target_layout_assertion() {
    assert_eq!(LibInfoAbi::SIZE, 12);
    assert_eq!(LibInfoAbi::ALIGN, 4);
}

#[test]
fn lib_info_repr_c_layout_scales_with_the_compilation_pointer_width() {
    let pointer_size = core::mem::size_of::<usize>();

    assert_eq!(core::mem::size_of::<LibInfo>(), pointer_size * 3);
    assert_eq!(core::mem::align_of::<LibInfo>(), pointer_size);
    assert_eq!(core::mem::offset_of!(LibInfo, stop_fun), 0);
    assert_eq!(core::mem::offset_of!(LibInfo, arg), pointer_size);
    assert_eq!(core::mem::offset_of!(LibInfo, base_addr), pointer_size * 2);
}

#[test]
fn raw_vesc_if_table_covers_the_current_vesc_firmware_header() {
    let pointer_size = core::mem::size_of::<usize>();
    let table_size = core::mem::size_of::<crate::raw::VescIf>();

    assert_eq!(core::mem::align_of::<crate::raw::VescIf>(), pointer_size);
    assert_eq!(
        core::mem::offset_of!(crate::raw::VescIf, lbm_enc_sym_true),
        core::mem::offset_of!(crate::raw::VescIf, lbm_enc_sym_nil) + 4
    );
    assert_eq!(
        core::mem::offset_of!(crate::raw::VescIf, lbm_is_symbol_nil),
        core::mem::offset_of!(crate::raw::VescIf, lbm_enc_sym_merror) + 8
    );
    assert_eq!(
        table_size,
        core::mem::offset_of!(crate::raw::VescIf, shutdown_disable) + pointer_size
    );
}

#[test]
fn raw_vesc_if_mock_function_slots_have_pointer_layout() {
    let pointer_size = core::mem::size_of::<usize>();
    assert_eq!(
        core::mem::size_of::<Option<unsafe extern "C" fn()>>(),
        pointer_size
    );
    assert_eq!(
        core::mem::align_of::<Option<unsafe extern "C" fn()>>(),
        pointer_size
    );
}

#[test]
fn concrete_abi_types_match_the_pinned_stm32_word_layout() {
    let pointer_size = core::mem::size_of::<usize>();
    assert_eq!(core::mem::size_of::<crate::raw::EepromVar>(), 4);
    assert_eq!(
        core::mem::size_of::<crate::raw::LbmFlatValue>(),
        pointer_size + 8
    );
    assert_eq!(
        core::mem::size_of::<crate::raw::LbmArrayHeader>(),
        pointer_size + 8
    );
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg>(), 20);
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg2>(), 16);
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg3>(), 16);
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg4>(), 24);
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg5>(), 16);
    assert_eq!(core::mem::size_of::<crate::raw::CanStatusMsg6>(), 24);
    assert_eq!(core::mem::size_of::<crate::raw::GnssData>(), 40);
    assert_eq!(core::mem::size_of::<crate::raw::AttitudeInfo>(), 52);
    assert_eq!(core::mem::size_of::<crate::raw::RemoteState>(), 16);
    let packet_size = pointer_size * 2 + 12 + (512 + 8) * 2;
    assert_eq!(
        core::mem::size_of::<crate::raw::PacketState>(),
        packet_size.div_ceil(pointer_size) * pointer_size
    );
    assert_eq!(core::mem::offset_of!(crate::raw::GnssData, last_update), 36);
    assert_eq!(
        core::mem::offset_of!(crate::raw::AttitudeInfo, initialUpdateDone),
        32
    );
}

#[test]
fn eeprom_values_preserve_the_generated_union_bits() {
    let unsigned = crate::raw::EepromVar {
        as_u32: 0xfeed_beef,
    };
    assert_eq!(unsafe { unsigned.as_u32 }, 0xfeed_beef);

    let signed = crate::raw::EepromVar { as_i32: -42 };
    assert_eq!(unsafe { signed.as_i32 }, -42);

    let floating = crate::raw::EepromVar { as_float: 12.5 };
    assert_eq!(unsafe { floating.as_float }, 12.5);
}

#[test]
fn vesc_if_slot_constants_name_the_package_header_offsets() {
    assert_eq!(VescIfAbi::BASE_ADDR, NativeAddress(0x1000_f800));
    assert_eq!(VescIfAbi::USED_SLOT_COUNT, VescIfAbi::USED_SLOTS.len());

    assert!(VescIfAbi::USED_SLOTS.contains(&VescIfAbi::SLEEP_US));
    assert!(VescIfAbi::USED_SLOTS.contains(&VescIfAbi::FOC_GET_ID));
    assert!(VescIfAbi::USED_SLOTS.contains(&VescIfAbi::THREAD_SET_PRIORITY));
    assert_eq!(VescIfAbi::CAN_GET_STATUS_MSG_INDEX.slot_index(), 75);
    assert_eq!(VescIfAbi::SHUTDOWN_DISABLE.slot_index(), 252);
}
#[test]
fn vesc_if_slot_reports_firmware_offsets() {
    let slot = VescIfSlot::new("custom", 64);

    assert_eq!(slot.name(), "custom");
    assert_eq!(slot.vesc32_byte_offset(), 64);
    assert_eq!(slot.slot_index(), 16);
}

#[test]
fn newtypes_wrap_the_expected_scalar_shapes() {
    assert_eq!(core::mem::size_of::<LbmInt>(), core::mem::size_of::<i32>());
    assert_eq!(core::mem::size_of::<LbmUint>(), core::mem::size_of::<u32>());
    assert_eq!(core::mem::size_of::<LbmType>(), core::mem::size_of::<u32>());
    assert_eq!(core::mem::size_of::<LbmCid>(), core::mem::size_of::<u32>());
    assert_eq!(
        core::mem::size_of::<LbmFloat>(),
        core::mem::size_of::<f32>()
    );
    assert_eq!(
        core::mem::size_of::<LbmSymbol>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<ProgramAddress>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<AppDataPacket<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<MutablePacket<'_>>(),
        core::mem::size_of::<&mut [u8]>()
    );
    assert_eq!(
        core::mem::size_of::<CommandPacket<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<ReplyPacket<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<HalfDuplex>(),
        core::mem::size_of::<bool>()
    );
    assert_eq!(
        core::mem::size_of::<SystemTicks>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<ThreadName<'_>>(),
        core::mem::size_of::<&CStr>()
    );
    assert_eq!(
        core::mem::size_of::<FirmwarePtr::<u8>>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<CanPayload<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<PlotPoint>(),
        core::mem::size_of::<f32>() * 2
    );
    assert_eq!(core::mem::size_of::<VescPin>(), core::mem::size_of::<i32>());
    assert_eq!(
        core::mem::size_of::<NvmBytes<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
}

#[test]
fn transparent_wrappers_expose_raw_tuple_fields() {
    let raw = [1_u8, 2, 3];
    let mut mut_raw = [4_u8, 5, 6];
    let name = c"axis";

    assert_eq!(LbmInt(-7).0, -7);
    assert_eq!(LbmFloat(3.5).0, 3.5);
    assert!(HalfDuplex(true).0);
    assert_eq!(ConfigXmlBytes(&raw).0, &raw);
    assert_eq!(ConfigPayload(&raw).0, &raw);
    assert_eq!(ThreadName(name).0, name);
    assert_eq!(CanPayload(&raw).0, &raw);
    assert_eq!(PlotAxisName(name).0, name);
    assert_eq!(PlotGraphName(name).0, name);
    assert_eq!(NvmBytes(&raw).0, &raw);
    {
        let packet = MutablePacket(&mut mut_raw);
        packet.0[0] = 9;
    }
    assert_eq!(mut_raw[0], 9);
    let point = PlotPoint { x: 1.5, y: 2.5 };
    assert_eq!(point.x, 1.5);
    assert_eq!(point.y, 2.5);
}

#[test]
fn can_baud_representation_preserves_pinned_enum_values() {
    assert_eq!(core::mem::size_of::<CanBaud>(), core::mem::size_of::<i32>());
    assert_eq!(CanBaud::K125.raw(), 0);
    assert_eq!(CanBaud::K500.raw(), 2);
    assert_eq!(CanBaud::K100.raw(), 8);
}

#[test]
fn vesc_if_manifest_matches_generated_header_descriptors() {
    assert_eq!(crate::c_vesc_if::FIELD_COUNT, VescIfAbi::FIELD_COUNT);
    assert_eq!(VescIfAbi::FIELD_COUNT, 253);
    assert_eq!(VescIfAbi::CALLABLE_SLOT_COUNT, 248);
    assert_eq!(VescIfAbi::ALL_SLOTS.len(), VescIfAbi::FIELD_COUNT);
    assert_eq!(VescIfAbi::ALL_ENTRIES.len(), VescIfAbi::FIELD_COUNT);
    assert_eq!(
        VescIfAbi::SOURCE_REPOSITORY,
        "https://github.com/lukash/vesc_pkg_lib"
    );
    assert_eq!(
        VescIfAbi::SOURCE_COMMIT,
        "e8bdc8296b90a266713da3762868f0d18ec027fe"
    );
    assert_eq!(
        VescIfAbi::SOURCE_HEADER,
        "third_party/vesc_pkg_lib/vesc_c_if.h"
    );
    assert_eq!(VescIfAbi::ALL_SLOTS[0].name(), "lbm_add_extension");
    assert_eq!(VescIfAbi::ALL_SLOTS[252].name(), "shutdown_disable");

    for (index, (slot, entry)) in VescIfAbi::ALL_SLOTS
        .iter()
        .zip(VescIfAbi::ALL_ENTRIES.iter())
        .enumerate()
    {
        assert_eq!(slot.slot_index(), index);
        assert_eq!(entry.slot(), *slot);
    }
    assert_eq!(VescIfAbi::ALL_ENTRIES[0].kind(), VescIfSlotKind::Function);
    let scalar = VescIfAbi::ALL_ENTRIES
        .iter()
        .find(|entry| entry.slot().name() == "lbm_enc_sym_nil")
        .expect("bindgen scalar slot");
    assert_eq!(scalar.kind(), VescIfSlotKind::Scalar);
    assert_eq!(
        VescIfAbi::ALL_ENTRIES
            .iter()
            .filter(|entry| entry.is_callable())
            .count(),
        VescIfAbi::CALLABLE_SLOT_COUNT
    );
}

#[test]
fn vesc_if_presence_tracks_holes_and_profiles_from_observed_words() {
    let mut words = [1_usize; VescIfAbi::FIELD_COUNT];
    words[VescIfAbi::SYSTEM_TIME_TICKS.slot_index()] = 0;
    words[VescIfAbi::THREAD_SET_PRIORITY.slot_index()] = 0;

    let presence = crate::VescIfPresence::from_words(&words);
    assert!(presence.contains(VescIfAbi::LBM_ADD_EXTENSION));
    assert!(!presence.contains(VescIfAbi::SYSTEM_TIME_TICKS));
    assert!(!presence.supports_revision(Stm32AbiRevision::Firmware605));
    assert_eq!(presence.revision(), Stm32AbiRevision::Base);

    let base_words = [1_usize; VescIfAbi::BASE_SLOT_COUNT];
    let base_presence = crate::VescIfPresence::from_words(&base_words);
    assert!(base_presence.supports_revision(Stm32AbiRevision::Base));
    assert_eq!(base_presence.revision(), Stm32AbiRevision::Base);

    assert_eq!(
        base_presence.require("thread priority", VescIfAbi::THREAD_SET_PRIORITY),
        Err(AbiError::MissingRequired {
            capability: "thread priority",
            slot: VescIfAbi::THREAD_SET_PRIORITY,
        })
    );
    assert_eq!(
        base_presence.optional("thread priority", VescIfAbi::THREAD_SET_PRIORITY),
        Err(AbiError::Unsupported {
            capability: "thread priority",
            slot: VescIfAbi::THREAD_SET_PRIORITY,
        })
    );
}

#[test]
fn vesc_if_capabilities_expose_named_subsystems_without_raw_slot_names() {
    let mut words = vec![0; VescIfAbi::FIELD_COUNT];
    words[VescIfAbi::CAN_TRANSMIT_SID.slot_index()] = 1;
    words[VescIfAbi::READ_NVM.slot_index()] = 1;
    let capabilities = VescIfCapabilities::new(crate::VescIfPresence::from_words(&words));

    assert_eq!(capabilities.can().unwrap().subsystem(), VescIfSubsystem::Can);
    assert_eq!(capabilities.nvm().unwrap().subsystem(), VescIfSubsystem::Nvm);
    assert_eq!(capabilities.audio().unwrap_err().capability(), "FOC audio");
}
