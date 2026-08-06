//! Shared VESC package BMS monitoring state and threshold evaluation.

use crate::{TimestampTicks, VescSeconds, Voltage, timer_older};

/// Integer BMS temperature used by VESC package configuration and telemetry.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BmsTemperature(i32);

impl BmsTemperature {
    /// Build a BMS temperature from whole degrees Celsius.
    #[must_use]
    pub const fn from_degrees_celsius(degrees_celsius: i32) -> Self {
        Self(degrees_celsius)
    }

    /// Decode VESC's signed one-byte BMS temperature configuration.
    #[must_use]
    pub fn from_config_byte(encoded: u8) -> Self {
        Self(i32::from(i8::from_be_bytes([encoded])))
    }
}

crate::typed_fields! {
    /// Latest typed BMS telemetry used by the shared monitor.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BmsSample {
        cell_low_voltage: Voltage => cell_low_voltage,
        cell_high_voltage: Voltage => cell_high_voltage,
        cell_low_temperature: BmsTemperature => cell_low_temperature,
        cell_high_temperature: BmsTemperature => cell_high_temperature,
        bms_high_temperature: BmsTemperature => bms_high_temperature,
        message_age: VescSeconds => message_age,
    }
}

crate::typed_fields! {
    /// Typed BMS fault thresholds supplied by one package's configuration.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BmsThresholds {
        cell_low_voltage: Voltage => cell_low_voltage,
        cell_high_voltage: Voltage => cell_high_voltage,
        cell_balance_voltage: Voltage => cell_balance_voltage,
        cell_low_temperature: BmsTemperature => cell_low_temperature,
        cell_high_temperature: BmsTemperature => cell_high_temperature,
        bms_high_temperature: BmsTemperature => bms_high_temperature,
    }
}

impl Default for BmsSample {
    fn default() -> Self {
        Self::source_startup()
    }
}

impl BmsSample {
    /// Build the placeholder used by VESC's shared package BMS monitor.
    #[must_use]
    pub const fn source_startup() -> Self {
        Self::new(
            Voltage::ZERO,
            Voltage::ZERO,
            BmsTemperature::from_degrees_celsius(0),
            BmsTemperature::from_degrees_celsius(0),
            BmsTemperature::from_degrees_celsius(0),
            VescSeconds::from_seconds(42.0),
        )
    }

    /// Decode one finite numeric BMS telemetry sample.
    #[must_use]
    pub fn try_from_telemetry(
        cell_low_voltage: f32,
        cell_high_voltage: f32,
        cell_low_temperature: i32,
        cell_high_temperature: i32,
        bms_high_temperature: i32,
        message_age: f32,
    ) -> Option<Self> {
        [cell_low_voltage, cell_high_voltage, message_age]
            .into_iter()
            .all(f32::is_finite)
            .then(|| {
                Self::new(
                    Voltage::from_volts(cell_low_voltage),
                    Voltage::from_volts(cell_high_voltage),
                    BmsTemperature::from_degrees_celsius(cell_low_temperature),
                    BmsTemperature::from_degrees_celsius(cell_high_temperature),
                    BmsTemperature::from_degrees_celsius(bms_high_temperature),
                    VescSeconds::from_seconds(message_age),
                )
            })
    }
}

bitflags::bitflags! {
    /// Standard shared VESC package BMS fault mask.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct BmsFaults: u8 {
        /// BMS telemetry timed out after startup grace.
        const CONNECTION = 1 << 0;
        /// The BMS temperature exceeded its configured maximum.
        const BMS_OVER_TEMPERATURE = 1 << 1;
        /// The highest cell voltage exceeded its configured maximum.
        const CELL_OVER_VOLTAGE = 1 << 2;
        /// The lowest cell voltage fell below its configured minimum.
        const CELL_UNDER_VOLTAGE = 1 << 3;
        /// The highest cell temperature exceeded its configured maximum.
        const CELL_OVER_TEMPERATURE = 1 << 4;
        /// The lowest cell temperature fell below its configured minimum.
        const CELL_UNDER_TEMPERATURE = 1 << 5;
        /// The cell-voltage spread exceeded its configured maximum.
        const CELL_BALANCE = 1 << 6;
    }
}

/// Whether the shared BMS connection-fault startup grace has elapsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsStartupGrace {
    /// Suppress a stale-message connection fault during startup.
    Active,
    /// Report a stale-message connection fault.
    Elapsed,
}

impl BmsStartupGrace {
    /// Select startup-grace state from the package timer result.
    #[must_use]
    pub const fn from_elapsed(elapsed: bool) -> Self {
        if elapsed { Self::Elapsed } else { Self::Active }
    }
}

impl BmsFaults {
    /// Evaluate the shared VESC package BMS threshold and timeout policy.
    #[must_use]
    pub fn evaluate(
        enabled: bool,
        sample: BmsSample,
        thresholds: BmsThresholds,
        startup_grace: BmsStartupGrace,
    ) -> Self {
        if !enabled {
            return Self::empty();
        }
        if sample.message_age() > VescSeconds::from_seconds(5.0) {
            return match startup_grace {
                BmsStartupGrace::Active => Self::empty(),
                BmsStartupGrace::Elapsed => Self::CONNECTION,
            };
        }

        let mut faults = Self::empty();
        faults.set(
            Self::CELL_UNDER_VOLTAGE,
            sample.cell_low_voltage() < thresholds.cell_low_voltage(),
        );
        faults.set(
            Self::CELL_OVER_VOLTAGE,
            sample.cell_high_voltage() > thresholds.cell_high_voltage(),
        );
        let temperatures_enabled = thresholds.cell_high_temperature() > BmsTemperature::default();
        faults.set(
            Self::CELL_OVER_TEMPERATURE,
            temperatures_enabled
                && sample.cell_high_temperature() > thresholds.cell_high_temperature(),
        );
        faults.set(
            Self::CELL_UNDER_TEMPERATURE,
            temperatures_enabled
                && sample.cell_low_temperature() < thresholds.cell_low_temperature(),
        );
        faults.set(
            Self::BMS_OVER_TEMPERATURE,
            thresholds.bms_high_temperature() > BmsTemperature::default()
                && sample.bms_high_temperature() > thresholds.bms_high_temperature(),
        );
        faults.set(
            Self::CELL_BALANCE,
            (sample.cell_low_voltage() - sample.cell_high_voltage()).abs()
                > thresholds.cell_balance_voltage(),
        );
        faults
    }
}

/// Shared latest-sample, fault-mask, and startup-grace BMS monitor state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BmsMonitor {
    sample: BmsSample,
    faults: BmsFaults,
    start_ticks: Option<TimestampTicks>,
    last_push_ticks: Option<TimestampTicks>,
}

impl Default for BmsMonitor {
    fn default() -> Self {
        Self {
            sample: BmsSample::default(),
            faults: BmsFaults::empty(),
            start_ticks: None,
            last_push_ticks: None,
        }
    }
}

impl BmsMonitor {
    /// Atomically replace the latest BMS sample.
    pub fn record_sample(&mut self, sample: BmsSample) {
        self.sample = sample;
        self.last_push_ticks = None;
    }

    /// Set the package startup epoch used for connection-fault grace.
    pub fn initialize_start_epoch(&mut self, now: TimestampTicks) {
        self.start_ticks = Some(now);
    }

    /// Refresh the fault mask from the latest sample and thresholds.
    pub fn refresh(&mut self, enabled: bool, thresholds: BmsThresholds, now: TimestampTicks) {
        let last_push_ticks = *self.last_push_ticks.get_or_insert(now);
        let message_age = self.sample.message_age()
            + TimestampTicks::from_ticks(
                now.wrapping_duration_since(last_push_ticks).as_ticks(),
            )
            .as_vesc_seconds();
        let sample = self.sample.with_message_age(message_age);
        let start_ticks = *self.start_ticks.get_or_insert(now);
        let startup_timeout_elapsed = timer_older(now, start_ticks, VescSeconds::from_seconds(5.0));
        self.faults = BmsFaults::evaluate(
            enabled,
            sample,
            thresholds,
            BmsStartupGrace::from_elapsed(startup_timeout_elapsed),
        );
    }

    /// Return the latest complete BMS sample.
    #[must_use]
    pub const fn sample(self) -> BmsSample {
        self.sample
    }

    /// Return the current evaluated BMS fault mask.
    #[must_use]
    pub const fn faults(self) -> BmsFaults {
        self.faults
    }
}

#[cfg(test)]
mod tests {
    use super::{BmsFaults, BmsMonitor, BmsSample, BmsStartupGrace, BmsTemperature, BmsThresholds};
    use crate::{TimestampTicks, Voltage};

    #[test]
    fn shared_bms_monitor_preserves_threshold_and_startup_grace_policy() {
        let thresholds = BmsThresholds::new(
            Voltage::from_volts(3.0),
            Voltage::from_volts(4.2),
            Voltage::from_volts(0.2),
            BmsTemperature::from_degrees_celsius(5),
            BmsTemperature::from_degrees_celsius(50),
            BmsTemperature::from_degrees_celsius(60),
        );
        let sample =
            BmsSample::try_from_telemetry(2.9, 4.3, 4, 51, 61, 0.0).expect("finite BMS sample");
        assert_eq!(
            BmsFaults::evaluate(true, sample, thresholds, BmsStartupGrace::Elapsed),
            BmsFaults::CELL_UNDER_VOLTAGE
                | BmsFaults::CELL_OVER_VOLTAGE
                | BmsFaults::CELL_UNDER_TEMPERATURE
                | BmsFaults::CELL_OVER_TEMPERATURE
                | BmsFaults::BMS_OVER_TEMPERATURE
                | BmsFaults::CELL_BALANCE
        );

        let stale =
            BmsSample::try_from_telemetry(2.9, 4.3, 4, 51, 61, 5.1).expect("finite BMS sample");
        assert_eq!(
            BmsFaults::evaluate(true, stale, thresholds, BmsStartupGrace::Active),
            BmsFaults::empty()
        );
        assert_eq!(
            BmsFaults::evaluate(true, stale, thresholds, BmsStartupGrace::Elapsed),
            BmsFaults::CONNECTION
        );
        assert_eq!(
            BmsFaults::evaluate(false, sample, thresholds, BmsStartupGrace::Elapsed),
            BmsFaults::empty()
        );
    }

    #[test]
    fn shared_bms_monitor_owns_sample_faults_and_strict_startup_timeout() {
        let thresholds = BmsThresholds::new(
            Voltage::from_volts(3.0),
            Voltage::from_volts(4.2),
            Voltage::from_volts(0.2),
            BmsTemperature::default(),
            BmsTemperature::default(),
            BmsTemperature::default(),
        );
        let mut monitor = BmsMonitor::default();
        monitor.initialize_start_epoch(TimestampTicks::from_ticks(100));
        monitor.refresh(true, thresholds, TimestampTicks::from_ticks(50_100));
        assert_eq!(monitor.faults(), BmsFaults::empty());
        monitor.refresh(true, thresholds, TimestampTicks::from_ticks(50_101));
        assert_eq!(monitor.faults(), BmsFaults::CONNECTION);

        let sample =
            BmsSample::try_from_telemetry(3.9, 4.0, 20, 30, 35, 0.0).expect("finite BMS sample");
        monitor.record_sample(sample);
        monitor.refresh(true, thresholds, TimestampTicks::from_ticks(50_102));
        assert_eq!(monitor.sample(), sample);
        assert_eq!(monitor.faults(), BmsFaults::empty());
    }
}
