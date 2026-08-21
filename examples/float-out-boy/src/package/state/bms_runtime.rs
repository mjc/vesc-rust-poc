use crate::bms::{
    FloatOutBoyBmsConnectionMonitoring, FloatOutBoyBmsFault, FloatOutBoyBmsFaults,
    FloatOutBoyBmsIntegration, FloatOutBoyBmsSample,
};
use vescpkg_rs::{TimestampTicks, VescSeconds};
use vescpkg_rs::{
    timer_older as float_out_boy_ticks_elapsed_seconds,
    timer_older_whole_seconds as float_out_boy_ticks_elapsed,
};

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
    alert_ticks: TimestampTicks,
}

impl Default for BmsRuntimeState {
    fn default() -> Self {
        Self {
            sample: FloatOutBoyBmsSample::default(),
            faults: FloatOutBoyBmsFaults::NONE,
            start_ticks: None,
            alert_ticks: TimestampTicks::from_ticks(0),
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
        integration: FloatOutBoyBmsIntegration,
        system_time_ticks: TimestampTicks,
    ) {
        let start_ticks = *self.start_ticks.get_or_insert(system_time_ticks);
        let connection_monitoring = if float_out_boy_ticks_elapsed_seconds(
            system_time_ticks,
            start_ticks,
            VescSeconds::from_seconds(5.0),
        ) {
            FloatOutBoyBmsConnectionMonitoring::Armed
        } else {
            FloatOutBoyBmsConnectionMonitoring::Deferred
        };
        self.faults =
            FloatOutBoyBmsFaults::evaluate(integration, self.sample, connection_monitoring);
    }

    pub(super) fn take_ready_alert_fault(
        &mut self,
        system_time_ticks: TimestampTicks,
        disengage_ticks: TimestampTicks,
    ) -> Option<BmsReadyAlertFault> {
        let connection = self.faults.contains(FloatOutBoyBmsFault::Connection);
        let balance = self.faults.contains(FloatOutBoyBmsFault::CellBalance)
            && float_out_boy_ticks_elapsed(system_time_ticks, disengage_ticks, 5);
        ((connection || balance)
            && float_out_boy_ticks_elapsed(system_time_ticks, self.alert_ticks, 15))
        .then(|| {
            self.alert_ticks = system_time_ticks;
            if connection {
                BmsReadyAlertFault::Connection
            } else {
                BmsReadyAlertFault::CellBalance
            }
        })
    }

    pub(super) const fn contains(&self, fault: FloatOutBoyBmsFault) -> bool {
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
