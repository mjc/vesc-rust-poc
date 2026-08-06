use core::ops::{Deref, DerefMut};

use crate::bms::FloatOutBoyBmsFaults;
use vescpkg_rs::{BmsMonitor, TimestampTicks, WrappingTimer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BmsReadyAlertFault {
    Connection,
    CellBalance,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct BmsRuntimeState {
    monitor: BmsMonitor,
    alert_ticks: WrappingTimer,
}

impl Deref for BmsRuntimeState {
    type Target = BmsMonitor;

    fn deref(&self) -> &Self::Target {
        &self.monitor
    }
}

impl DerefMut for BmsRuntimeState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.monitor
    }
}

impl BmsRuntimeState {
    pub(super) fn take_ready_alert_fault(
        &mut self,
        system_time_ticks: TimestampTicks,
        disengage_ticks: WrappingTimer,
    ) -> Option<BmsReadyAlertFault> {
        let connection = self.faults().contains(FloatOutBoyBmsFaults::CONNECTION);
        let balance = self.faults().contains(FloatOutBoyBmsFaults::CELL_BALANCE)
            && disengage_ticks.older_than_secs(system_time_ticks, 5);
        ((connection || balance) && self.alert_ticks.older_than_secs(system_time_ticks, 15)).then(
            || {
                self.alert_ticks.restart(system_time_ticks);
                if connection {
                    BmsReadyAlertFault::Connection
                } else {
                    BmsReadyAlertFault::CellBalance
                }
            },
        )
    }
}
