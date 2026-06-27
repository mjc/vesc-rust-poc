#[allow(unused_imports)]
use crate::{
    AppDataBindings, AppDataLen, AppDataPacket, CanControllerId, CanFrameLen, CanPayload,
    CanStatusIndex, CfgFloat, CfgInt, CfgParam, CommandPacket, ConfigPayload, ConfigSetResult,
    ConfigXmlBytes, EepromAddress, EepromVar, ExtensionDescriptor, ExtensionHandler,
    FirmwareNonNull, FirmwarePtr, GpioPin, GpioPortPtr, HalfDuplex, HardwareType, ImageOffset,
    LbmApi, LbmBindings, LbmBoolSymbol, LbmCid, LbmCount, LbmErrorSymbol, LbmFloat, LbmInt,
    LbmIoSymbol, LbmNilSymbol, LbmSymbol, LbmType, LbmUint, LbmValue, LibInfo, LibInfoAbi,
    LoaderBaseAddress, LoopbackLifecycle, MallocLen, MotorIndex, MutablePacket, MutexHandle,
    NativeAddress, NativeImage, NvmAddress, NvmBytes, NvmLen, OwnedFirmwareAllocation,
    PackageLifecycle, PlotAxisName, PlotGraphIndex, PlotGraphName, PlotPoint, ProgramAddress,
    RegisterError, ReplyPacket, SemaphoreHandle, StackSizeBytes, SystemTicks, ThreadHandle,
    ThreadName, UartBaudRate, UartWriteLen, VescIfAbi, VescIfSlot, VescPin, VescPinMode,
};
use core::ffi::{CStr, c_char};

use crate::test_support::{FakeAppDataBindings, FakeBindings};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

#[test]
fn register_extension_from_image_rebases_handler_before_firmware_call() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let handler_offset = 0x31_usize;
    let descriptor = ExtensionDescriptor::new(c"ext-test", unsafe {
        core::mem::transmute::<usize, ExtensionHandler>(handler_offset)
    });
    let image = NativeImage::new(0x2000);

    assert_eq!(
        lifecycle.register_extension_from_image(image, descriptor),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().last_handler.get(), 0x2031);
}

#[test]
fn loopback_lifecycle_forwards_firmware_app_data_calls_through_bindings() {
    let bindings = FakeAppDataBindings::with_ticks(1234);
    let lifecycle = LoopbackLifecycle::new(bindings);
    let payload = [1_u8, 2, 3];

    assert_eq!(lifecycle.system_time_ticks(), 1234);
    unsafe { lifecycle.send_app_data(payload.as_ptr(), 3) };

    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_len.get(), 3);
    assert_eq!(
        lifecycle.bindings().last_data.get(),
        payload.as_ptr() as usize
    );
}

#[test]
fn wrapper_delegates_through_the_binding_trait() {
    let bindings = FakeBindings::new();
    let api = LbmApi::new(bindings);
    let name = c"ext-rust-add";

    assert!(api.register_extension(name, stub_handler));
    assert_eq!(api.decode_i32(LbmValue(3)), 3);
    assert_eq!(api.encode_i32(9), LbmValue(9));
    assert!(api.is_number(LbmValue(9)));
    assert_eq!(api.encode_eval_error(), LbmValue(0xffff_ffff));
}

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
fn package_registration_reports_name_validation_and_firmware_rejection() {
    let bindings = FakeBindings::with_add_results([false, true]);
    let lifecycle = PackageLifecycle::new(bindings);

    let invalid = ExtensionDescriptor::new(c"bad-name", stub_handler);
    assert_eq!(
        lifecycle.register_extension(invalid),
        Err(RegisterError::InvalidExtensionName)
    );

    let rejected = ExtensionDescriptor::new(c"ext-rust-reject", stub_handler);
    assert_eq!(
        lifecycle.register_extension(rejected),
        Err(RegisterError::FirmwareRejected)
    );
}

#[test]
fn repeated_package_registration_reports_each_firmware_result() {
    let bindings = FakeBindings::with_add_results([false, true]);
    let lifecycle = PackageLifecycle::new(bindings);

    let first = ExtensionDescriptor::new(c"ext-rust-a", stub_handler);
    let second = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);
    assert_eq!(
        lifecycle.register_extension(first),
        Err(RegisterError::FirmwareRejected)
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    assert_eq!(lifecycle.register_extension(second), Ok(()));
    assert_eq!(lifecycle.bindings().add_calls.get(), 2);
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
        ]
    );
    assert_eq!(
        slots.map(|slot| slot.vesc32_byte_offset()),
        [0, 64, 100, 124, 148, 592, 596, 952]
    );
    assert_eq!(
        slots.map(|slot| slot.slot_index()),
        [0, 16, 25, 31, 37, 148, 149, 238]
    );
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
        core::mem::size_of::<LbmErrorSymbol>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<LbmBoolSymbol>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<LbmNilSymbol>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<ProgramAddress>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<LoaderBaseAddress>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<AppDataLen>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<UartBaudRate>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<UartWriteLen>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<MotorIndex>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<CanControllerId>(),
        core::mem::size_of::<u8>()
    );
    assert_eq!(
        core::mem::size_of::<CanFrameLen>(),
        core::mem::size_of::<u8>()
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
        core::mem::size_of::<CfgParam>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<CfgFloat>(),
        core::mem::size_of::<f32>()
    );
    assert_eq!(core::mem::size_of::<CfgInt>(), core::mem::size_of::<i32>());
    assert_eq!(
        core::mem::size_of::<ConfigSetResult>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<ConfigXmlBytes<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<ConfigPayload<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<ThreadName<'_>>(),
        core::mem::size_of::<&CStr>()
    );
    assert_eq!(
        core::mem::size_of::<StackSizeBytes>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<ThreadHandle>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<MutexHandle>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<SemaphoreHandle>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<FirmwarePtr::<u8>>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<FirmwareNonNull::<u8>>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<MallocLen>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<OwnedFirmwareAllocation::<u8>>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(
        core::mem::size_of::<CanPayload<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<CanStatusIndex>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<HardwareType>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<PlotAxisName<'_>>(),
        core::mem::size_of::<&CStr>()
    );
    assert_eq!(
        core::mem::size_of::<PlotGraphName<'_>>(),
        core::mem::size_of::<&CStr>()
    );
    assert_eq!(
        core::mem::size_of::<PlotGraphIndex>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<PlotPoint>(),
        core::mem::size_of::<f32>() * 2
    );
    assert_eq!(core::mem::size_of::<VescPin>(), core::mem::size_of::<i32>());
    assert_eq!(
        core::mem::size_of::<VescPinMode>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<GpioPortPtr>(),
        core::mem::size_of::<usize>()
    );
    assert_eq!(core::mem::size_of::<GpioPin>(), core::mem::size_of::<u32>());
    assert_eq!(
        core::mem::size_of::<LbmIoSymbol>(),
        core::mem::size_of::<LbmSymbol>()
    );
    assert_eq!(
        core::mem::size_of::<NvmAddress>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(core::mem::size_of::<NvmLen>(), core::mem::size_of::<u32>());
    assert_eq!(
        core::mem::size_of::<NvmBytes<'_>>(),
        core::mem::size_of::<&[u8]>()
    );
    assert_eq!(
        core::mem::size_of::<EepromAddress>(),
        core::mem::size_of::<i32>()
    );
    assert_eq!(
        core::mem::size_of::<EepromVar>(),
        core::mem::size_of::<i32>()
    );
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
fn extension_descriptor_validate_accepts_ext_prefix() {
    let descriptor = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);

    assert!(descriptor.validate().is_ok());
}

#[test]
fn register_extensions_from_image_registers_each_descriptor() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let image = NativeImage::new(0x2000);
    let first = ExtensionDescriptor::new(c"ext-rust-a", stub_handler);
    let second = ExtensionDescriptor::new(c"ext-rust-b", stub_handler);

    assert_eq!(
        lifecycle.register_extensions_from_image(image, [first, second]),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 2);
}

#[test]
fn register_extension_reports_success_when_firmware_accepts() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(c"ext-rust-ok", stub_handler);

    assert_eq!(lifecycle.register_extension(descriptor), Ok(()));
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
}

#[test]
fn loopback_lifecycle_install_sets_stop_hook() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);
    let mut info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    unsafe extern "C" fn stop(_arg: *mut core::ffi::c_void) {}
    unsafe extern "C" fn app_data(_data: *mut u8, _len: u32) {}

    assert!(unsafe { lifecycle.install(&mut info, stop, app_data) });
    assert!(info.stop_fun.is_some());
}

#[test]
fn loopback_lifecycle_registers_and_clears_app_data_handler() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

    assert!(lifecycle.register_app_data_handler(handler));
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
    assert_eq!(
        lifecycle.bindings().last_handler.get(),
        handler as *const () as usize
    );

    assert!(lifecycle.clear_app_data_handler());
    assert_eq!(lifecycle.bindings().handler_calls.get(), 2);
    assert_eq!(lifecycle.bindings().last_handler.get(), 0);
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
