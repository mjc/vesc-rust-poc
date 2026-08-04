use crate::package::FloatOutBoyPackageState;
use crate::package::test_support::{lock_float_out_boy_runtime_state, sample_all_data_payloads};

fn assert_no_runtime_state() {
    assert!(!crate::__VESCPKG_PACKAGE_STATE.is_installed());
}

#[test]
fn startup_state_install_rejects_null_loader_metadata_without_runtime_slot() {
    let _runtime_state = lock_float_out_boy_runtime_state();
    let mut start = vescpkg_rs::test_support::package_start_without_loader();

    assert!(super::install_float_out_boy_package_state(&mut start).is_err());
    // C map: upstream writes `info->stop_fun` and `info->arg` at
    // `third_party/float-out-boy/src/main.c:2431-2432`; without loader metadata,
    // Rust must fail closed and keep custom-config state unreachable.
    assert_no_runtime_state();
}

#[test]
fn package_start_installs_typed_float_out_boy_state_for_handler_retrieval() {
    let _runtime_state = lock_float_out_boy_runtime_state();
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

    assert_eq!(start.install_runtime_state(state), Ok(()));
    // C map: Float Out Boy stores `Data *` in `info->arg` at
    // `third_party/float-out-boy/src/main.c:2432`; app-data/custom-config paths
    // recover package state through the same loader metadata boundary.
    assert_eq!(
        start
            .with_runtime_state::<FloatOutBoyPackageState, _>(|state| state.all_data_payloads())
            .expect("installed state"),
        sample_all_data_payloads()
    );
    let mut empty_info = vescpkg_rs::test_support::LoaderInfo::new();
    assert!(
        vescpkg_rs::test_support::package_start(&mut empty_info)
            .with_runtime_state::<FloatOutBoyPackageState, _>(|_| ())
            .is_none()
    );
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));
}

#[test]
fn package_start_installs_float_out_boy_state_before_callbacks_like_float_out_boy_startup() {
    let _runtime_state = lock_float_out_boy_runtime_state();
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);

    assert_eq!(
        super::install_float_out_boy_package_state(&mut start),
        Ok(())
    );
    assert_eq!(
        start.with_runtime_state::<FloatOutBoyPackageState, _>(|state| *state),
        Some(FloatOutBoyPackageState::default())
    );
    // Upstream sets `info->stop_fun` and `info->arg` at `third_party/float-out-boy/src/main.c:2431-2432`,
    // before registering custom config/app-data/extensions at `third_party/float-out-boy/src/main.c:2455-2459`.
    assert!(start.finish_start(true));
    assert!(info.has_stop_handler());
    assert!(info.argument().is_some());
    assert!(vescpkg_rs::test_support::stop_package(&mut info));
}
