use super::*;
use std::vec::Vec;
use vescpkg_rs::prelude::{FirmwareFault, FirmwareFaultWireCode, TimestampTicks};

fn fault(code: u8) -> FirmwareFault {
    match code {
        0 => FirmwareFault::None,
        5 => FirmwareFault::Active(vescpkg_rs::prelude::FirmwareFaultId::AbsoluteOverCurrent),
        6 => FirmwareFault::Active(vescpkg_rs::prelude::FirmwareFaultId::OverTemperatureMotor),
        _ => FirmwareFault::Unknown,
    }
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

    tracker.update(fault(5), TimestampTicks::from_ticks(1), true);
    tracker.update(fault(5), TimestampTicks::from_ticks(2), true);
    tracker.update(fault(6), TimestampTicks::from_ticks(3), true);
    tracker.update(fault(0), TimestampTicks::from_ticks(4), true);

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

    tracker.update(fault(5), TimestampTicks::from_ticks(1), true);
    tracker.update(fault(0), TimestampTicks::from_ticks(2), true);
    assert!(tracker.fatal_error());

    tracker.clear_fatal();
    assert!(!tracker.fatal_error());

    tracker.update(fault(5), TimestampTicks::from_ticks(3), false);
    tracker.update(fault(0), TimestampTicks::from_ticks(4), false);
    assert!(!tracker.fatal_error());
}

#[test]
fn record_query_is_strictly_newer_and_keeps_the_latest_twenty() {
    let mut tracker = AlertTrackerState::default();
    for tick in 1..=21 {
        let code = if tick % 2 == 0 { 5 } else { 6 };
        tracker.update(fault(code), TimestampTicks::from_ticks(tick), true);
    }

    let records = records_since(&tracker, TimestampTicks::from_ticks(1));
    assert_eq!(records.len(), ALERT_RECORD_CAPACITY);
    assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(2));
    assert_eq!(records[19].timestamp, TimestampTicks::from_ticks(21));

    let records = records_since(&tracker, TimestampTicks::from_ticks(20));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].timestamp, TimestampTicks::from_ticks(21));
}
