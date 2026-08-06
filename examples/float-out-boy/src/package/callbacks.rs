//! Float Out Boy package callback and loader-state plumbing.
//!
//! C map: package init stores loader ARG/stop handlers and registers app-data
//! callbacks at `third_party/float-out-boy/src/main.c:2419-2461`.

use super::state::FloatOutBoyPackageState;
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
#[cfg(test)]
use vescpkg_rs::{Imu, MotorTelemetry};

#[cfg(test)]
pub(crate) fn handle_float_out_boy_app_data_packet(
    state: &mut FloatOutBoyPackageState,
    telemetry: &impl MotorTelemetry,
    imu: &impl Imu,
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
    reply: &mut impl FnMut(&[u8]) -> bool,
    command: FloatOutBoyAppDataCommand,
    payload: &[u8],
) -> bool {
    let _ = imu;
    state.handle_command_with_telemetry(telemetry, now, reply, command, payload)
}

pub(crate) struct FloatOutBoyAppData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatOutBoyCommandHeaderError {
    Truncated(usize),
    InvalidPackageId(u8),
}

fn float_out_boy_command(
    bytes: &[u8],
) -> Result<Option<(FloatOutBoyAppDataCommand, &[u8])>, FloatOutBoyCommandHeaderError> {
    let [package_id, command, payload @ ..] = bytes else {
        return Err(FloatOutBoyCommandHeaderError::Truncated(bytes.len()));
    };
    if *package_id != FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID {
        return Err(FloatOutBoyCommandHeaderError::InvalidPackageId(*package_id));
    }
    Ok(FloatOutBoyAppDataCommand::try_from(*command)
        .ok()
        .map(|command| (command, payload)))
}

#[inline(never)]
fn log_float_out_boy_command_header_error(
    effects: &vescpkg_rs::FirmwareEffects,
    error: FloatOutBoyCommandHeaderError,
) {
    let mut log = vescpkg_rs::FirmwareLog::<64>::new();
    match error {
        FloatOutBoyCommandHeaderError::Truncated(0) => {
            log.write_bytes(b"Received command data too short: 0 bytes.");
        }
        FloatOutBoyCommandHeaderError::Truncated(_) => {
            log.write_bytes(b"Received command data too short: 1 byte.");
        }
        FloatOutBoyCommandHeaderError::InvalidPackageId(package_id) => {
            log.write_bytes(b"Invalid Package ID: ");
            log.write_u8_decimal(package_id);
        }
    }
    let _ = log.flush(effects);
}

fn finish_restored_config(
    context: &mut vescpkg_rs::StatefulCallbackContext<'_, FloatOutBoyPackageState>,
    committed: bool,
) {
    if committed {
        let migration = context.with_effects(super::state::migrate_legacy_firmware_imu_settings);
        context.with_state(|state| state.finish_configure_active(migration));
    }
}

#[cfg_attr(target_arch = "arm", inline(never))]
fn handle_phased_tune_packet(
    context: &mut vescpkg_rs::StatefulCallbackContext<'_, FloatOutBoyPackageState>,
    reply: &mut vescpkg_rs::AppDataReply<'_>,
    command: FloatOutBoyAppDataCommand,
    payload: &[u8],
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
) -> Option<bool> {
    if !matches!(
        command,
        FloatOutBoyAppDataCommand::TuneDefaults
            | FloatOutBoyAppDataCommand::RuntimeTune
            | FloatOutBoyAppDataCommand::TuneTilt
            | FloatOutBoyAppDataCommand::TuneOther
            | FloatOutBoyAppDataCommand::Booster
    ) {
        return None;
    }

    Some(
        reply
            .with_scratch::<{ crate::config::FLOAT_OUT_BOY_CONFIG_LEN }, _>(|config| {
                context.with_state(|state| {
                    config.copy_from_slice(state.serialized_config());
                    let Some(commit) =
                        FloatOutBoyPackageState::prepare_tune_config(config, command, payload)
                    else {
                        return false;
                    };
                    state.commit_prepared_tune(config, commit, now());
                    true
                })
            })
            .unwrap_or(false),
    )
}

#[cfg_attr(target_arch = "arm", inline(never))]
fn handle_effectful_float_out_boy_packet(
    context: &mut vescpkg_rs::StatefulCallbackContext<'_, FloatOutBoyPackageState>,
    command: FloatOutBoyAppDataCommand,
    payload: &[u8],
    now: &mut impl FnMut() -> vescpkg_rs::TimestampTicks,
) -> Option<bool> {
    match command {
        FloatOutBoyAppDataCommand::ConfigSave => {
            let requested_at = now();
            let config =
                context.with_state(|state| state.begin_active_config_persistence(requested_at));
            if let Some(config) = config {
                let stored = context
                    .with_effects(|effects| super::state::store_persisted_config(effects, &config));
                let finished_at = now();
                context.with_state(|state| {
                    state.finish_config_persistence(&config, stored, finished_at);
                });
            }
            Some(true)
        }
        FloatOutBoyAppDataCommand::ConfigRestore => {
            if !context.with_state(super::state::FloatOutBoyPackageState::begin_config_eeprom_read)
            {
                return Some(true);
            }
            let loaded = context.with_effects(super::state::load_persisted_config);
            let restored_at = now();
            context.with_state(|state| {
                state.begin_restore_persisted_config(&loaded, restored_at);
                state.finish_config_eeprom_read();
            });
            finish_restored_config(context, true);
            Some(true)
        }
        FloatOutBoyAppDataCommand::Lock => {
            let Some(disabled) = payload.first() else {
                return Some(false);
            };
            let can_read =
                context.with_state(|state| !state.is_running() && state.begin_config_eeprom_read());
            if can_read {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                let config = context.with_state(|state| {
                    state.apply_lock_from_persisted(&loaded, *disabled != 0, restored_at)
                });
                context
                    .with_state(super::state::FloatOutBoyPackageState::finish_config_eeprom_read);
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
            let Some(restore) = context.with_state(|state| state.prepare_handtest_command(payload))
            else {
                return Some(false);
            };
            if restore {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                let committed =
                    context.with_state(|state| state.commit_handtest_restore(&loaded, restored_at));
                context
                    .with_state(super::state::FloatOutBoyPackageState::finish_config_eeprom_read);
                finish_restored_config(context, committed);
            }
            Some(true)
        }
        FloatOutBoyAppDataCommand::Flywheel => {
            let Some(restore) = context.with_state(|state| state.prepare_flywheel_command(payload))
            else {
                return Some(false);
            };
            if restore {
                let loaded = context.with_effects(super::state::load_persisted_config);
                let restored_at = now();
                context.with_state(|state| state.commit_flywheel_restore(&loaded, restored_at));
                context
                    .with_state(super::state::FloatOutBoyPackageState::finish_config_eeprom_read);
                finish_restored_config(context, true);
            }
            Some(true)
        }
        _ => None,
    }
}

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
        let bytes = packet.as_bytes();
        let (command, payload) = match float_out_boy_command(bytes) {
            Ok(Some(command)) => command,
            Ok(None) => return,
            Err(error) => {
                context.with_effects(|effects| {
                    log_float_out_boy_command_header_error(effects, error);
                });
                return;
            }
        };
        if handle_effectful_float_out_boy_packet(context, command, payload, &mut now).is_some() {
            return;
        }
        if handle_phased_tune_packet(context, reply, command, payload, &mut now).is_some() {
            return;
        }
        let mut write_reply = |bytes: &[u8]| reply.write(bytes).is_ok();
        let _ = context.with_state(|state| {
            state.handle_command_with_telemetry(
                firmware.telemetry(),
                &mut now,
                &mut write_reply,
                command,
                payload,
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
