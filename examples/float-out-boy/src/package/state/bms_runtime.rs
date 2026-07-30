use crate::bms::{
    FloatOutBoyBmsFaults, FloatOutBoyBmsSample, FloatOutBoyBmsStartupGrace,
    FloatOutBoyBmsThresholds,
};
use vescpkg_rs::{TimestampTicks, VescSeconds, WrappingTimer, timer_older};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BmsReadyAlertFault {
    Connection,
    CellBalance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BmsRuntimeState {
    sample: FloatOutBoyBmsSample,
    faults: FloatOutBoyBmsFaults,
    start_ticks: Option<TimestampTicks>,
    alert_ticks: WrappingTimer,
}

impl Default for BmsRuntimeState {
    fn default() -> Self {
        Self {
            sample: FloatOutBoyBmsSample::default(),
            faults: FloatOutBoyBmsFaults::empty(),
            start_ticks: None,
            alert_ticks: WrappingTimer::started_at(TimestampTicks::from_ticks(0)),
        }
    }
}

impl BmsRuntimeState {
    pub(super) fn record_sample(&mut self, sample: FloatOutBoyBmsSample) {
        self.sample = sample;
    }

    pub(super) fn initialize_start_epoch(&mut self, now: TimestampTicks) {
        self.start_ticks = Some(now);
    }

    pub(super) fn refresh(
        &mut self,
        enabled: bool,
        thresholds: FloatOutBoyBmsThresholds,
        system_time_ticks: TimestampTicks,
    ) {
        let start_ticks = *self.start_ticks.get_or_insert(system_time_ticks);
        let startup_timeout_elapsed = timer_older(
            system_time_ticks,
            start_ticks,
            VescSeconds::from_seconds(5.0),
        );
        self.faults = FloatOutBoyBmsFaults::evaluate(
            enabled,
            self.sample,
            thresholds,
            FloatOutBoyBmsStartupGrace::from_elapsed(startup_timeout_elapsed),
        );
    }

    pub(super) fn take_ready_alert_fault(
        &mut self,
        system_time_ticks: TimestampTicks,
        disengage_ticks: WrappingTimer,
    ) -> Option<BmsReadyAlertFault> {
        let connection = self.faults.contains(FloatOutBoyBmsFaults::CONNECTION);
        let balance = self.faults.contains(FloatOutBoyBmsFaults::CELL_BALANCE)
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

    pub(super) const fn contains(&self, fault: FloatOutBoyBmsFaults) -> bool {
        self.faults.contains(fault)
    }

    #[cfg(test)]
    pub(super) const fn sample(self) -> FloatOutBoyBmsSample {
        self.sample
    }

    #[cfg(test)]
    pub(super) const fn faults(self) -> FloatOutBoyBmsFaults {
        self.faults
    }
}
