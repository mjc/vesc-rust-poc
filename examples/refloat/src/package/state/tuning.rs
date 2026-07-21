use super::*;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, IntegralCurrentGain,
    MahonyPitchGain, MotorCurrent, PidScale, RateCurrentGain, Ratio, Rpm, WireByte,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatTuneNibble(u8);

impl RefloatTuneNibble {
    const fn low(byte: u8) -> Self {
        Self(byte & 0x0F)
    }

    const fn high(byte: u8) -> Self {
        Self(byte >> 4)
    }

    fn scaled<T>(self, scale: f32, offset: f32, constructor: fn(f32) -> T) -> T {
        WireByte::new(self.0).scaled(scale, offset, constructor)
    }

    fn scaled_ratio<T>(
        self,
        numerator: f32,
        denominator: f32,
        offset: f32,
        constructor: fn(f32) -> T,
    ) -> T {
        WireByte::new(self.0).scaled_ratio(numerator, denominator, offset, constructor)
    }

    fn angle_from(self, base: AngleDegrees) -> AngleDegrees {
        base + match self.0 {
            0 => AngleDegrees::from_degrees(0.0),
            1 => AngleDegrees::from_degrees(1.0),
            2 => AngleDegrees::from_degrees(2.0),
            3 => AngleDegrees::from_degrees(3.0),
            4 => AngleDegrees::from_degrees(4.0),
            5 => AngleDegrees::from_degrees(5.0),
            6 => AngleDegrees::from_degrees(6.0),
            7 => AngleDegrees::from_degrees(7.0),
            8 => AngleDegrees::from_degrees(8.0),
            9 => AngleDegrees::from_degrees(9.0),
            10 => AngleDegrees::from_degrees(10.0),
            11 => AngleDegrees::from_degrees(11.0),
            12 => AngleDegrees::from_degrees(12.0),
            13 => AngleDegrees::from_degrees(13.0),
            14 => AngleDegrees::from_degrees(14.0),
            15 => AngleDegrees::from_degrees(15.0),
            _ => unreachable!(),
        }
    }

    fn booster_current(self) -> MotorCurrent {
        MotorCurrent::new(match self.0 {
            0 => Current::ZERO,
            1 => Current::from_amps(10.0),
            2 => Current::from_amps(12.0),
            3 => Current::from_amps(14.0),
            4 => Current::from_amps(16.0),
            5 => Current::from_amps(18.0),
            6 => Current::from_amps(20.0),
            7 => Current::from_amps(22.0),
            8 => Current::from_amps(24.0),
            9 => Current::from_amps(26.0),
            10 => Current::from_amps(28.0),
            11 => Current::from_amps(30.0),
            12 => Current::from_amps(32.0),
            13 => Current::from_amps(34.0),
            14 => Current::from_amps(36.0),
            15 => Current::from_amps(38.0),
            _ => unreachable!(),
        })
    }

    fn integral_gain(self) -> IntegralCurrentGain {
        match self.0 {
            0 => IntegralCurrentGain::new(0.0),
            1 => IntegralCurrentGain::new(0.005),
            value => WireByte::new(value - 1).scaled(0.01, 0.0, IntegralCurrentGain::new),
        }
    }

    fn integral_limit(self) -> MotorCurrent {
        if self.0 == 0 {
            MotorCurrent::new(Current::ZERO)
        } else {
            self.scaled(1.0, 19.0, motor_current)
        }
    }

    fn atr_strength(self) -> PidScale {
        if self.0 == 0 {
            PidScale::new(0.0)
        } else {
            self.scaled(0.1, 0.5, PidScale::new)
        }
    }
}

fn motor_current(amps: f32) -> MotorCurrent {
    MotorCurrent::new(Current::from_amps(amps))
}

fn electrical_speed(erpm: f32) -> ElectricalSpeed {
    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(erpm))
}

pub(super) fn handle_runtime_tune_packet(state: &mut RefloatPackageState, bytes: &[u8]) -> bool {
    let Some(payload) = refloat_command_payload(bytes, RefloatAppDataCommand::RuntimeTune) else {
        return false;
    };

    if let [
        pid,
        integral,
        booster,
        booster_turn,
        turn_filter,
        atr_strength,
        atr_speed,
        atr_limits,
        atr_boost,
        atr_ratios,
        brake_tilt,
        _unused,
        ..,
    ] = payload
    {
        let pid_low = RefloatTuneNibble::low(*pid);
        let pid_high = RefloatTuneNibble::high(*pid);
        let integral_low = RefloatTuneNibble::low(*integral);
        let integral_high = RefloatTuneNibble::high(*integral);
        let booster_low = RefloatTuneNibble::low(*booster);
        let booster_high = RefloatTuneNibble::high(*booster);
        let booster_current = RefloatTuneNibble::low(*booster_turn);
        let turn_strength = RefloatTuneNibble::high(*booster_turn);
        let turn = RefloatTuneNibble::low(*turn_filter);
        let mahony = RefloatTuneNibble::high(*turn_filter);
        let atr_up = RefloatTuneNibble::low(*atr_strength);
        let atr_down = RefloatTuneNibble::high(*atr_strength);
        let atr_speed_sign = RefloatTuneNibble::low(*atr_speed);
        let atr_speed_amount = RefloatTuneNibble::high(*atr_speed);
        let atr_angle = RefloatTuneNibble::low(*atr_limits);
        let atr_speeds = RefloatTuneNibble::high(*atr_limits);
        let response_boost = RefloatTuneNibble::low(*atr_boost);
        let transition_boost = RefloatTuneNibble::high(*atr_boost);
        let accel_ratio = RefloatTuneNibble::low(*atr_ratios);
        let decel_ratio = RefloatTuneNibble::high(*atr_ratios);
        let brake_strength = RefloatTuneNibble::low(*brake_tilt);
        let brake_lingering = RefloatTuneNibble::high(*brake_tilt);
        let speed_boost_scale = if atr_speed_sign.0 == 0 { 0.05 } else { -0.05 };

        let mut config = state.serialized_config.editor();
        let updated = [
            config.set_kp(pid_low.scaled(1.0, 15.0, AngleCurrentGain::new)),
            config.set_kp2(pid_high.scaled(0.1, 0.0, RateCurrentGain::new)),
            config.set_ki(integral_low.integral_gain()),
            config.set_ki_limit(integral_high.integral_limit()),
            config.set_booster_angle(booster_low.angle_from(AngleDegrees::from_degrees(5.0))),
            config.set_booster_ramp(booster_high.angle_from(AngleDegrees::from_degrees(2.0))),
            config.set_booster_current(booster_current.booster_current()),
            config.set_turn_tilt_strength(turn_strength.scaled(1.0, 0.0, PidScale::new)),
            config.set_turn_tilt_angle_limit(RefloatTuneNibble(turn.0 & 0x03).scaled(
                1.0,
                2.0,
                AngleDegrees::from_degrees,
            )),
            config.set_turn_tilt_start_erpm(RefloatTuneNibble(turn.0 >> 2).scaled(
                500.0,
                1000.0,
                electrical_speed,
            )),
            config.set_mahony_kp(mahony.scaled(0.1, 1.5, MahonyPitchGain::new)),
            config.set_atr_strength_up(atr_up.atr_strength()),
            config.set_atr_strength_down(atr_down.atr_strength()),
            config.set_atr_speed_boost(atr_speed_amount.scaled(
                speed_boost_scale,
                0.0,
                PidScale::new,
            )),
            config.set_atr_angle_limit(atr_angle.angle_from(AngleDegrees::from_degrees(5.0))),
            config.set_atr_on_speed(RefloatTuneNibble(atr_speeds.0 & 0x03).scaled(
                1.0,
                3.0,
                AngularVelocity::from_degrees_per_second,
            )),
            config.set_atr_off_speed(RefloatTuneNibble(atr_speeds.0 >> 2).scaled(
                1.0,
                2.0,
                AngularVelocity::from_degrees_per_second,
            )),
            config.set_atr_response_boost(response_boost.scaled(0.1, 1.0, PidScale::new)),
            config.set_atr_transition_boost(transition_boost.scaled(0.2, 1.0, PidScale::new)),
            config.set_atr_amps_accel_ratio(accel_ratio.scaled(1.0, 5.0, PidScale::new)),
            config.set_atr_amps_decel_ratio(decel_ratio.scaled(1.0, 5.0, PidScale::new)),
            config.set_brake_tilt_strength(brake_strength.scaled(1.0, 0.0, PidScale::new)),
            config.set_brake_tilt_lingering(brake_lingering.scaled(1.0, 0.0, PidScale::new)),
        ]
        .into_iter()
        .all(core::convert::identity);
        debug_assert!(updated);
    }

    if let Some([threshold, torque, torque_limits, torque_speeds]) = payload.get(12..16) {
        let threshold_up = RefloatTuneNibble::low(*threshold);
        let threshold_down = RefloatTuneNibble::high(*threshold);
        let torque_up = RefloatTuneNibble::low(*torque);
        let torque_down = RefloatTuneNibble::high(*torque);
        let torque_angle = RefloatTuneNibble::low(*torque_limits);
        let torque_current = RefloatTuneNibble::high(*torque_limits);
        let torque_on = RefloatTuneNibble::low(*torque_speeds);
        let torque_off = RefloatTuneNibble::high(*torque_speeds);
        let mut config = state.serialized_config.editor();
        let updated = [
            config.set_atr_threshold_up(threshold_up.scaled(0.5, 0.0, AngleDegrees::from_degrees)),
            config.set_atr_threshold_down(threshold_down.scaled(
                0.5,
                0.0,
                AngleDegrees::from_degrees,
            )),
            config.set_torque_tilt_strength(torque_up.scaled_ratio(0.3, 10.0, 0.0, PidScale::new)),
            config.set_torque_tilt_regen_strength(torque_down.scaled_ratio(
                0.3,
                10.0,
                0.0,
                PidScale::new,
            )),
            config.set_torque_tilt_angle_limit(torque_angle.scaled(
                0.5,
                0.0,
                AngleDegrees::from_degrees,
            )),
            config.set_torque_tilt_start_current(torque_current.scaled(1.0, 15.0, motor_current)),
            config.set_torque_tilt_on_speed(torque_on.scaled(
                0.5,
                0.0,
                AngularVelocity::from_degrees_per_second,
            )),
            config.set_torque_tilt_off_speed(torque_off.scaled(
                1.0,
                3.0,
                AngularVelocity::from_degrees_per_second,
            )),
        ]
        .into_iter()
        .all(core::convert::identity);
        debug_assert!(updated);
    }

    if let Some(brake) = payload.get(16) {
        let mut config = state.serialized_config.editor();
        let updated = [
            config.set_kp_brake(RefloatTuneNibble::low(*brake).scaled(0.1, 0.1, PidScale::new)),
            config.set_kp2_brake(RefloatTuneNibble::high(*brake).scaled(0.1, 0.0, PidScale::new)),
        ]
        .into_iter()
        .all(core::convert::identity);
        debug_assert!(updated);
        state.alert_beeper(RefloatBeeperAlert::Long(RefloatBeeperCount::ONE));
    }

    state.refresh_config_runtime_state();
    true
}

pub(super) fn handle_tilt_tune_packet(state: &mut RefloatPackageState, bytes: &[u8]) -> bool {
    let Some([flags, return_speed, duty, duty_angle, duty_speed, ..]) =
        refloat_command_payload(bytes, RefloatAppDataCommand::TuneTilt)
    else {
        return false;
    };

    let mut config = state.serialized_config.editor();
    let mut updated = [
        config.set_duty_beep_enabled(*flags & 0x01 != 0),
        config.set_duty_pushback_threshold(WireByte::new(*duty).scaled_ratio(
            1.0,
            100.0,
            0.0,
            Ratio::from_ratio_const,
        )),
        config.set_duty_pushback_angle(WireByte::new(*duty_angle).scaled(
            0.1,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_duty_pushback_speed(WireByte::new(*duty_speed).scaled(
            0.1,
            0.0,
            AngularVelocity::from_degrees_per_second,
        )),
    ]
    .into_iter()
    .all(core::convert::identity);
    if *return_speed != 0 {
        updated &= config.set_tiltback_return_speed(WireByte::new(*return_speed).scaled(
            0.1,
            0.0,
            AngularVelocity::from_degrees_per_second,
        ));
    }
    debug_assert!(updated);
    state.refresh_config_runtime_state();
    state.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::THREE));
    true
}

pub(super) fn handle_other_tune_packet(state: &mut RefloatPackageState, bytes: &[u8]) -> bool {
    let Some(
        [
            flags,
            startup_speed,
            pitch_tolerance,
            roll_tolerance,
            brake_current,
            click_current,
            tilt_constant,
            nose_speed,
            constant_erpm,
            variable_rate,
            variable_max,
            variable_erpm,
            optional_input @ ..,
        ],
    ) = refloat_command_payload(bytes, RefloatAppDataCommand::TuneOther)
    else {
        return false;
    };

    let mut config = state.serialized_config.editor();
    let mut updated = [
        config.set_beeper_enabled(*flags & 0x02 != 0),
        config.set_reversestop_enabled(*flags & 0x04 != 0),
        config.set_dual_switch(*flags & 0x08 != 0),
        config.set_darkride_enabled(*flags & 0x10 != 0),
        config.set_dirty_landings_enabled(*flags & 0x20 != 0),
        config.set_simplestart_enabled(*flags & 0x40 != 0),
        config.set_pushstart_enabled(*flags & 0x80 != 0),
        config.set_startup_speed(WireByte::new(*startup_speed).scaled(
            1.0,
            0.0,
            AngularVelocity::from_degrees_per_second,
        )),
        config.set_startup_pitch_tolerance(WireByte::new(*pitch_tolerance).scaled(
            0.1,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_startup_roll_tolerance(WireByte::new(*roll_tolerance).scaled(
            1.0,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_brake_current(MotorCurrent::new(WireByte::new(*brake_current).scaled(
            0.5,
            0.0,
            Current::from_amps,
        ))),
        config.set_startup_click_current(WireByte::new(*click_current)),
    ]
    .into_iter()
    .all(core::convert::identity);

    if (80..=120).contains(tilt_constant) {
        updated &= [
            config.set_tiltback_constant(WireByte::new(*tilt_constant).scaled(
                0.5,
                -50.0,
                AngleDegrees::from_degrees,
            )),
            config.set_tiltback_constant_erpm(WireByte::new(*constant_erpm).scaled(
                100.0,
                0.0,
                electrical_speed,
            )),
            config.set_tiltback_variable(WireByte::new(*variable_rate).scaled(
                0.01,
                0.0,
                PidScale::new,
            )),
            config.set_tiltback_variable_max(WireByte::new(*variable_max).scaled(
                0.1,
                0.0,
                AngleDegrees::from_degrees,
            )),
            config.set_tiltback_variable_erpm(WireByte::new(*variable_erpm).scaled(
                100.0,
                0.0,
                electrical_speed,
            )),
        ]
        .into_iter()
        .all(core::convert::identity);
        if *nose_speed != 0 {
            updated &= config.set_nose_angling_speed(WireByte::new(*nose_speed).scaled(
                0.1,
                0.0,
                AngularVelocity::from_degrees_per_second,
            ));
        }
    }

    if let [input, input_speed, ..] = optional_input {
        let remote_type = *input & 0x03;
        if remote_type <= 2 {
            updated &= config.set_input_tilt_remote_type(WireByte::new(remote_type));
            if remote_type != 0 {
                updated &= config.set_input_tilt_angle_limit(WireByte::new(*input >> 2).scaled(
                    1.0,
                    0.0,
                    AngleDegrees::from_degrees,
                ));
                updated &= config.set_input_tilt_speed(WireByte::new(*input_speed).scaled(
                    1.0,
                    0.0,
                    AngularVelocity::from_degrees_per_second,
                ));
            }
        }
    }

    debug_assert!(updated);
    state.refresh_config_runtime_state();
    true
}

pub(super) fn handle_booster_packet(state: &mut RefloatPackageState, bytes: &[u8]) -> bool {
    let Some(
        [
            booster,
            booster_current,
            brake_booster,
            brake_booster_current,
        ],
    ) = refloat_command_payload(bytes, RefloatAppDataCommand::Booster)
    else {
        return false;
    };

    // C map: `cmd_booster` splits four bytes into low/high nibbles at
    // `third_party/refloat/src/main.c:1448-1481`; only the low nibble of each
    // current byte is used.
    let mut config = state.serialized_config.editor();
    let updated = [
        config.set_booster_angle(
            RefloatTuneNibble::low(*booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_booster_ramp(
            RefloatTuneNibble::high(*booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_booster_current(RefloatTuneNibble::low(*booster_current).booster_current()),
        config.set_brake_booster_angle(
            RefloatTuneNibble::low(*brake_booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_brake_booster_ramp(
            RefloatTuneNibble::high(*brake_booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_brake_booster_current(
            RefloatTuneNibble::low(*brake_booster_current).booster_current(),
        ),
    ]
    .into_iter()
    .all(core::convert::identity);
    debug_assert!(updated);
    state.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::ONE));
    true
}

#[cfg(test)]
mod tests {
    use super::RefloatTuneNibble;
    use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent};

    #[test]
    fn tune_nibble_keeps_exact_endpoints_without_primitive_conversions() {
        assert_eq!(RefloatTuneNibble::low(0xF0), RefloatTuneNibble(0));
        assert_eq!(RefloatTuneNibble::high(0xF0), RefloatTuneNibble(15));
        assert_eq!(
            RefloatTuneNibble(0).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(5.0),
        );
        assert_eq!(
            RefloatTuneNibble(15).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(20.0),
        );
        assert_eq!(
            RefloatTuneNibble(0).booster_current(),
            MotorCurrent::new(Current::ZERO),
        );
        assert_eq!(
            RefloatTuneNibble(15).booster_current(),
            MotorCurrent::new(Current::from_amps(38.0)),
        );
    }
}
