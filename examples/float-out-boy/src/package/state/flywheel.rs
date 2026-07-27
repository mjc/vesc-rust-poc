use super::{
    FloatOutBoyAppDataCommand, FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount, FloatOutBoyMode,
    FloatOutBoyPackageState, FloatOutBoyRunState, LoopConfig, float_out_boy_command_payload,
};
use crate::config::FloatOutBoyFlywheelConfig;
use vescpkg_rs::WireByte;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, RateCurrentGain, Ratio,
};

const FLYWHEEL_COMMAND_ARMED: u8 = 0x80;
const FLYWHEEL_COMMAND_MASK: u8 = 0x7f;
const FLYWHEEL_RECALIBRATE: u8 = 2;
const FLYWHEEL_RELAX_ROLL: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FloatOutBoyFlywheelOffsets {
    pitch: AngleDegrees,
    roll: AngleDegrees,
}

impl FloatOutBoyFlywheelOffsets {
    pub(super) const fn source_startup() -> Self {
        Self {
            pitch: AngleDegrees::ZERO,
            roll: AngleDegrees::ZERO,
        }
    }

    fn calibrated(pitch: AngleDegrees, roll: AngleDegrees) -> Self {
        Self { pitch, roll }
    }

    fn needs_calibration(self) -> bool {
        self.pitch.is_zero()
    }

    fn apply(
        self,
        mode: FloatOutBoyMode,
        pitch: AngleDegrees,
        roll: AngleDegrees,
    ) -> (AngleDegrees, AngleDegrees) {
        if !matches!(mode, FloatOutBoyMode::Flywheel) {
            return (pitch, roll);
        }

        let pitch = self.pitch - pitch;
        let roll = roll - self.roll;
        let roll = if roll < AngleDegrees::from_degrees(-200.0) {
            roll + AngleDegrees::from_degrees(360.0)
        } else if roll > AngleDegrees::from_degrees(200.0) {
            roll - AngleDegrees::from_degrees(360.0)
        } else {
            roll
        };
        (pitch, roll)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FloatOutBoyFlywheelRuntime {
    offsets: FloatOutBoyFlywheelOffsets,
    config: Option<FloatOutBoyFlywheelConfig>,
    abort: bool,
}

impl FloatOutBoyFlywheelRuntime {
    pub(super) const fn source_startup() -> Self {
        Self {
            offsets: FloatOutBoyFlywheelOffsets::source_startup(),
            config: None,
            abort: false,
        }
    }

    fn needs_calibration(self) -> bool {
        self.offsets.needs_calibration()
    }

    const fn is_active(self) -> bool {
        self.config.is_some()
    }

    fn calibrate(&mut self, pitch: AngleDegrees, roll: AngleDegrees) {
        self.offsets = FloatOutBoyFlywheelOffsets::calibrated(pitch, roll);
    }

    fn activate(&mut self, config: FloatOutBoyFlywheelConfig) {
        self.config = Some(config);
        self.abort = false;
    }

    fn deactivate(&mut self) {
        self.config = None;
        self.abort = false;
    }

    pub(super) fn latch_abort(&mut self, abort: bool) {
        self.abort |= abort;
    }

    pub(super) const fn should_stop(self, both_footpads: bool) -> bool {
        self.abort || both_footpads
    }

    const fn config(self) -> Option<FloatOutBoyFlywheelConfig> {
        self.config
    }

    fn apply_attitude(
        self,
        mode: FloatOutBoyMode,
        pitch: AngleDegrees,
        roll: AngleDegrees,
    ) -> (AngleDegrees, AngleDegrees) {
        self.offsets.apply(mode, pitch, roll)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatOutBoyFlywheelStart {
    recalibrate: bool,
    config: FloatOutBoyFlywheelConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FloatOutBoyFlywheelRequest {
    Stop,
    Start(FloatOutBoyFlywheelStart),
}

impl FloatOutBoyFlywheelRequest {
    fn from_packet(bytes: &[u8]) -> Option<Self> {
        let [
            command,
            kp,
            kp2,
            duty_angle,
            duty_threshold,
            _allow_abort,
            optional @ ..,
        ] = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::Flywheel)?
        else {
            return None;
        };
        if command & FLYWHEEL_COMMAND_ARMED == 0 {
            return None;
        }

        let command = command & FLYWHEEL_COMMAND_MASK;
        if command == 0 {
            return Some(Self::Stop);
        }

        let duty_speed = optional
            .first()
            .copied()
            .filter(|speed| (2..100).contains(speed))
            .map_or_else(
                || AngularVelocity::from_degrees_per_second(5.0),
                |speed| {
                    WireByte::new(speed).scaled(0.5, 0.0, AngularVelocity::from_degrees_per_second)
                },
            );
        Some(Self::Start(FloatOutBoyFlywheelStart {
            recalibrate: command == FLYWHEEL_RECALIBRATE,
            config: FloatOutBoyFlywheelConfig {
                kp: if *kp == 0 {
                    AngleCurrentGain::new(8.0)
                } else {
                    WireByte::new(*kp).scaled(0.1, 0.0, AngleCurrentGain::new)
                },
                kp2: if *kp2 == 0 {
                    RateCurrentGain::new(0.3)
                } else {
                    WireByte::new(*kp2).scaled(0.01, 0.0, RateCurrentGain::new)
                },
                duty_angle: if *duty_angle == 0 {
                    AngleDegrees::from_degrees(2.0)
                } else {
                    WireByte::new(*duty_angle).scaled(0.1, 0.0, AngleDegrees::from_degrees)
                },
                duty_threshold: if *duty_threshold == 0 {
                    Ratio::from_ratio_const(0.1)
                } else {
                    WireByte::new(*duty_threshold).scaled(0.01, 0.0, Ratio::from_ratio_const)
                },
                duty_speed,
                relaxed_roll: command & FLYWHEEL_RELAX_ROLL != 0,
            },
        }))
    }

    fn apply_to(self, state: &mut FloatOutBoyPackageState) {
        let ride_state = state.all_data_payloads.base().status().ride_state();
        if !matches!(
            ride_state.mode(),
            FloatOutBoyMode::Normal | FloatOutBoyMode::Flywheel
        ) || !matches!(ride_state.run_state(), FloatOutBoyRunState::Ready)
            && !matches!(ride_state.mode(), FloatOutBoyMode::Flywheel)
        {
            return;
        }

        match self {
            Self::Stop => state.stop_flywheel(),
            Self::Start(start) => state.start_flywheel(start),
        }
    }
}

impl FloatOutBoyPackageState {
    pub(super) fn handle_flywheel_packet(&mut self, bytes: &[u8]) -> bool {
        FloatOutBoyFlywheelRequest::from_packet(bytes).is_some_and(|request| {
            request.apply_to(self);
            true
        })
    }

    fn accept_flywheel_calibration(&mut self, recalibrate: bool) -> bool {
        if self.flywheel.needs_calibration() || recalibrate {
            let attitude = self.all_data_payloads.base().attitude();
            let pitch = AngleDegrees::from(attitude.pitch().angle());
            if pitch.abs() < AngleDegrees::from_degrees(70.0) {
                if self.flywheel.is_active() {
                    self.restore_flywheel_config();
                } else {
                    self.set_ride_mode(FloatOutBoyMode::Normal);
                }
                return false;
            }
            self.flywheel
                .calibrate(pitch, AngleDegrees::from(attitude.roll().angle()));
            self.alert_beeper(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::ONE));
        } else {
            self.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
        }
        true
    }

    fn start_flywheel(&mut self, start: FloatOutBoyFlywheelStart) {
        self.set_ride_mode(FloatOutBoyMode::Flywheel);
        if !self.accept_flywheel_calibration(start.recalibrate) {
            return;
        }
        let mut config = self.serialized_config;
        let updated = config.editor().apply_flywheel_overrides(start.config);
        if !updated {
            // A failed write means the in-memory configuration layout is not the
            // layout this package was built for. Reload the saved configuration
            // instead of running with a mixture of old and partially written values.
            self.restore_flywheel_config();
            return;
        }
        self.replace_active_config(&config);
        self.flywheel.activate(start.config);
    }

    pub(super) fn stop_flywheel(&mut self) {
        self.restore_flywheel_config();
    }

    pub(super) fn restore_flywheel_config(&mut self) {
        self.force_beeper_on();
        self.set_ride_mode(FloatOutBoyMode::Normal);
        self.flywheel.deactivate();
        self.read_config_from_eeprom();
        self.refresh_idle_epoch(vescpkg_rs::FirmwareClock::current_timestamp());
    }

    pub(super) fn runtime_duty_pushback_threshold(&self) -> Ratio {
        self.flywheel.config().map_or_else(
            || self.serialized_config.duty_pushback_threshold(),
            |config| config.duty_threshold,
        )
    }

    pub(super) fn runtime_duty_pushback_angle(&self) -> AngleDegrees {
        self.flywheel.config().map_or_else(
            || self.serialized_config.duty_pushback_angle(),
            |config| config.duty_angle,
        )
    }

    pub(super) fn runtime_duty_pushback_step(&self) -> AngleDegrees {
        let speed = self.flywheel.config().map_or_else(
            || self.serialized_config.duty_pushback_speed(),
            |config| config.duty_speed,
        );
        self.runtime_setpoint_step(speed)
    }

    pub(super) fn runtime_tiltback_return_step(&self) -> AngleDegrees {
        let speed = self.flywheel.config().map_or_else(
            || self.serialized_config.tiltback_return_speed(),
            |config| config.duty_speed,
        );
        self.runtime_setpoint_step(speed)
    }

    fn runtime_setpoint_step(&self, speed: AngularVelocity) -> AngleDegrees {
        self.serialized_config
            .startup()
            .sample_rate()
            .sample_period()
            .map_or(AngleDegrees::ZERO, |period| {
                AngleDegrees::from(speed * period)
            })
    }

    pub(super) fn runtime_balance_loop_config(&self) -> LoopConfig {
        let mut config = self.serialized_config.balance_loop_config();
        if let Some(flywheel) = self.flywheel.config() {
            config.kp = flywheel.kp;
            config.kp2 = flywheel.kp2;
        }
        config
    }

    pub(super) fn flywheel_attitude(
        &self,
        mode: FloatOutBoyMode,
        pitch: AngleDegrees,
        roll: AngleDegrees,
    ) -> (AngleDegrees, AngleDegrees) {
        self.flywheel.apply_attitude(mode, pitch, roll)
    }
}
