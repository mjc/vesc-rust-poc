use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use crate::wire::FloatOutBoyPacket;
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{FirmwareFaultWireCode, TimestampTicks};

const ALERTS_RESPONSE_CAPACITY: usize = 511;
const FAULT_NAME_MAX_BYTES: usize = 50;
const FAULT_NAME_PREFIX_BYTES: usize = 11;

impl FloatOutBoyPackageState {
    pub(super) fn handle_alert_packet(
        &mut self,
        telemetry: &impl MotorTelemetry,
        reply: &mut impl FnMut(&[u8]) -> bool,
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
            let mut response = FloatOutBoyPacket::<ALERTS_RESPONSE_CAPACITY>::new();
            response.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
            response.push(FloatOutBoyAppDataCommand::AlertsList.id());
            response.push_u32(
                self.alert_tracker
                    .active_alerts()
                    .active_alert_mask_compat(),
            );
            response.push_u32(0);
            let fault = self.alert_tracker.firmware_fault_code();
            response.push(fault.wire_code());
            push_fault_name(&mut response, telemetry, fault);
            let count_index = response.len();
            response.push(0);
            let mut count = 0_u8;
            self.alert_tracker.for_each_record_since(since, |record| {
                if response.remaining() < 58 {
                    return false;
                }
                response.push_u32(record.timestamp.as_ticks());
                response.push(record.id.id());
                response.push(u8::from(record.active));
                response.push(record.code.wire_code());
                push_fault_name(&mut response, telemetry, record.code);
                count = count.saturating_add(1);
                true
            });
            response.set(count_index, count);
            return reply(response.as_bytes());
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

fn push_fault_name<const N: usize>(
    packet: &mut FloatOutBoyPacket<N>,
    telemetry: &impl MotorTelemetry,
    code: FirmwareFaultWireCode,
) {
    if code.wire_code() == 0 {
        return;
    }

    let name = bounded_fault_name(
        telemetry
            .firmware_fault_description_for(code)
            .unwrap_or_default()
            .as_bytes(),
    );
    packet.push(u8::try_from(name.len()).unwrap_or(u8::MAX));
    packet.extend(name);
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
mod tests;
