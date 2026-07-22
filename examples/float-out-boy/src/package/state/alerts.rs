use super::super::protocol::wire::{
    float_out_boy_realtime_push_u8, float_out_boy_realtime_push_u32,
};
use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::domain::{FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FloatOutBoyAppDataCommand};
use vescpkg_rs::MotorTelemetry;
use vescpkg_rs::prelude::{FirmwareFaultCode, FirmwareFaultWireCode, TimestampTicks};

const ALERTS_RESPONSE_CAPACITY: usize = 511;

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
                if index + 58 > response.len() {
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
                count += 1;
                true
            });
            response[count_index] = count;
            return send(&response[..index]);
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

    let name = telemetry
        .firmware_fault_name(FirmwareFaultCode::from_wire_code(code.wire_code()))
        .unwrap_or_default();
    let name = &name[..name.len().min(50)];
    float_out_boy_realtime_push_u8(buffer, index, name.len() as u8);
    for byte in name {
        float_out_boy_realtime_push_u8(buffer, index, *byte);
    }
}
