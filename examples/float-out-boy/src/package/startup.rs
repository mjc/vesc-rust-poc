#[cfg(any(test, target_arch = "arm"))]
use super::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::FloatOutBoyAllDataPayloads;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::PackageStart;

/// Install source-startup Float Out Boy state without registering callbacks.
///
/// Upstream allocates `Data`, runs `data_init`, and stores `stop`/`Data *` in
/// loader metadata at `third_party/float-out-boy/src/main.c:2419-2432`; callback/LispBM registration
/// follows at `third_party/float-out-boy/src/main.c:2455-2459`.
///
#[cfg(test)]
fn install_float_out_boy_startup_state_with(
    start: &mut PackageStart,
    state: &mut FloatOutBoyPackageState,
) -> bool {
    *state = FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup());
    start
        .install_runtime_state_with(
            core::mem::replace(
                state,
                FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup()),
            ),
            FloatOutBoyPackageState::load_persisted_config_on_startup,
        )
        .is_ok()
}

/// Allocate and install source-startup Float Out Boy state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `third_party/float-out-boy/src/main.c:2419`, runs
/// `data_init` at `third_party/float-out-boy/src/main.c:2424`, and stores the same pointer in
/// `info->arg` at `third_party/float-out-boy/src/main.c:2432`. This Rust path still allocates a narrow
/// `FloatOutBoyPackageState`, but keeps the same loader metadata order before the
/// registration tail at `third_party/float-out-boy/src/main.c:2455-2459`.
///
#[cfg(target_arch = "arm")]
fn allocate_float_out_boy_startup_state(
    start: &mut PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state_with(
        FloatOutBoyPackageState::new(FloatOutBoyAllDataPayloads::source_startup()),
        FloatOutBoyPackageState::load_persisted_config_on_startup,
    )
}

/// Allocate and install Float Out Boy startup state using firmware memory.
///
/// This matches the loader metadata step from upstream `third_party/float-out-boy/src/main.c:2419-2432`;
/// callback/LispBM registration is a separate step at `third_party/float-out-boy/src/main.c:2455-2459`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn install_float_out_boy_package_state(
    start: &mut PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    allocate_float_out_boy_startup_state(start)
}

/// Register Float Out Boy custom config and app-data callbacks.
///
/// Upstream registers these callbacks at `third_party/float-out-boy/src/main.c:2455-2456`, after runtime
/// thread startup at `third_party/float-out-boy/src/main.c:2439-2449` and IMU setup at
/// `third_party/float-out-boy/src/main.c:2454`.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_float_out_boy_app_data_callbacks(start: &mut PackageStart) -> bool {
    super::custom_config::register_float_out_boy_callbacks(start).is_ok()
}

#[cfg(test)]
mod tests {
    use crate::package::FloatOutBoyPackageState;
    use crate::package::test_support::{
        lock_float_out_boy_runtime_state, sample_all_data_payloads,
    };

    fn assert_no_runtime_state() {
        assert!(!crate::__VESCPKG_PACKAGE_STATE.is_installed());
    }

    #[test]
    fn startup_state_install_rejects_null_loader_metadata_without_runtime_slot() {
        let _runtime_state = lock_float_out_boy_runtime_state();
        let mut start = vescpkg_rs::test_support::package_start_without_loader();
        let mut state = FloatOutBoyPackageState::new(sample_all_data_payloads());

        assert!(!super::install_float_out_boy_startup_state_with(
            &mut start, &mut state
        ));
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
        let state = FloatOutBoyPackageState::new(sample_all_data_payloads());

        assert_eq!(start.install_runtime_state(state), Ok(()));
        // Upstream sets `info->stop_fun` and `info->arg` at `third_party/float-out-boy/src/main.c:2431-2432`,
        // before registering custom config/app-data/extensions at `third_party/float-out-boy/src/main.c:2455-2459`.
        assert!(start.finish_start(true));
        assert!(info.has_stop_handler());
        assert!(info.argument().is_some());
        assert!(vescpkg_rs::test_support::stop_package(&mut info));
    }
}
