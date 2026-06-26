#![cfg_attr(not(test), no_std)]

pub use vesc_protocol::{Frame as ProtocolFrame, WireCommand, WireVersion};

pub mod ble_loopback_device;
pub mod ffi;
pub mod package_lifecycle;

#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) {
    package_lifecycle::init_package();
    ble_loopback_device::init_package(info);
}

#[cfg(test)]
pub extern "C" fn package_lib_init(info: *mut ffi::LibInfo) {
    ble_loopback_device::init_package_for_tests(info);
}

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::{ble_loopback_device, ffi, rust_add, ProtocolFrame, WireCommand, WireVersion};

    #[test]
    fn cargo_test_smoke() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn device_side_can_use_the_shared_protocol_crate() {
        let frame = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[7, 8]);

        assert_eq!(frame.version(), WireVersion::CURRENT);
        assert_eq!(frame.command(), WireCommand::Ping);
        assert_eq!(frame.payload(), &[7, 8]);
    }

    #[test]
    fn rust_add_stays_a_plain_integer_function() {
        assert_eq!(rust_add(1, 2), 3);
        assert_eq!(rust_add(-8, 11), 3);
    }

    #[test]
    fn package_lib_init_runs_the_device_loopback_entrypoint_path() {
        ble_loopback_device::reset_init_call_count_for_tests();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        super::package_lib_init(&mut info);

        assert_eq!(ble_loopback_device::init_call_count_for_tests(), 1);
        assert!(info.stop_fun.is_some());
    }
}
