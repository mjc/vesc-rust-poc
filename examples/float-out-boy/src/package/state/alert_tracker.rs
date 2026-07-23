#![cfg_attr(not(test), deny(clippy::arithmetic_side_effects))]

use crate::domain::{FloatOutBoyAlertId, FloatOutBoyFatalErrorState, FloatOutBoyRealtimeAlertMask};
use vescpkg_rs::prelude::{FirmwareFaultCode, FirmwareFaultWireCode, TimestampTicks};

const ALERT_RECORD_CAPACITY: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AlertRecord {
    pub(super) timestamp: TimestampTicks,
    pub(super) id: FloatOutBoyAlertId,
    pub(super) active: bool,
    pub(super) code: FirmwareFaultWireCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AlertTrackerState {
    active_alerts: FloatOutBoyRealtimeAlertMask,
    firmware_fault_code: FirmwareFaultWireCode,
    fatal_error: FloatOutBoyFatalErrorState,
    records: [Option<AlertRecord>; ALERT_RECORD_CAPACITY],
    next_record: usize,
    record_count: usize,
}

impl Default for AlertTrackerState {
    fn default() -> Self {
        Self {
            active_alerts: FloatOutBoyRealtimeAlertMask::empty(),
            firmware_fault_code: FirmwareFaultWireCode::from_wire_code(0),
            fatal_error: FloatOutBoyFatalErrorState::None,
            records: [None; ALERT_RECORD_CAPACITY],
            next_record: 0,
            record_count: 0,
        }
    }
}

impl AlertTrackerState {
    pub(super) fn update_firmware_fault(
        &mut self,
        fault: FirmwareFaultCode,
        timestamp: TimestampTicks,
        persistent_fatal_error: bool,
    ) {
        let code = FirmwareFaultWireCode::try_from(fault)
            .unwrap_or(FirmwareFaultWireCode::from_wire_code(0));
        let was_active = self
            .active_alerts
            .contains(FloatOutBoyAlertId::FirmwareFault);
        let is_active = !fault.is_none();

        if is_active && (!was_active || code != self.firmware_fault_code) {
            self.push_record(AlertRecord {
                timestamp,
                id: FloatOutBoyAlertId::FirmwareFault,
                active: true,
                code,
            });
        } else if was_active && !is_active {
            self.push_record(AlertRecord {
                timestamp,
                id: FloatOutBoyAlertId::FirmwareFault,
                active: false,
                code: FirmwareFaultWireCode::from_wire_code(0),
            });
        }

        self.active_alerts = if is_active {
            FloatOutBoyRealtimeAlertMask::empty().with_alert(FloatOutBoyAlertId::FirmwareFault)
        } else {
            FloatOutBoyRealtimeAlertMask::empty()
        };
        self.firmware_fault_code = if is_active {
            code
        } else {
            FirmwareFaultWireCode::from_wire_code(0)
        };
        self.fatal_error = match (is_active, persistent_fatal_error, self.fatal_error) {
            (true, _, _) | (false, true, FloatOutBoyFatalErrorState::Present) => {
                FloatOutBoyFatalErrorState::Present
            }
            _ => FloatOutBoyFatalErrorState::None,
        };
    }

    pub(super) fn clear_fatal(&mut self) {
        self.fatal_error = FloatOutBoyFatalErrorState::None;
    }

    pub(super) const fn active_alerts(&self) -> FloatOutBoyRealtimeAlertMask {
        self.active_alerts
    }

    pub(super) const fn firmware_fault_code(&self) -> FirmwareFaultWireCode {
        self.firmware_fault_code
    }

    pub(super) const fn fatal_error(&self) -> FloatOutBoyFatalErrorState {
        self.fatal_error
    }

    pub(super) fn for_each_record_since(
        &self,
        since: TimestampTicks,
        mut visit: impl FnMut(AlertRecord) -> bool,
    ) {
        let first = if self.record_count == ALERT_RECORD_CAPACITY {
            self.next_record
        } else {
            0
        };
        for offset in 0..self.record_count {
            let index = first.saturating_add(offset) % ALERT_RECORD_CAPACITY;
            if let Some(record) = self.records.get(index).copied().flatten()
                && record.timestamp > since
                && !visit(record)
            {
                break;
            }
        }
    }

    fn push_record(&mut self, record: AlertRecord) {
        if let Some(slot) = self.records.get_mut(self.next_record) {
            *slot = Some(record);
        }
        self.next_record = self.next_record.saturating_add(1) % ALERT_RECORD_CAPACITY;
        self.record_count = self
            .record_count
            .saturating_add(1)
            .min(ALERT_RECORD_CAPACITY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fault(code: u8) -> FirmwareFaultCode {
        FirmwareFaultCode::from_wire_code(code)
    }

    fn records_since(tracker: &AlertTrackerState, since: TimestampTicks) -> Vec<AlertRecord> {
        let mut records = Vec::new();
        tracker.for_each_record_since(since, |record| {
            records.push(record);
            true
        });
        records
    }

    #[test]
    fn firmware_fault_records_only_transitions_and_code_changes() {
        let mut tracker = AlertTrackerState::default();

        tracker.update_firmware_fault(fault(5), TimestampTicks::from_ticks(1), true);
        tracker.update_firmware_fault(fault(5), TimestampTicks::from_ticks(2), true);
        tracker.update_firmware_fault(fault(6), TimestampTicks::from_ticks(3), true);
        tracker.update_firmware_fault(fault(0), TimestampTicks::from_ticks(4), true);

        let records = records_since(&tracker, TimestampTicks::from_ticks(0));
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(1));
        assert_eq!(records[1].code, FirmwareFaultWireCode::from_wire_code(6));
        assert!(!records[2].active);
        assert_eq!(records[2].code, FirmwareFaultWireCode::from_wire_code(0));
    }

    #[test]
    fn persistent_fatal_survives_fault_clear_until_control_clear() {
        let mut tracker = AlertTrackerState::default();

        tracker.update_firmware_fault(fault(5), TimestampTicks::from_ticks(1), true);
        tracker.update_firmware_fault(fault(0), TimestampTicks::from_ticks(2), true);
        assert_eq!(tracker.fatal_error(), FloatOutBoyFatalErrorState::Present);

        tracker.clear_fatal();
        assert_eq!(tracker.fatal_error(), FloatOutBoyFatalErrorState::None);

        tracker.update_firmware_fault(fault(5), TimestampTicks::from_ticks(3), false);
        tracker.update_firmware_fault(fault(0), TimestampTicks::from_ticks(4), false);
        assert_eq!(tracker.fatal_error(), FloatOutBoyFatalErrorState::None);
    }

    #[test]
    fn record_query_is_strictly_newer_and_keeps_the_latest_twenty() {
        let mut tracker = AlertTrackerState::default();
        for tick in 1..=21 {
            let code = if tick % 2 == 0 { 5 } else { 6 };
            tracker.update_firmware_fault(fault(code), TimestampTicks::from_ticks(tick), true);
        }

        let records = records_since(&tracker, TimestampTicks::from_ticks(1));
        assert_eq!(records.len(), ALERT_RECORD_CAPACITY);
        assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(2));
        assert_eq!(records[19].timestamp, TimestampTicks::from_ticks(21));

        let records = records_since(&tracker, TimestampTicks::from_ticks(20));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(21));
    }
}
