//! Refloat VESC package payload.
//!
//! This crate owns Refloat-specific ride state, balancing, command, and app-data
//! semantics for the Rust port. Generic loader, lifecycle, firmware, units, and
//! semantic wrapper code lives in `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

pub mod domain;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::domain::{
        FootpadSensorSample, FootpadSensorState, REFLOAT_APP_DATA_PACKAGE_ID,
        REFLOAT_REALTIME_DATA_ITEMS, REFLOAT_REALTIME_RECORDED_ITEMS,
        REFLOAT_REALTIME_RUNTIME_ITEMS, RefloatAppDataCommand, RefloatBeepReason,
        RefloatChargingState, RefloatDarkRideState, RefloatDataRecorderFlags,
        RefloatFatalErrorState, RefloatHardwareConfig, RefloatHardwareLedsConfig, RefloatImuSample,
        RefloatLedAnimationMode, RefloatLedAnimationSpeed, RefloatLedBarConfig, RefloatLedColor,
        RefloatLedColorOrder, RefloatLedMode, RefloatLedPin, RefloatLedPinConfig,
        RefloatLedStripConfig, RefloatLedStripOrder, RefloatLedTransition, RefloatLedsConfig,
        RefloatMode, RefloatMotorCommand, RefloatMotorTelemetry, RefloatRealtimeDataHeader,
        RefloatRealtimeDataItem, RefloatRealtimeDataItemGroup, RefloatRealtimeDataRecordPolicy,
        RefloatRideState, RefloatRunState, RefloatSetpointAdjustment, RefloatStatusBarConfig,
        RefloatStatusBarIdleTimeout, RefloatStopCondition, RefloatWheelSlipState,
    };
    use vescpkg_rs::prelude::*;

    #[test]
    fn package_author_models_refloat_ride_inputs_without_raw_float_handoff() {
        let footpad = FootpadSensorSample::new(
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.65)),
            AdcDecodedLevel::new(Ratio::from_ratio_const(0.72)),
            FootpadSensorState::Both,
        );
        let imu = RefloatImuSample::new(
            ImuPitch::new(AngleRadians::from_radians(0.03)),
            ImuRoll::new(AngleRadians::from_radians(-0.01)),
            ImuYaw::new(AngleRadians::from_radians(1.25)),
            ImuAngularRate::new([
                AngularVelocity::from_degrees_per_second(12.0),
                AngularVelocity::from_degrees_per_second(0.0),
                AngularVelocity::from_degrees_per_second(-1.0),
            ]),
        );
        let telemetry = RefloatMotorTelemetry::new(
            ElectricalSpeed::new(Rpm::from_revolutions_per_minute(2400.0)),
            VehicleSpeed::new(Speed::from_meters_per_second(4.5)),
            DirectionalMotorCurrent::new(Current::from_amps(8.0)),
            BatteryCurrent::new(Current::from_amps(3.0)),
            DutyCycle::new(SignedRatio::from_ratio_const(0.18)),
            BatteryVoltage::new(Voltage::from_volts(74.0)),
        );

        assert_eq!(footpad.state(), FootpadSensorState::Both);
        assert_eq!(imu.pitch().angle().as_radians(), 0.03);
        assert_eq!(
            telemetry
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute(),
            2400.0
        );
        assert_eq!(telemetry.motor_current().current().as_amps(), 8.0);
        assert_eq!(telemetry.battery_current().current().as_amps(), 3.0);
    }

    #[test]
    fn package_author_requests_refloat_motor_current_with_domain_intent() {
        fn apply_requested_current(command: RefloatMotorCommand) -> MotorCurrent {
            command.requested_current()
        }

        let command = RefloatMotorCommand::new(MotorCurrent::new(Current::from_amps(11.0)));

        assert_eq!(apply_requested_current(command).current().as_amps(), 11.0);
    }

    #[test]
    fn package_author_reads_refloat_state_as_enums_not_bool_or_integer_flags() {
        let ready_pitch_fault = RefloatRideState::new(
            RefloatRunState::Ready,
            RefloatMode::Normal,
            RefloatSetpointAdjustment::None,
            RefloatStopCondition::Pitch,
        );
        let running_tiltback = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::PushbackHighVoltage,
            RefloatStopCondition::None,
        );

        assert_eq!(ready_pitch_fault.float_state_compat(), 6);
        assert_eq!(running_tiltback.float_state_compat(), 2);
        assert_eq!(running_tiltback.setpoint_adjustment_compat(), 4);
        assert_eq!(
            running_tiltback
                .with_wheelslip(RefloatWheelSlipState::Detected)
                .float_state_compat(),
            2
        );
        assert_eq!(
            running_tiltback
                .with_charging(RefloatChargingState::Charging)
                .with_wheelslip(RefloatWheelSlipState::Detected)
                .float_state_compat(),
            14
        );
    }

    #[test]
    fn package_author_parses_refloat_app_data_commands_as_domain_enum() {
        let commands = [
            (0, RefloatAppDataCommand::Info),
            (1, RefloatAppDataCommand::GetRealtimeData),
            (2, RefloatAppDataCommand::RuntimeTune),
            (3, RefloatAppDataCommand::TuneDefaults),
            (4, RefloatAppDataCommand::ConfigSave),
            (5, RefloatAppDataCommand::ConfigRestore),
            (6, RefloatAppDataCommand::TuneOther),
            (7, RefloatAppDataCommand::RcMove),
            (8, RefloatAppDataCommand::Booster),
            (9, RefloatAppDataCommand::PrintInfo),
            (10, RefloatAppDataCommand::GetAllData),
            (11, RefloatAppDataCommand::Experiment),
            (12, RefloatAppDataCommand::Lock),
            (13, RefloatAppDataCommand::HandTest),
            (14, RefloatAppDataCommand::TuneTilt),
            (20, RefloatAppDataCommand::LightsControl),
            (22, RefloatAppDataCommand::Flywheel),
            (24, RefloatAppDataCommand::LcmPoll),
            (25, RefloatAppDataCommand::LcmLightInfo),
            (26, RefloatAppDataCommand::LcmLightControl),
            (27, RefloatAppDataCommand::LcmDeviceInfo),
            (28, RefloatAppDataCommand::ChargingState),
            (29, RefloatAppDataCommand::LcmGetBattery),
            (31, RefloatAppDataCommand::RealtimeData),
            (32, RefloatAppDataCommand::RealtimeDataIds),
            (35, RefloatAppDataCommand::AlertsList),
            (36, RefloatAppDataCommand::AlertsControl),
            (41, RefloatAppDataCommand::DataRecordRequest),
            (99, RefloatAppDataCommand::LcmDebug),
        ];

        assert_eq!(REFLOAT_APP_DATA_PACKAGE_ID.get(), 101);
        assert!(commands.into_iter().all(|(id, command)| {
            RefloatAppDataCommand::try_from_id(id)
                .is_ok_and(|parsed| parsed == command && parsed.id() == id)
        }));
        assert_eq!(
            RefloatAppDataCommand::try_from_id(200)
                .expect_err("unstable command should stay explicit")
                .value(),
            200
        );
    }

    #[test]
    fn package_author_builds_realtime_data_header_without_raw_bit_flags() {
        let ride_state = RefloatRideState::new(
            RefloatRunState::Running,
            RefloatMode::Flywheel,
            RefloatSetpointAdjustment::PushbackLowVoltage,
            RefloatStopCondition::QuickStop,
        )
        .with_charging(RefloatChargingState::Charging)
        .with_wheelslip(RefloatWheelSlipState::Detected)
        .with_darkride(RefloatDarkRideState::Active);
        let recorder = RefloatDataRecorderFlags::inactive()
            .with_recording()
            .with_autostop();
        let header = RefloatRealtimeDataHeader::new(
            SystemTimestamp::new(TimestampTicks::from_ticks(123_456)),
            ride_state,
            FootpadSensorState::Both,
            RefloatBeepReason::FirmwareFault,
        )
        .with_data_recorder(recorder)
        .with_fatal_error(RefloatFatalErrorState::Present);

        assert_eq!(header.timestamp().ticks().as_ticks(), 123_456);
        assert_eq!(header.data_mask_compat(), 0b0000_0111);
        assert_eq!(header.extra_flags_compat(), 0b0000_1101);
        assert_eq!(header.state_byte_compat(), 0x23);
        assert_eq!(header.footpad_flags_compat(), 0b1110_0011);
        assert_eq!(header.stop_setpoint_byte_compat(), 0xB6);
        assert_eq!(header.beep_reason_compat(), 19);
    }

    #[test]
    fn package_author_reads_realtime_data_item_ids_as_typed_contract() {
        assert_eq!(
            REFLOAT_REALTIME_DATA_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "motor.speed",
                "motor.erpm",
                "motor.current",
                "motor.dir_current",
                "motor.filt_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "motor.batt_current",
                "motor.mosfet_temp",
                "motor.motor_temp",
                "imu.pitch",
                "imu.balance_pitch",
                "imu.roll",
                "footpad.adc1",
                "footpad.adc2",
                "remote.input",
            ]
        );
        assert_eq!(
            REFLOAT_REALTIME_RUNTIME_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "setpoint",
                "atr.setpoint",
                "brake_tilt.setpoint",
                "torque_tilt.setpoint",
                "turn_tilt.setpoint",
                "remote.setpoint",
                "balance_current",
                "atr.accel_diff",
                "atr.speed_boost",
                "booster.current",
            ]
        );
        assert_eq!(
            REFLOAT_REALTIME_RECORDED_ITEMS.map(RefloatRealtimeDataItem::id),
            [
                "motor.erpm",
                "motor.dir_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "imu.pitch",
                "imu.balance_pitch",
                "setpoint",
                "atr.setpoint",
                "torque_tilt.setpoint",
                "balance_current",
            ]
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorSpeed.group(),
            RefloatRealtimeDataItemGroup::Always
        );
        assert_eq!(
            RefloatRealtimeDataItem::BalanceCurrent.group(),
            RefloatRealtimeDataItemGroup::Runtime
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorErpm.record_policy(),
            RefloatRealtimeDataRecordPolicy::Record
        );
        assert_eq!(
            RefloatRealtimeDataItem::MotorSpeed.record_policy(),
            RefloatRealtimeDataRecordPolicy::SendOnly
        );
    }

    #[test]
    fn package_author_reads_hardware_led_config_without_raw_bit_masks() {
        let disabled = RefloatHardwareLedsConfig::new(RefloatLedMode::Off);
        let internal = RefloatHardwareLedsConfig::new(RefloatLedMode::Internal);
        let external = RefloatHardwareLedsConfig::new(RefloatLedMode::External);
        let both = RefloatHardwareLedsConfig::new(RefloatLedMode::Both);

        assert_eq!(RefloatLedMode::Off.id(), 0);
        assert_eq!(RefloatLedMode::Internal.id(), 0x1);
        assert_eq!(RefloatLedMode::External.id(), 0x2);
        assert_eq!(RefloatLedMode::Both.id(), 0x3);
        assert!(!disabled.uses_internal_leds());
        assert!(!disabled.uses_external_leds());
        assert!(internal.uses_internal_leds());
        assert!(!internal.uses_external_leds());
        assert!(!external.uses_internal_leds());
        assert!(external.uses_external_leds());
        assert!(both.uses_internal_leds());
        assert!(both.uses_external_leds());
    }

    #[test]
    fn package_author_composes_hardware_leds_without_raw_config_fields() {
        let defaults = RefloatHardwareLedsConfig::new(RefloatLedMode::Off);

        assert_eq!(defaults.pin(), RefloatLedPin::B7);
        assert_eq!(defaults.pin_config(), RefloatLedPinConfig::PullupTo5v);
        assert_eq!(defaults.status_strip().order(), RefloatLedStripOrder::First);
        assert_eq!(defaults.status_strip().count(), 10);
        assert_eq!(defaults.front_strip().order(), RefloatLedStripOrder::Second);
        assert_eq!(defaults.front_strip().count(), 20);
        assert_eq!(defaults.rear_strip().order(), RefloatLedStripOrder::Third);
        assert_eq!(defaults.rear_strip().count(), 20);

        let status_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::First, 8, RefloatLedColorOrder::Grbw);
        let front_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::Second, 24, RefloatLedColorOrder::Rgb);
        let rear_strip =
            RefloatLedStripConfig::new(RefloatLedStripOrder::Third, 24, RefloatLedColorOrder::Grb)
                .with_reverse(true);

        let hardware_leds = RefloatHardwareLedsConfig::new(RefloatLedMode::Both)
            .with_pin(RefloatLedPin::C9)
            .with_pin_config(RefloatLedPinConfig::NoPullup)
            .with_status_strip(status_strip)
            .with_front_strip(front_strip)
            .with_rear_strip(rear_strip);
        let hardware = RefloatHardwareConfig::new(hardware_leds);

        assert_eq!(hardware.leds().mode(), RefloatLedMode::Both);
        assert_eq!(hardware.leds().pin(), RefloatLedPin::C9);
        assert_eq!(hardware.leds().pin_config(), RefloatLedPinConfig::NoPullup);
        assert_eq!(
            hardware.leds().status_strip().color_order(),
            RefloatLedColorOrder::Grbw
        );
        assert_eq!(
            hardware.leds().front_strip().color_order(),
            RefloatLedColorOrder::Rgb
        );
        assert!(hardware.leds().rear_strip().is_reversed());
    }

    #[test]
    fn package_author_reads_led_wiring_config_without_raw_numbers() {
        let strip = RefloatLedStripConfig::new(
            RefloatLedStripOrder::Second,
            24,
            RefloatLedColorOrder::Grbw,
        )
        .with_reverse(true);

        assert_eq!(RefloatLedPin::B6.id(), 0);
        assert_eq!(RefloatLedPin::B7.id(), 1);
        assert_eq!(RefloatLedPin::C9.id(), 2);
        assert_eq!(RefloatLedPinConfig::PullupTo5v.id(), 0);
        assert_eq!(RefloatLedPinConfig::NoPullup.id(), 1);
        assert_eq!(RefloatLedColorOrder::Grb.id(), 0);
        assert_eq!(RefloatLedColorOrder::Grbw.id(), 1);
        assert_eq!(RefloatLedColorOrder::Rgb.id(), 2);
        assert_eq!(RefloatLedColorOrder::Wrgb.id(), 3);
        assert_eq!(RefloatLedStripOrder::None.id(), 0);
        assert_eq!(RefloatLedStripOrder::First.id(), 1);
        assert_eq!(RefloatLedStripOrder::Second.id(), 2);
        assert_eq!(RefloatLedStripOrder::Third.id(), 3);
        assert_eq!(strip.order(), RefloatLedStripOrder::Second);
        assert_eq!(strip.count(), 24);
        assert_eq!(strip.color_order(), RefloatLedColorOrder::Grbw);
        assert!(strip.is_reversed());
    }

    #[test]
    fn package_author_reads_led_bar_config_without_raw_ids() {
        let bar = RefloatLedBarConfig::new(
            Ratio::from_ratio_const(0.8),
            RefloatLedColor::Gold,
            RefloatLedColor::Black,
            RefloatLedAnimationMode::Pulse,
            RefloatLedAnimationSpeed::from_units(1.5),
        );

        let color_ids = [
            (RefloatLedColor::Black, 0),
            (RefloatLedColor::WhiteFull, 1),
            (RefloatLedColor::WhiteRgb, 2),
            (RefloatLedColor::WhiteSingle, 3),
            (RefloatLedColor::Red, 4),
            (RefloatLedColor::Ferrari, 5),
            (RefloatLedColor::Flame, 6),
            (RefloatLedColor::Coral, 7),
            (RefloatLedColor::Sunset, 8),
            (RefloatLedColor::Sunrise, 9),
            (RefloatLedColor::Gold, 10),
            (RefloatLedColor::Orange, 11),
            (RefloatLedColor::Yellow, 12),
            (RefloatLedColor::Banana, 13),
            (RefloatLedColor::Lime, 14),
            (RefloatLedColor::Acid, 15),
            (RefloatLedColor::Sage, 16),
            (RefloatLedColor::Green, 17),
            (RefloatLedColor::Mint, 18),
            (RefloatLedColor::Tiffany, 19),
            (RefloatLedColor::Cyan, 20),
            (RefloatLedColor::Steel, 21),
            (RefloatLedColor::Sky, 22),
            (RefloatLedColor::Azure, 23),
            (RefloatLedColor::Sapphire, 24),
            (RefloatLedColor::Blue, 25),
            (RefloatLedColor::Violet, 26),
            (RefloatLedColor::Amethyst, 27),
            (RefloatLedColor::Magenta, 28),
            (RefloatLedColor::Pink, 29),
            (RefloatLedColor::Fuchsia, 30),
            (RefloatLedColor::Lavender, 31),
        ];
        let animation_ids = [
            (RefloatLedAnimationMode::Solid, 0),
            (RefloatLedAnimationMode::Fade, 1),
            (RefloatLedAnimationMode::Pulse, 2),
            (RefloatLedAnimationMode::Strobe, 3),
            (RefloatLedAnimationMode::KnightRider, 4),
            (RefloatLedAnimationMode::Felony, 5),
            (RefloatLedAnimationMode::RainbowCycle, 6),
            (RefloatLedAnimationMode::RainbowFade, 7),
            (RefloatLedAnimationMode::RainbowRoll, 8),
        ];
        let transition_ids = [
            (RefloatLedTransition::Fade, 0),
            (RefloatLedTransition::FadeOutIn, 1),
            (RefloatLedTransition::Cipher, 2),
            (RefloatLedTransition::MonoCipher, 3),
        ];

        assert!(
            color_ids
                .iter()
                .all(|(color, expected)| color.id() == *expected)
        );
        assert!(
            animation_ids
                .iter()
                .all(|(mode, expected)| mode.id() == *expected)
        );
        assert!(
            transition_ids
                .iter()
                .all(|(transition, expected)| transition.id() == *expected)
        );
        assert!((bar.brightness().as_ratio() - 0.8).abs() < f32::EPSILON);
        assert_eq!(bar.primary_color(), RefloatLedColor::Gold);
        assert_eq!(bar.secondary_color(), RefloatLedColor::Black);
        assert_eq!(bar.animation_mode(), RefloatLedAnimationMode::Pulse);
        assert!((bar.animation_speed().as_units() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn package_author_reads_status_bar_config_without_raw_scalars() {
        let status = RefloatStatusBarConfig::new(
            RefloatStatusBarIdleTimeout::from_seconds(30),
            Ratio::from_ratio_const(0.12),
            Ratio::from_ratio_const(0.25),
            Ratio::from_ratio_const(0.70),
            Ratio::from_ratio_const(0.20),
        )
        .showing_sensors_while_running();

        assert_eq!(status.idle_timeout().as_seconds(), 30);
        assert!((status.duty_threshold().as_ratio() - 0.12).abs() < f32::EPSILON);
        assert!((status.red_bar_percentage().as_ratio() - 0.25).abs() < f32::EPSILON);
        assert!(status.shows_sensors_while_running());
        assert!((status.brightness_headlights_on().as_ratio() - 0.70).abs() < f32::EPSILON);
        assert!((status.brightness_headlights_off().as_ratio() - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn package_author_composes_leds_config_without_raw_flags() {
        let headlights = RefloatLedBarConfig::new(
            Ratio::from_ratio_const(0.9),
            RefloatLedColor::WhiteFull,
            RefloatLedColor::Black,
            RefloatLedAnimationMode::Solid,
            RefloatLedAnimationSpeed::from_units(1.0),
        );
        let taillights = RefloatLedBarConfig::new(
            Ratio::from_ratio_const(0.5),
            RefloatLedColor::Red,
            RefloatLedColor::Black,
            RefloatLedAnimationMode::Pulse,
            RefloatLedAnimationSpeed::from_units(1.5),
        );
        let status = RefloatStatusBarConfig::new(
            RefloatStatusBarIdleTimeout::from_seconds(45),
            Ratio::from_ratio_const(0.10),
            Ratio::from_ratio_const(0.20),
            Ratio::from_ratio_const(0.75),
            Ratio::from_ratio_const(0.25),
        );

        let leds = RefloatLedsConfig::new(
            headlights, taillights, headlights, taillights, status, taillights,
        )
        .with_headlights_transition(RefloatLedTransition::FadeOutIn)
        .with_direction_transition(RefloatLedTransition::Cipher)
        .enabled()
        .with_headlights_on()
        .lights_off_when_lifted()
        .status_on_front_when_lifted();

        assert!(leds.is_enabled());
        assert!(leds.are_headlights_on());
        assert_eq!(
            leds.headlights_transition(),
            RefloatLedTransition::FadeOutIn
        );
        assert_eq!(leds.direction_transition(), RefloatLedTransition::Cipher);
        assert!(leds.turns_lights_off_when_lifted());
        assert!(leds.shows_status_on_front_when_lifted());
        assert_eq!(
            leds.headlights().primary_color(),
            RefloatLedColor::WhiteFull
        );
        assert_eq!(leds.taillights().primary_color(), RefloatLedColor::Red);
        assert_eq!(
            leds.front().animation_mode(),
            RefloatLedAnimationMode::Solid
        );
        assert_eq!(leds.rear().animation_mode(), RefloatLedAnimationMode::Pulse);
        assert_eq!(leds.status().idle_timeout().as_seconds(), 45);
        assert_eq!(leds.status_idle().primary_color(), RefloatLedColor::Red);
    }
}
