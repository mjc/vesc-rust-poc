use crate::domain::{FloatOutBoyAlertId, FloatOutBoyFatalErrorState, FloatOutBoyRealtimeAlertMask};
use vescpkg_rs::prelude::{FirmwareFault, FirmwareFaultWireCode, TimestampTicks};

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
        fault: FirmwareFault,
        timestamp: TimestampTicks,
        persistent_fatal_error: bool,
    ) {
        let (is_active, code) = match fault {
            FirmwareFault::None => (false, FirmwareFaultWireCode::from_wire_code(0)),
            FirmwareFault::Active(fault) => (true, fault.wire_code()),
            FirmwareFault::Unknown => (true, FirmwareFaultWireCode::from_wire_code(0)),
        };
        let was_active = self
            .active_alerts
            .contains(FloatOutBoyAlertId::FirmwareFault);
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
mod tests;
