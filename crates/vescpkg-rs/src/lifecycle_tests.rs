use crate::MotorTelemetryApi;
use crate::ble_loopback::register_loopback_app_data_handler_with;
use crate::extension::ExtensionDescriptor;
use crate::lifecycle::register_extension_from_image;
use crate::lifecycle_core::{
    AppDataHandlerRegistrationError, LbmApi, LoopbackLifecycle, PackageLifecycle,
};
use crate::test_support::{FakeAppDataBindings, FakeBindings, FakeMotorTelemetryBindings, stubs};
use crate::types::{
    AmpHoursCharged, AmpHoursDischarged, BatteryLevel, FirmwareFaultCode, InputVoltage,
    MosfetTemperature, MotorTemperature, TripDistance, WattHoursCharged, WattHoursDischarged,
};
use crate::units::{Charge, Distance, Energy, OdometerMeters, Ratio, Temperature, Voltage};
use crate::{RegisterError, ffi};
use rstest::rstest;
use vescpkg_rs_sys::{ExtensionHandler, LbmValue, LibInfo, NativeImage};

unsafe extern "C" fn stub_handler(_args: *mut u32, _count: u32) -> u32 {
    0
}

const EXT_HOST_TEST_PROBE_NAME: &core::ffi::CStr = c"ext-c-probe-v12";

fn handler_at_offset(offset: usize) -> ExtensionHandler {
    unsafe { core::mem::transmute(offset) }
}

#[rstest]
#[case::invalid_name(c"bad-name", "invalid", 0_usize)]
#[case::missing_ext_prefix(c"rust-probe-v5", "invalid", 0_usize)]
#[case::firmware_reject(c"ext-rust-reject", "reject", 1_usize)]
#[case::success(c"ext-rust-ok", "accept", 1_usize)]
fn register_extension_reports_outcome(
    #[case] name: &'static core::ffi::CStr,
    #[case] mode: &'static str,
    #[case] expected_add_calls: usize,
) {
    let bindings = if mode == "reject" {
        FakeBindings::rejecting()
    } else {
        FakeBindings::new()
    };
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(name, stub_handler);

    let result = lifecycle.register_extension(descriptor);

    match mode {
        "invalid" => assert_eq!(result, Err(RegisterError::InvalidExtensionName)),
        "reject" => assert_eq!(result, Err(RegisterError::FirmwareRejected)),
        "accept" => assert_eq!(result, Ok(())),
        other => panic!("unexpected mode: {other}"),
    }
    assert_eq!(lifecycle.bindings().add_calls.get(), expected_add_calls);
}

#[rstest]
#[case::rebase_handler(c"ext-test", 0x31_usize, 0x2000_u32, "accept", true, 0x2031_usize)]
#[case::firmware_reject(c"ext-c-probe-v12", 0_usize, 0x2000_u32, "reject", false, 0_usize)]
fn register_extension_from_image_reports_outcome(
    #[case] name: &'static core::ffi::CStr,
    #[case] handler_offset: usize,
    #[case] base_addr: u32,
    #[case] mode: &'static str,
    #[case] check_last_handler: bool,
    #[case] expected_last_handler: usize,
) {
    let bindings = if mode == "reject" {
        FakeBindings::rejecting()
    } else {
        FakeBindings::new()
    };
    let lifecycle = PackageLifecycle::new(bindings);
    let handler = if handler_offset == 0 {
        stubs::extension_handler
    } else {
        handler_at_offset(handler_offset)
    };
    let descriptor = ExtensionDescriptor::new(name, handler);
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr,
    };

    // Test metadata models a loaded image whose handler offsets are intentionally rebased.
    let result = unsafe { register_extension_from_image(&info, &lifecycle, descriptor) };

    match mode {
        "accept" => assert_eq!(result, Ok(())),
        "reject" => assert_eq!(result, Err(RegisterError::FirmwareRejected)),
        other => panic!("unexpected mode: {other}"),
    }
    if check_last_handler {
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            expected_last_handler
        );
    }
    if mode == "reject" {
        assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    }
}

#[rstest]
#[case::decode_i32(LbmValue(3), 3)]
fn lbm_api_decode_i32_forwards_through_bindings(#[case] input: LbmValue, #[case] expected: i32) {
    let api = LbmApi::new(FakeBindings::new());
    assert_eq!(api.decode_i32(input), expected);
}

#[rstest]
#[case::encode_i32(9, LbmValue(9))]
fn lbm_api_encode_i32_forwards_through_bindings(#[case] input: i32, #[case] expected: LbmValue) {
    let api = LbmApi::new(FakeBindings::new());
    assert_eq!(api.encode_i32(input), expected);
}

#[rstest]
#[case::is_number(LbmValue(9), true)]
fn lbm_api_is_number_forwards_through_bindings(#[case] input: LbmValue, #[case] expected: bool) {
    let api = LbmApi::new(FakeBindings::new());
    assert_eq!(api.is_number(input), expected);
}

#[test]
fn lbm_api_encode_eval_error_forwards_through_bindings() {
    let api = LbmApi::new(FakeBindings::new());
    assert_eq!(api.encode_eval_error(), LbmValue(0xffff_ffff));
}

#[test]
fn lbm_api_registers_extensions_through_bindings() {
    let api = LbmApi::new(FakeBindings::new());
    assert!(api.register_extension(c"ext-rust-add", stub_handler));
}

#[rstest]
#[case::default_ticks(0, 0)]
#[case::configured_ticks(1234, 1234)]
fn loopback_lifecycle_forwards_system_time_ticks(
    #[case] configured_ticks: u32,
    #[case] expected_ticks: u32,
) {
    let lifecycle = LoopbackLifecycle::new(FakeAppDataBindings::with_ticks(configured_ticks));
    assert_eq!(lifecycle.system_time_ticks(), expected_ticks);
}

#[test]
fn motor_telemetry_api_forwards_absolute_distance_as_trip_distance() {
    let distance = TripDistance::new(Distance::from_meters(12.5));
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_distance_abs(distance));

    assert_eq!(telemetry.distance_abs(), distance);
    assert_eq!(telemetry.bindings().distance_abs_calls.get(), 1);
}

#[test]
fn motor_telemetry_api_forwards_filtered_motor_temperatures() {
    let mosfet = MosfetTemperature::new(Temperature::from_degrees_celsius(44.0));
    let motor = MotorTemperature::new(Temperature::from_degrees_celsius(51.5));
    let telemetry =
        MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_temperatures(mosfet, motor));

    assert_eq!(telemetry.mosfet_temperature(), mosfet);
    assert_eq!(telemetry.motor_temperature(), motor);
    assert_eq!(telemetry.bindings().mosfet_temperature_calls.get(), 1);
    assert_eq!(telemetry.bindings().motor_temperature_calls.get(), 1);
}

#[test]
fn motor_telemetry_api_forwards_accumulated_ride_totals() {
    let odometer = OdometerMeters::from_meters(123_456);
    let discharged_charge = AmpHoursDischarged::new(Charge::from_amp_hours(3.2));
    let charged_charge = AmpHoursCharged::new(Charge::from_amp_hours(0.8));
    let discharged_energy = WattHoursDischarged::new(Energy::from_watt_hours(170.0));
    let charged_energy = WattHoursCharged::new(Energy::from_watt_hours(18.5));
    let battery_level = BatteryLevel::new(Ratio::from_ratio_const(0.72));
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_ride_totals(
        odometer,
        discharged_charge,
        charged_charge,
        discharged_energy,
        charged_energy,
        battery_level,
    ));

    assert_eq!(telemetry.odometer(), odometer);
    assert_eq!(telemetry.amp_hours_discharged(), discharged_charge);
    assert_eq!(telemetry.amp_hours_charged(), charged_charge);
    assert_eq!(telemetry.watt_hours_discharged(), discharged_energy);
    assert_eq!(telemetry.watt_hours_charged(), charged_energy);
    assert_eq!(telemetry.battery_level(), battery_level);
    assert_eq!(telemetry.bindings().odometer_calls.get(), 1);
    assert_eq!(telemetry.bindings().amp_hours_discharged_calls.get(), 1);
    assert_eq!(telemetry.bindings().amp_hours_charged_calls.get(), 1);
    assert_eq!(telemetry.bindings().watt_hours_discharged_calls.get(), 1);
    assert_eq!(telemetry.bindings().watt_hours_charged_calls.get(), 1);
    assert_eq!(telemetry.bindings().battery_level_calls.get(), 1);
}

#[test]
fn motor_telemetry_api_forwards_firmware_fault_code() {
    let fault = FirmwareFaultCode::from_compat_code(5);
    let telemetry = MotorTelemetryApi::new(FakeMotorTelemetryBindings::with_firmware_fault(fault));

    assert_eq!(telemetry.firmware_fault(), fault);
    assert_eq!(telemetry.bindings().firmware_fault_calls.get(), 1);
}

#[test]
fn motor_telemetry_api_forwards_filtered_input_voltage() {
    let voltage = InputVoltage::new(Voltage::from_volts(84.2));
    let telemetry = MotorTelemetryApi::new(
        FakeMotorTelemetryBindings::with_input_voltage_filtered(voltage),
    );

    assert_eq!(telemetry.input_voltage_filtered(), voltage);
    assert_eq!(telemetry.bindings().input_voltage_filtered_calls.get(), 1);
}

#[rstest]
#[case::three_byte_payload([1_u8, 2, 3], 3)]
fn loopback_lifecycle_forwards_send_app_data(#[case] payload: [u8; 3], #[case] expected_len: u32) {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);
    unsafe { lifecycle.send_app_data(payload.as_ptr(), expected_len) };

    assert_eq!(lifecycle.bindings().send_calls.get(), 1);
    assert_eq!(lifecycle.bindings().last_len.get(), expected_len);
    assert_eq!(
        lifecycle.bindings().last_data.get(),
        payload.as_ptr() as usize
    );
}

#[rstest]
#[case::register_stub("register", 1, true)]
#[case::register_custom("register", 1, false)]
#[case::clear("clear", 1, false)]
fn loopback_lifecycle_app_data_handler_forwards_to_bindings(
    #[case] mode: &'static str,
    #[case] expected_handler_calls: usize,
    #[case] use_stub_handler: bool,
) {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    unsafe extern "C" fn custom_handler(_data: *mut u8, _len: u32) {}

    if mode == "register" {
        let registered = if use_stub_handler {
            stubs::app_data_handler
        } else {
            custom_handler
        };
        assert_eq!(lifecycle.register_app_data_handler(registered), Ok(()));
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            registered as *const () as usize
        );
    } else {
        assert_eq!(lifecycle.clear_app_data_handler(), Ok(()));
        assert_eq!(lifecycle.bindings().last_handler.get(), 0);
    }

    assert_eq!(
        lifecycle.bindings().handler_calls.get(),
        expected_handler_calls
    );
}

#[test]
fn register_loopback_app_data_handler_with_forwards_to_bindings() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);

    assert_eq!(
        register_loopback_app_data_handler_with(&lifecycle, stubs::app_data_handler),
        Ok(())
    );
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
}

#[test]
fn app_data_handler_registration_reports_firmware_rejection() {
    let bindings = FakeAppDataBindings::with_handler_results([false, true]);
    let lifecycle = LoopbackLifecycle::new(bindings);

    assert_eq!(
        lifecycle.register_app_data_handler(stubs::app_data_handler),
        Err(AppDataHandlerRegistrationError::FirmwareRejected)
    );
    assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
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
        unsafe { lifecycle.register_extensions_from_image(image, [first, second]) },
        Ok(())
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 2);
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
fn registers_an_extension_through_the_lifecycle_helper() {
    let bindings = FakeBindings::new();
    let lifecycle = PackageLifecycle::new(bindings);
    let descriptor = ExtensionDescriptor::new(EXT_HOST_TEST_PROBE_NAME, stubs::extension_handler);
    let info = LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    assert_eq!(
        unsafe { register_extension_from_image(&info, &lifecycle, descriptor) },
        Ok(())
    );
    assert_eq!(lifecycle.bindings().add_calls.get(), 1);
    assert_eq!(
        EXT_HOST_TEST_PROBE_NAME.to_bytes_with_nul(),
        b"ext-c-probe-v12\0"
    );
}

#[test]
fn lifecycle_descriptor_installs_the_stop_hook() {
    let bindings = FakeAppDataBindings::new();
    let lifecycle = LoopbackLifecycle::new(bindings);
    let mut info = ffi::LibInfo {
        stop_fun: None,
        arg: core::ptr::null_mut(),
        base_addr: 0x2000,
    };

    assert!(unsafe { lifecycle.install(&mut info, stubs::stop_handler, stubs::app_data_handler) });

    assert_eq!(
        info.stop_fun.expect("stop hook") as *const () as usize,
        stubs::stop_handler as *const () as usize
    );
    assert_eq!(lifecycle.bindings().handler_calls.get(), 0);
}
