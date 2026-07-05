#[cfg(all(not(test), target_arch = "arm"))]
use super::refloat_app_data_handler;
#[cfg(any(test, target_arch = "arm"))]
use super::{RefloatPackageLifecycle, RefloatPackageState};
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::RefloatAllDataPayloads;
#[cfg(test)]
use vescpkg_rs::CustomConfigBindings;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::PackageStart;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{AllocBindings, AppDataBindings, FirmwareAllocator, ffi};

/// Install source-startup Refloat state without registering callbacks.
///
/// Upstream allocates `Data`, runs `data_init`, and stores `stop`/`Data *` in
/// loader metadata at `third_party/refloat/src/main.c:2419-2432`; callback/LispBM registration
/// follows at `third_party/refloat/src/main.c:2455-2459`.
///
#[cfg(test)]
fn install_refloat_startup_state_with(
    start: &mut PackageStart,
    state: &mut RefloatPackageState,
) -> bool {
    *state = RefloatPackageState::new(RefloatAllDataPayloads::source_startup());
    start.install_loader_state(super::refloat_stop_handler(), state)
}

/// Install source-startup Refloat state and callback registrations.
///
/// Upstream stores loader metadata at `third_party/refloat/src/main.c:2431-2432` before registering
/// custom config/app-data callbacks at `third_party/refloat/src/main.c:2456-2457`.
///
#[cfg(test)]
fn install_refloat_startup_app_data_with<B: AppDataBindings + CustomConfigBindings>(
    start: &mut PackageStart,
    state: &mut RefloatPackageState,
    lifecycle: &RefloatPackageLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    if !install_refloat_startup_state_with(start, state) {
        return false;
    }
    lifecycle.install_refloat_callbacks(handler).is_ok()
}

/// Allocate and install source-startup Refloat state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `third_party/refloat/src/main.c:2419`, runs
/// `data_init` at `third_party/refloat/src/main.c:2424`, and stores the same pointer in
/// `info->arg` at `third_party/refloat/src/main.c:2432`. This Rust path still allocates a narrow
/// `RefloatPackageState`, but keeps the same loader metadata order before the
/// registration tail at `third_party/refloat/src/main.c:2455-2459`.
///
#[cfg(any(test, target_arch = "arm"))]
fn allocate_refloat_startup_state_with<A: AllocBindings>(
    start: &mut PackageStart,
    allocator: &FirmwareAllocator<'_, A>,
) -> bool {
    start.allocate_loader_state(
        allocator,
        super::refloat_stop_handler(),
        RefloatPackageState::new(RefloatAllDataPayloads::source_startup()),
    )
}

/// Allocate source-startup Refloat state and register app-data callbacks.
///
/// Upstream performs state setup at `third_party/refloat/src/main.c:2419-2432`, starts runtime
/// threads at `third_party/refloat/src/main.c:2439-2449`, then registers custom config/app-data
/// callbacks at `third_party/refloat/src/main.c:2456-2457` after IMU setup. This compatibility
/// helper only keeps state-before-callback order for tests.
///
#[cfg(test)]
fn allocate_refloat_startup_app_data_with<
    A: AllocBindings,
    B: AppDataBindings + CustomConfigBindings,
>(
    start: &mut PackageStart,
    allocator: &FirmwareAllocator<'_, A>,
    lifecycle: &RefloatPackageLifecycle<B>,
    handler: ffi::AppDataHandler,
) -> bool {
    if !allocate_refloat_startup_state_with(start, allocator) {
        return false;
    }

    if lifecycle.install_refloat_callbacks(handler).is_err() {
        start.clear_loader_info();
        return false;
    }

    true
}

/// Allocate and install Refloat startup state using firmware memory.
///
/// This matches the loader metadata step from upstream `third_party/refloat/src/main.c:2419-2432`;
/// callback/LispBM registration is a separate step at `third_party/refloat/src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_refloat_package_state(start: &mut PackageStart) -> bool {
    let alloc_bindings = vescpkg_rs::RealBindings;
    let allocator = vescpkg_rs::FirmwareAllocator::new(&alloc_bindings);
    allocate_refloat_startup_state_with(start, &allocator)
}

/// Register Refloat custom config and app-data callbacks.
///
/// Upstream registers these callbacks at `third_party/refloat/src/main.c:2456-2457`, after runtime
/// thread startup at `third_party/refloat/src/main.c:2439-2449` and IMU setup at
/// `third_party/refloat/src/main.c:2455`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_refloat_app_data_callbacks(_start: &mut PackageStart) -> bool {
    let lifecycle = RefloatPackageLifecycle::new(vescpkg_rs::RealBindings);
    let handler = refloat_app_data_handler();
    lifecycle.install_refloat_callbacks(handler).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{allocate_refloat_startup_app_data_with, install_refloat_startup_app_data_with};
    use crate::domain::RefloatAllDataPayloads;
    use crate::package::test_support::{
        RecordingAllocBindings, RecordingAppDataBindings, sample_all_data_payloads,
    };
    use crate::package::{RefloatPackageLifecycle, RefloatPackageState};
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use vescpkg_rs::{FirmwareAllocator, ffi};

    struct NoopAppData;

    impl vescpkg_rs::AppDataCallback for NoopAppData {
        fn handle(_packet: ffi::AppDataPacket<'static>) {}
    }

    fn noop_app_data_handler() -> ffi::AppDataHandler {
        vescpkg_rs::app_data_callback::<NoopAppData>
    }

    #[test]
    fn startup_app_data_install_seeds_state_and_registers_handler() {
        let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut state = RefloatPackageState::new(sample_all_data_payloads());
        let mut start = vescpkg_rs::PackageStart::from_raw(&mut info);

        assert!(install_refloat_startup_app_data_with(
            &mut start,
            &mut state,
            &lifecycle,
            noop_app_data_handler()
        ));
        assert_eq!(lifecycle.bindings().handler_calls.get(), 1);
        assert_eq!(
            state.all_data_payloads(),
            RefloatAllDataPayloads::source_startup()
        );
        assert_eq!(
            RefloatPackageState::from_info_arg(&mut info)
                .expect("installed state")
                .all_data_payloads(),
            RefloatAllDataPayloads::source_startup(),
        );
    }

    #[test]
    fn startup_app_data_install_uses_firmware_allocated_state() {
        let lifecycle = RefloatPackageLifecycle::new(RecordingAppDataBindings::accepting());
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut backing = MaybeUninit::<RefloatPackageState>::uninit();
        let alloc_bindings = RecordingAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = FirmwareAllocator::new(&alloc_bindings);
        let mut start = vescpkg_rs::PackageStart::from_raw(&mut info);

        assert!(allocate_refloat_startup_app_data_with(
            &mut start,
            &allocator,
            &lifecycle,
            noop_app_data_handler()
        ));
        assert_eq!(lifecycle.bindings().custom_config_register_calls.get(), 1);
        assert_eq!(alloc_bindings.malloc_calls.get(), 1);
        assert_eq!(
            alloc_bindings.last_requested_len.get(),
            core::mem::size_of::<RefloatPackageState>()
        );
        assert_eq!(alloc_bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        let allocated_state =
            RefloatPackageState::from_info_arg(&mut info).expect("allocated state");
        assert_eq!(
            *allocated_state,
            RefloatPackageState::new(RefloatAllDataPayloads::source_startup()),
        );
    }
}
