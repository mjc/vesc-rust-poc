use super::*;
use crate::config::RefloatFlywheelConfig;
use vescpkg_rs::WireByte;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, RateCurrentGain, Ratio,
};

const FLYWHEEL_COMMAND_ARMED: u8 = 0x80;
const FLYWHEEL_COMMAND_MASK: u8 = 0x7f;
const FLYWHEEL_RECALIBRATE: u8 = 2;
const FLYWHEEL_RELAX_ROLL: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RefloatFlywheelOffsets {
    pitch: AngleDegrees,
    roll: AngleDegrees,
}

impl RefloatFlywheelOffsets {
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
        mode: RefloatMode,
        pitch: AngleDegrees,
        roll: AngleDegrees,
    ) -> (AngleDegrees, AngleDegrees) {
        if !matches!(mode, RefloatMode::Flywheel) {
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
struct RefloatFlywheelStart {
    recalibrate: bool,
    config: RefloatFlywheelConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RefloatFlywheelRequest {
    Stop,
    Start(RefloatFlywheelStart),
}

impl RefloatFlywheelRequest {
    fn from_packet(bytes: &[u8]) -> Option<Self> {
        let [
            command,
            kp,
            kp2,
            duty_angle,
            duty_threshold,
            _allow_abort,
            optional @ ..,
        ] = refloat_command_payload(bytes, RefloatAppDataCommand::Flywheel)?
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
        Some(Self::Start(RefloatFlywheelStart {
            recalibrate: command == FLYWHEEL_RECALIBRATE,
            config: RefloatFlywheelConfig {
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

    fn apply_to(self, state: &mut RefloatPackageState) {
        let ride_state = state.all_data_payloads.base().status().ride_state();
        if !matches!(
            ride_state.mode(),
            RefloatMode::Normal | RefloatMode::Flywheel
        ) || !matches!(ride_state.run_state(), RefloatRunState::Ready)
            && !matches!(ride_state.mode(), RefloatMode::Flywheel)
        {
            return;
        }

        match self {
            Self::Stop => state.stop_flywheel(),
            Self::Start(start) => state.start_flywheel(start),
        }
    }
}

impl RefloatPackageState {
    pub(super) fn handle_flywheel_packet(&mut self, bytes: &[u8]) -> bool {
        let Some(request) = RefloatFlywheelRequest::from_packet(bytes) else {
            return false;
        };
        request.apply_to(self);
        true
    }

    fn start_flywheel(&mut self, start: RefloatFlywheelStart) {
        self.set_ride_mode(RefloatMode::Flywheel);
        if self.flywheel_offsets.needs_calibration() || start.recalibrate {
            let attitude = self.all_data_payloads.base().attitude();
            let pitch = AngleDegrees::from(attitude.pitch().angle());
            if pitch.abs() < AngleDegrees::from_degrees(70.0) {
                self.set_ride_mode(RefloatMode::Normal);
                return;
            }
            self.flywheel_offsets = RefloatFlywheelOffsets::calibrated(
                pitch,
                AngleDegrees::from(attitude.roll().angle()),
            );
            self.alert_beeper(RefloatBeeperAlert::Long(RefloatBeeperCount::ONE));
        } else {
            self.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::THREE));
        }
        self.flywheel_abort = false;

        let updated = self
            .serialized_config
            .editor()
            .apply_flywheel_overrides(start.config);
        debug_assert!(updated);
    }

    pub(super) fn stop_flywheel(&mut self) {
        self.set_ride_mode(RefloatMode::Normal);
        self.restore_flywheel_config();
    }

    pub(super) fn restore_flywheel_config(&mut self) {
        self.read_config_from_eeprom();
        self.refresh_balance_filter_config();
        self.refresh_config_runtime_state();
        let run_state = self
            .all_data_payloads
            .base()
            .status()
            .ride_state()
            .run_state();
        self.alert_beeper(if matches!(run_state, RefloatRunState::Disabled) {
            RefloatBeeperAlert::Short(RefloatBeeperCount::THREE)
        } else {
            RefloatBeeperAlert::Short(RefloatBeeperCount::ONE)
        });
    }

    pub(super) fn flywheel_attitude(
        &self,
        mode: RefloatMode,
        pitch: AngleDegrees,
        roll: AngleDegrees,
    ) -> (AngleDegrees, AngleDegrees) {
        self.flywheel_offsets.apply(mode, pitch, roll)
    }
}
