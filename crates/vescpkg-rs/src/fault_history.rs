use crate::RingCursor;
use crate::prelude::{FirmwareFault, FirmwareFaultWireCode, TimestampTicks};

/// One firmware-fault transition retained in chronological history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareFaultRecord {
    /// Firmware timestamp at which the transition was observed.
    pub timestamp: TimestampTicks,
    /// Whether this record activates or clears a fault.
    pub active: bool,
    /// Firmware wire code, or zero for a clear transition.
    pub code: FirmwareFaultWireCode,
}

/// Current firmware fault state plus a bounded transition history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareFaultHistory<const N: usize> {
    active: bool,
    code: FirmwareFaultWireCode,
    fatal: bool,
    records: [Option<FirmwareFaultRecord>; N],
    ring: RingCursor,
}

impl<const N: usize> Default for FirmwareFaultHistory<N> {
    fn default() -> Self {
        Self {
            active: false,
            code: FirmwareFaultWireCode::from_wire_code(0),
            fatal: false,
            records: [None; N],
            ring: RingCursor::default(),
        }
    }
}

impl<const N: usize> FirmwareFaultHistory<N> {
    /// Observe one firmware fault and retain only transitions or code changes.
    pub fn update(
        &mut self,
        fault: FirmwareFault,
        timestamp: TimestampTicks,
        persistent_fatal: bool,
    ) {
        let (active, code) = match fault {
            FirmwareFault::None => (false, FirmwareFaultWireCode::from_wire_code(0)),
            FirmwareFault::Active(fault) => (true, fault.wire_code()),
            FirmwareFault::Unknown => (true, FirmwareFaultWireCode::from_wire_code(0)),
        };
        if active && (!self.active || code != self.code) {
            self.push(FirmwareFaultRecord {
                timestamp,
                active: true,
                code,
            });
        } else if self.active && !active {
            self.push(FirmwareFaultRecord {
                timestamp,
                active: false,
                code: FirmwareFaultWireCode::from_wire_code(0),
            });
        }

        self.active = active;
        self.code = if active {
            code
        } else {
            FirmwareFaultWireCode::from_wire_code(0)
        };
        self.fatal = active || (persistent_fatal && self.fatal);
    }

    /// Clear the retained fatal latch without changing current fault state.
    pub fn clear_fatal(&mut self) {
        self.fatal = false;
    }

    /// Return whether a firmware fault is currently active.
    #[must_use]
    pub const fn firmware_fault_active(&self) -> bool {
        self.active
    }

    /// Return the active firmware wire code, or zero when clear.
    #[must_use]
    pub const fn firmware_fault_code(&self) -> FirmwareFaultWireCode {
        self.code
    }

    /// Return whether the fatal latch is set.
    #[must_use]
    pub const fn fatal_error(&self) -> bool {
        self.fatal
    }

    /// Visit retained records newer than `since`, oldest first.
    pub fn for_each_record_since(
        &self,
        since: TimestampTicks,
        mut visit: impl FnMut(FirmwareFaultRecord) -> bool,
    ) {
        for logical_index in 0..self.ring.len(N) {
            let Some(record) = self
                .ring
                .slot_at(logical_index, N)
                .and_then(|slot| self.records.get(slot))
                .copied()
                .flatten()
            else {
                continue;
            };
            if record.timestamp > since && !visit(record) {
                break;
            }
        }
    }

    fn push(&mut self, record: FirmwareFaultRecord) {
        let Some(slot) = self.ring.write_slot(N) else {
            return;
        };
        if let Some(target) = self.records.get_mut(slot) {
            *target = Some(record);
            self.ring.commit_write(N);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FirmwareFaultHistory, FirmwareFaultRecord};
    use crate::prelude::{FirmwareFault, FirmwareFaultId, TimestampTicks};
    use std::vec::Vec;

    fn records<const N: usize>(history: &FirmwareFaultHistory<N>) -> Vec<FirmwareFaultRecord> {
        let mut records = Vec::new();
        history.for_each_record_since(TimestampTicks::from_ticks(0), |record| {
            records.push(record);
            true
        });
        records
    }

    #[test]
    fn records_transitions_and_code_changes_only() {
        let mut history = FirmwareFaultHistory::<4>::default();
        let first = FirmwareFault::Active(FirmwareFaultId::AbsoluteOverCurrent);
        let second = FirmwareFault::Active(FirmwareFaultId::OverTemperatureMotor);
        history.update(first, TimestampTicks::from_ticks(1), true);
        history.update(first, TimestampTicks::from_ticks(2), true);
        history.update(second, TimestampTicks::from_ticks(3), true);
        history.update(FirmwareFault::None, TimestampTicks::from_ticks(4), true);

        let records = records(&history);
        assert_eq!(records.len(), 3);
        assert!(records[0].active);
        assert_ne!(records[0].code, records[1].code);
        assert!(!records[2].active);
    }

    #[test]
    fn bounded_history_retains_latest_records_in_order() {
        let mut history = FirmwareFaultHistory::<2>::default();
        for (tick, fault) in [
            FirmwareFault::Active(FirmwareFaultId::AbsoluteOverCurrent),
            FirmwareFault::Active(FirmwareFaultId::OverTemperatureMotor),
            FirmwareFault::None,
        ]
        .into_iter()
        .enumerate()
        {
            let timestamp = u32::try_from(tick + 1).expect("three ticks fit");
            history.update(fault, TimestampTicks::from_ticks(timestamp), true);
        }
        let records = records(&history);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(2));
        assert_eq!(records[1].timestamp, TimestampTicks::from_ticks(3));
    }

    #[test]
    fn fatal_latch_obeys_persistence_and_explicit_clear() {
        let mut history = FirmwareFaultHistory::<0>::default();
        history.update(
            FirmwareFault::Active(FirmwareFaultId::AbsoluteOverCurrent),
            TimestampTicks::from_ticks(1),
            true,
        );
        history.update(FirmwareFault::None, TimestampTicks::from_ticks(2), true);
        assert!(history.fatal_error());
        history.clear_fatal();
        assert!(!history.fatal_error());
        assert!(!history.firmware_fault_active());
    }
}
