#[cfg(any(test, target_arch = "arm"))]
use super::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::FloatOutBoyAllDataPayloads;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::PackageStart;

/// Allocate and install source-startup Float Out Boy state through firmware memory.
///
/// Upstream uses firmware `malloc(sizeof(Data))` at `third_party/float-out-boy/src/main.c:2419`,
/// reads config in `data_init` at `third_party/float-out-boy/src/main.c:2424`, and stores the same
/// pointer in `info->arg` at `third_party/float-out-boy/src/main.c:2432`. Rust defers the EEPROM read
/// to the main-thread entry because its compiled 1976-byte loader call chain leaves almost no margin
/// in VESC's 2048-byte evaluator stack. This path
/// still installs the narrow `FloatOutBoyPackageState` before the registration tail at
/// `third_party/float-out-boy/src/main.c:2455-2459`.
///
#[cfg(any(test, target_arch = "arm"))]
fn allocate_float_out_boy_startup_state(
    start: &mut PackageStart,
) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(FloatOutBoyPackageState::new(
        FloatOutBoyAllDataPayloads::source_startup(),
    ))?;
    #[cfg(target_arch = "arm")]
    {
        let buffer = start.take_data_recorder_buffer();
        start
            .with_runtime_state::<FloatOutBoyPackageState, _>(|state| {
                state.initialize_data_recorder(buffer);
            })
            .ok_or(vescpkg_rs::PackageStartError::StateTypeMismatch)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Ok(())
    }
}

/// Allocate and install Float Out Boy startup state using firmware memory.
///
/// This matches the loader metadata step from upstream `third_party/float-out-boy/src/main.c:2419-2432`;
/// callback/LispBM registration is a separate step at `third_party/float-out-boy/src/main.c:2455-2459`.
#[cfg(any(test, target_arch = "arm"))]
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
mod tests;
