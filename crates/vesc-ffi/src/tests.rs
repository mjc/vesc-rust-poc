#[allow(unused_imports)]
use crate::{
    AppDataLen, AppDataPacket, CanControllerId, CanFrameLen, CanPayload, CanStatusIndex, CfgFloat,
    CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult, ConfigXmlBytes, EepromAddress,
    EepromVar, FirmwareNonNull, FirmwarePtr, GpioPin, GpioPortPtr, HalfDuplex, HardwareType,
    ImageOffset, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt, LbmIoSymbol,
    LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo, LibInfoAbi, LoaderBaseAddress,
    MallocLen, MotorIndex, MutablePacket, MutexHandle, NativeAddress, NativeImage, NvmAddress,
    NvmBytes, NvmLen, OwnedFirmwareAllocation, PlotAxisName, PlotGraphIndex, PlotGraphName,
    PlotPoint, ProgramAddress, ReplyPacket, SemaphoreHandle, StackSizeBytes, SystemTicks,
    ThreadHandle, ThreadName, UartBaudRate, UartWriteLen, VescIfAbi, VescIfSlot, VescPin,
    VescPinMode,
};
use core::ffi::{CStr, c_char};

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
fn raw_vesc_if_offsets_match_the_documented_32_bit_package_header_slots() {
    let expected =
        VescIfAbi::USED_SLOTS.map(|slot| slot.host_byte_offset(core::mem::size_of::<usize>()));

    assert_eq!(crate::raw::vesc_if_offsets_for_tests(), expected);
}

#[test]
fn raw_vesc_if_table_covers_the_current_vesc_firmware_header() {
    let pointer_size = core::mem::size_of::<usize>();

    assert_eq!(
        crate::raw::vesc_if_full_layout_for_tests(),
        (253 * pointer_size, pointer_size, 252 * pointer_size)
    );
}

#[test]
fn raw_vesc_if_callable_slots_are_nullable_c_function_pointers() {
    let pointer_size = core::mem::size_of::<usize>();

    assert_eq!(
        crate::raw::nullable_slot_layout_for_tests(),
        (pointer_size, pointer_size)
    );
}

#[test]
fn vesc_if_slot_constants_name_the_package_header_offsets() {
    let slots = VescIfAbi::USED_SLOTS;

    assert_eq!(VescIfAbi::BASE_ADDR, NativeAddress(0x1000_f800));
    assert_eq!(
        slots.map(|slot| slot.name()),
        [
            "lbm_add_extension",
            "lbm_enc_i",
            "lbm_dec_as_i32",
            "lbm_is_number",
            "lbm_enc_sym_eerror",
            "send_app_data",
            "set_app_data_handler",
            "system_time_ticks",
            "io_set_mode",
            "io_write",
            "io_read",
        ]
    );
    assert_eq!(
        slots.map(|slot| slot.vesc32_byte_offset()),
        [0, 64, 100, 124, 148, 592, 596, 952, 220, 224, 228]
    );
    assert_eq!(
        slots.map(|slot| slot.slot_index()),
        [0, 16, 25, 31, 37, 148, 149, 238, 55, 56, 57]
    );
}

#[test]
fn vesc_if_slot_host_byte_offset_scales_with_pointer_width() {
    let pointer_size = core::mem::size_of::<usize>();
    let slot = VescIfSlot::new("custom", 64);

    assert_eq!(slot.name(), "custom");
    assert_eq!(slot.vesc32_byte_offset(), 64);
    assert_eq!(slot.slot_index(), 16);
    assert_eq!(slot.host_byte_offset(pointer_size), 16 * pointer_size);
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
fn arm_asm_immediates_match_vesc_if_abi() {
    let raw = include_str!("raw.rs");
    let expected = [
        (
            "lbm_add_extension",
            VescIfAbi::LBM_ADD_EXTENSION.vesc32_byte_offset(),
        ),
        ("lbm_enc_i", VescIfAbi::LBM_ENC_I.vesc32_byte_offset()),
        (
            "lbm_dec_as_i32",
            VescIfAbi::LBM_DEC_AS_I32.vesc32_byte_offset(),
        ),
        (
            "lbm_is_number",
            VescIfAbi::LBM_IS_NUMBER.vesc32_byte_offset(),
        ),
        (
            "lbm_enc_sym_eerror",
            VescIfAbi::LBM_ENC_SYM_EERROR.vesc32_byte_offset(),
        ),
        (
            "set_app_data_handler",
            VescIfAbi::SET_APP_DATA_HANDLER.vesc32_byte_offset(),
        ),
        (
            "send_app_data",
            VescIfAbi::SEND_APP_DATA.vesc32_byte_offset(),
        ),
        (
            "system_time_ticks",
            VescIfAbi::SYSTEM_TIME_TICKS.vesc32_byte_offset(),
        ),
    ];

    for (slot, offset) in expected {
        let insn = std::format!("ldr {{{slot}}}, [{{vesc_if}}, #{offset}]");
        assert!(
            raw.contains(&insn),
            "missing ARM asm dispatch for {slot} at byte offset {offset}"
        );
    }
}
