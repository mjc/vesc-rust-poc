//! Float Out Boy package callback and loader-state plumbing.
//!
//! C map: package init stores loader ARG/stop handlers and registers app-data
//! callbacks at `third_party/float-out-boy/src/main.c:2419-2461`.

#[cfg(any(test, target_arch = "arm"))]
use super::state::FloatOutBoyPackageState;
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::{Imu, MotorTelemetry};

#[cfg(any(test, target_arch = "arm"))]
pub(crate) fn handle_float_out_boy_app_data_packet(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
    reply: &mut impl FnMut(&[u8]) -> bool,
    packet: vescpkg_rs::AppDataPacket<'_>,
) -> bool {
    state.handle_packet_with_runtime(telemetry, imu, now, reply, packet.as_bytes())
}

#[cfg(any(test, target_arch = "arm"))]
pub(crate) struct FloatOutBoyAppData;

#[cfg(any(test, target_arch = "arm"))]
fn float_out_boy_command(bytes: &[u8]) -> Option<(FloatOutBoyAppDataCommand, &[u8])> {
    let [package_id, command, payload @ ..] = bytes else {
        return None;
    };
    if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get() {
        return None;
    }
    FloatOutBoyAppDataCommand::try_from(*command)
        .ok()
        .map(|command| (command, payload))
}

#[cfg(any(test, target_arch = "arm"))]
fn finish_restored_config(
    context: &mut vescpkg_rs::StatefulCallbackContext<'_, FloatOutBoyPackageState>,
    committed: bool,
) {
    if committed {
        let migration = context.with_effects(super::state::migrate_legacy_firmware_imu_settings);
        context.with_state(|state| state.finish_configure_active(migration));
    }
}

#[cfg(any(test, target_arch = "arm"))]
#[cfg_attr(target_arch = "arm", inline(never))]
fn handle_effectful_float_out_boy_packet(
    context: &mut vescpkg_rs::StatefulCallbackContext<'_, FloatOutBoyPackageState>,
    bytes: &[u8],
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
) -> Option<bool> {
    let (command, payload) = float_out_boy_command(bytes)?;
    match command {
        FloatOutBoyAppDataCommand::ConfigSave => {
            let config = context.with_state(|state| state.active_config_image());
            let stored = context
                .with_effects(|effects| super::state::store_persisted_config(effects, &config));
            if stored {
                let written_at = now();
                context.with_state(|state| state.acknowledge_command_config_write(written_at));
            }
            Some(true)
        }
        FloatOutBoyAppDataCommand::ConfigRestore => {
            let loaded = context.with_effects(super::state::load_persisted_config);
            let restored_at = now();
            context.with_state(|state| state.begin_restore_persisted_config(&loaded, restored_at));
            finish_restored_config(context, true);
            Some(true)
        }
        FloatOutBoyAppDataCommand::Lock => {
            let Some(disabled) = payload.first() else {
                return Some(false);
            };
            if !context.with_state(|state| state.is_running()) {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                let config = context.with_state(|state| {
                    state.apply_lock_from_persisted(&loaded, *disabled != 0, restored_at)
                });
                if let Some(config) = config {
                    let stored = context.with_effects(|effects| {
                        super::state::store_persisted_config(effects, &config)
                    });
                    if stored {
                        let written_at = now();
                        context
                            .with_state(|state| state.acknowledge_command_config_write(written_at));
                    }
                    finish_restored_config(context, true);
                }
            }
            Some(true)
        }
        FloatOutBoyAppDataCommand::HandTest => {
            let Some(restore) = context.with_state(|state| state.prepare_handtest_packet(bytes))
            else {
                return Some(false);
            };
            if restore {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                let committed =
                    context.with_state(|state| state.commit_handtest_restore(&loaded, restored_at));
                finish_restored_config(context, committed);
            }
            Some(true)
        }
        FloatOutBoyAppDataCommand::Flywheel => {
            let Some(restore) = context.with_state(|state| state.prepare_flywheel_packet(bytes))
            else {
                return Some(false);
            };
            if restore {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                context.with_state(|state| state.commit_flywheel_restore(&loaded, restored_at));
                finish_restored_config(context, true);
            }
            Some(true)
        }
        _ => None,
    }
}

#[cfg(any(test, target_arch = "arm"))]
impl vescpkg_rs::AppDataHandler for FloatOutBoyAppData {
    type State = FloatOutBoyPackageState;

    fn handle(
        context: &mut vescpkg_rs::StatefulCallbackContext<'_, Self::State>,
        packet: vescpkg_rs::AppDataPacket<'_>,
        reply: &mut vescpkg_rs::AppDataReply<'_>,
    ) {
        // C map: upstream `on_command_received` recovers `Data *` through
        // `ARG(PROG_ADDR)` before app-data dispatch at
        // `third_party/float-out-boy/src/main.c:2143-2225`.
        let firmware = vescpkg_rs::Firmware::new();
        let mut now = || firmware.clock().now();
        let mut write_reply = |bytes: &[u8]| reply.write(bytes).is_ok();
        if handle_effectful_float_out_boy_packet(context, packet.as_bytes(), &mut now).is_some() {
            return;
        }
        let _ = context.with_state(|state| {
            handle_float_out_boy_app_data_packet(
                state,
                firmware.telemetry(),
                firmware.imu(),
                &mut now,
                &mut write_reply,
                packet,
            )
        });
    }
}

vescpkg_rs::firmware_stateful_app_data_callback!(
    float_out_boy_app_data_callback,
    FloatOutBoyAppData
);

#[cfg(test)]
mod tests;
