use super::super::protocol::wire::{
    float_out_boy_realtime_push_u8, float_out_boy_realtime_push_u32,
};
use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{FirmwareFaultWireCode, TimestampTicks};

const ALERTS_RESPONSE_CAPACITY: usize = 511;
const FAULT_NAME_MAX_BYTES: usize = 50;
const FAULT_NAME_PREFIX_BYTES: usize = 11;

impl FloatOutBoyPackageState {
    pub(super) fn handle_alert_packet(
        &mut self,
        telemetry: &impl MotorTelemetry,
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        if let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::AlertsList)
        {
            let since = match payload {
                [a, b, c, d, ..] => {
                    TimestampTicks::from_ticks(u32::from_be_bytes([*a, *b, *c, *d]))
                }
                _ => TimestampTicks::from_ticks(0),
            };
            let mut response = [0; ALERTS_RESPONSE_CAPACITY];
            let mut index = 0;
            float_out_boy_realtime_push_u8(
                &mut response,
                &mut index,
                FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get(),
            );
            float_out_boy_realtime_push_u8(
                &mut response,
                &mut index,
                FloatOutBoyAppDataCommand::AlertsList.id(),
            );
            float_out_boy_realtime_push_u32(
                &mut response,
                &mut index,
                self.alert_tracker
                    .active_alerts()
                    .active_alert_mask_compat(),
            );
            float_out_boy_realtime_push_u32(&mut response, &mut index, 0);
            let fault = self.alert_tracker.firmware_fault_code();
            float_out_boy_realtime_push_u8(&mut response, &mut index, fault.wire_code());
            push_fault_name(&mut response, &mut index, telemetry, fault);
            let count_index = index;
            float_out_boy_realtime_push_u8(&mut response, &mut index, 0);
            let mut count = 0_u8;
            self.alert_tracker.for_each_record_since(since, |record| {
                if response.len().saturating_sub(index) < 58 {
                    return false;
                }
                float_out_boy_realtime_push_u32(
                    &mut response,
                    &mut index,
                    record.timestamp.as_ticks(),
                );
                float_out_boy_realtime_push_u8(&mut response, &mut index, record.id.id());
                float_out_boy_realtime_push_u8(&mut response, &mut index, u8::from(record.active));
                float_out_boy_realtime_push_u8(&mut response, &mut index, record.code.wire_code());
                push_fault_name(&mut response, &mut index, telemetry, record.code);
                count = count.saturating_add(1);
                true
            });
            if let Some(count_slot) = response.get_mut(count_index) {
                *count_slot = count;
            }
            return response.get(..index).is_some_and(send);
        }

        if let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::AlertsControl)
        {
            if payload.first() == Some(&1) {
                self.alert_tracker.clear_fatal();
            }
            return true;
        }

        false
    }
}

fn push_fault_name(
    buffer: &mut [u8],
    index: &mut usize,
    telemetry: &impl MotorTelemetry,
    code: FirmwareFaultWireCode,
) {
    if code.wire_code() == 0 {
        return;
    }

    let name = bounded_fault_name(
        telemetry
            .firmware_fault_description()
            .unwrap_or_default()
            .as_bytes(),
    );
    float_out_boy_realtime_push_u8(buffer, index, u8::try_from(name.len()).unwrap_or(u8::MAX));
    crate::wire::push_bytes(buffer, index, name);
}

fn bounded_fault_name(name: &[u8]) -> &[u8] {
    let name = if name.len() > FAULT_NAME_PREFIX_BYTES && name.first() == Some(&b'F') {
        name.get(FAULT_NAME_PREFIX_BYTES..).unwrap_or(name)
    } else {
        name
    };
    name.get(..FAULT_NAME_MAX_BYTES).unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::{FAULT_NAME_MAX_BYTES, bounded_fault_name};

    #[test]
    fn fault_names_match_refloats_fifty_byte_wire_limit() {
        let long_name = [b'X'; FAULT_NAME_MAX_BYTES + 1];

        assert_eq!(bounded_fault_name(b"SHORT"), b"SHORT");
        assert_eq!(
            bounded_fault_name(&long_name),
            &long_name[..FAULT_NAME_MAX_BYTES],
        );
    }

    #[test]
    fn fault_names_drop_refloats_firmware_prefix() {
        assert_eq!(
            bounded_fault_name(b"FAULT_CODE_OVER_VOLTAGE"),
            b"OVER_VOLTAGE",
        );
    }
}
