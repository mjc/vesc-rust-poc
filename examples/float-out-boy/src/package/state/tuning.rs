use super::{
    FloatOutBoyAppDataCommand, FloatOutBoyBeeperAlert, FloatOutBoyBeeperCount,
    FloatOutBoyPackageState, float_out_boy_command_payload,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, IntegralCurrentGain,
    MahonyPitchGain, MotorCurrent, PidScale, RateCurrentGain, Ratio, Rpm, WireByte,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloatOutBoyTuneNibble(u8);

impl FloatOutBoyTuneNibble {
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

    fn divided<T>(self, denominator: f32, offset: f32, constructor: fn(f32) -> T) -> T {
        self.scaled_ratio(1.0, denominator, offset, constructor)
    }

    fn angle_from(self, base: AngleDegrees) -> AngleDegrees {
        base + AngleDegrees::from_degrees(f32::from(self.0))
    }

    fn booster_current(self) -> MotorCurrent {
        MotorCurrent::new(match self.0 {
            0 => Current::ZERO,
            value => Current::from_amps(f32::from(value) * 2.0 + 8.0),
        })
    }

    fn integral_gain(self) -> IntegralCurrentGain {
        match self.0 {
            0 => IntegralCurrentGain::new(0.0),
            1 => IntegralCurrentGain::new(0.005),
            value => {
                WireByte::new(value - 1).scaled_ratio(1.0, 100.0, 0.0, IntegralCurrentGain::new)
            }
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
            self.divided(10.0, 0.5, PidScale::new)
        }
    }

    fn torque_tilt_strength(self) -> PidScale {
        self.divided(10.0, 0.0, PidScale::new)
            .scaled_by(PidScale::new(0.3))
    }

    fn brake_gain(self) -> PidScale {
        WireByte::new(self.0 + 1).scaled_ratio(1.0, 10.0, 0.0, PidScale::new)
    }
}

fn motor_current(amps: f32) -> MotorCurrent {
    MotorCurrent::new(Current::from_amps(amps))
}

fn electrical_speed(erpm: f32) -> ElectricalSpeed {
    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(erpm))
}

fn apply_primary_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let [
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
    else {
        return true;
    };

    let pid_low = FloatOutBoyTuneNibble::low(*pid);
    let pid_high = FloatOutBoyTuneNibble::high(*pid);
    let integral_low = FloatOutBoyTuneNibble::low(*integral);
    let integral_high = FloatOutBoyTuneNibble::high(*integral);
    let booster_low = FloatOutBoyTuneNibble::low(*booster);
    let booster_high = FloatOutBoyTuneNibble::high(*booster);
    let booster_current = FloatOutBoyTuneNibble::low(*booster_turn);
    let turn_strength = FloatOutBoyTuneNibble::high(*booster_turn);
    let turn = FloatOutBoyTuneNibble::low(*turn_filter);
    let mahony = FloatOutBoyTuneNibble::high(*turn_filter);
    let atr_up = FloatOutBoyTuneNibble::low(*atr_strength);
    let atr_down = FloatOutBoyTuneNibble::high(*atr_strength);
    let atr_speed_sign = FloatOutBoyTuneNibble::low(*atr_speed);
    let atr_speed_amount = FloatOutBoyTuneNibble::high(*atr_speed);
    let atr_angle = FloatOutBoyTuneNibble::low(*atr_limits);
    let atr_speeds = FloatOutBoyTuneNibble::high(*atr_limits);
    let response_boost = FloatOutBoyTuneNibble::low(*atr_boost);
    let transition_boost = FloatOutBoyTuneNibble::high(*atr_boost);
    let accel_ratio = FloatOutBoyTuneNibble::low(*atr_ratios);
    let decel_ratio = FloatOutBoyTuneNibble::high(*atr_ratios);
    let brake_strength = FloatOutBoyTuneNibble::low(*brake_tilt);
    let brake_lingering = FloatOutBoyTuneNibble::high(*brake_tilt);
    let speed_boost_numerator = if atr_speed_sign.0 == 0 { 5.0 } else { -5.0 };

    let mut config = state.serialized_config.editor();
    [
        config.set_kp(pid_low.scaled(1.0, 15.0, AngleCurrentGain::new)),
        config.set_kp2(pid_high.divided(10.0, 0.0, RateCurrentGain::new)),
        config.set_ki(integral_low.integral_gain()),
        config.set_ki_limit(integral_high.integral_limit()),
        config.set_booster_angle(booster_low.angle_from(AngleDegrees::from_degrees(5.0))),
        config.set_booster_ramp(booster_high.angle_from(AngleDegrees::from_degrees(2.0))),
        config.set_booster_current(booster_current.booster_current()),
        config.set_turn_tilt_strength(turn_strength.scaled(1.0, 0.0, PidScale::new)),
        config.set_turn_tilt_angle_limit(FloatOutBoyTuneNibble(turn.0 & 0x03).scaled(
            1.0,
            2.0,
            AngleDegrees::from_degrees,
        )),
        config.set_turn_tilt_start_erpm(FloatOutBoyTuneNibble(turn.0 >> 2).scaled(
            500.0,
            1000.0,
            electrical_speed,
        )),
        config.set_mahony_kp(mahony.divided(10.0, 1.5, MahonyPitchGain::new)),
        config.set_atr_strength_up(atr_up.atr_strength()),
        config.set_atr_strength_down(atr_down.atr_strength()),
        config.set_atr_speed_boost(atr_speed_amount.scaled_ratio(
            speed_boost_numerator,
            100.0,
            0.0,
            PidScale::new,
        )),
        config.set_atr_angle_limit(atr_angle.angle_from(AngleDegrees::from_degrees(5.0))),
        config.set_atr_on_speed(FloatOutBoyTuneNibble(atr_speeds.0 & 0x03).scaled(
            1.0,
            3.0,
            AngularVelocity::from_degrees_per_second,
        )),
        config.set_atr_off_speed(FloatOutBoyTuneNibble(atr_speeds.0 >> 2).scaled(
            1.0,
            2.0,
            AngularVelocity::from_degrees_per_second,
        )),
        config.set_atr_response_boost(response_boost.divided(10.0, 1.0, PidScale::new)),
        config.set_atr_transition_boost(transition_boost.divided(5.0, 1.0, PidScale::new)),
        config.set_atr_amps_accel_ratio(accel_ratio.scaled(1.0, 5.0, PidScale::new)),
        config.set_atr_amps_decel_ratio(decel_ratio.scaled(1.0, 5.0, PidScale::new)),
        config.set_brake_tilt_strength(brake_strength.scaled(1.0, 0.0, PidScale::new)),
        config.set_brake_tilt_lingering(brake_lingering.scaled(1.0, 0.0, PidScale::new)),
    ]
    .into_iter()
    .all(core::convert::identity)
}

fn apply_torque_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some([threshold, torque, torque_limits, torque_speeds]) = payload.get(12..16) else {
        return true;
    };
    let threshold_up = FloatOutBoyTuneNibble::low(*threshold);
    let threshold_down = FloatOutBoyTuneNibble::high(*threshold);
    let torque_up = FloatOutBoyTuneNibble::low(*torque);
    let torque_down = FloatOutBoyTuneNibble::high(*torque);
    let torque_angle = FloatOutBoyTuneNibble::low(*torque_limits);
    let torque_current = FloatOutBoyTuneNibble::high(*torque_limits);
    let torque_on = FloatOutBoyTuneNibble::low(*torque_speeds);
    let torque_off = FloatOutBoyTuneNibble::high(*torque_speeds);
    let mut config = state.serialized_config.editor();
    [
        config.set_atr_threshold_up(threshold_up.scaled(0.5, 0.0, AngleDegrees::from_degrees)),
        config.set_atr_threshold_down(threshold_down.scaled(0.5, 0.0, AngleDegrees::from_degrees)),
        config.set_torque_tilt_strength(torque_up.torque_tilt_strength()),
        config.set_torque_tilt_regen_strength(torque_down.torque_tilt_strength()),
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
    .all(core::convert::identity)
}

fn apply_brake_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some(brake) = payload.get(16) else {
        return true;
    };
    let mut config = state.serialized_config.editor();
    let updated = [
        config.set_kp_brake(FloatOutBoyTuneNibble::low(*brake).brake_gain()),
        config.set_kp2_brake(FloatOutBoyTuneNibble::high(*brake).divided(10.0, 0.0, PidScale::new)),
    ]
    .into_iter()
    .all(core::convert::identity);
    if updated {
        state.alert_beeper(FloatOutBoyBeeperAlert::Long(FloatOutBoyBeeperCount::ONE));
    }
    updated
}

pub(super) fn handle_runtime_tune_packet(
    state: &mut FloatOutBoyPackageState,
    bytes: &[u8],
) -> bool {
    let Some(payload) =
        float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::RuntimeTune)
    else {
        return false;
    };

    let updated = apply_primary_runtime_tune(state, payload)
        && apply_torque_runtime_tune(state, payload)
        && apply_brake_runtime_tune(state, payload);
    if !updated {
        return false;
    }
    state.refresh_balance_filter_config();
    state.refresh_config_runtime_state();
    true
}

pub(super) fn handle_tilt_tune_packet(state: &mut FloatOutBoyPackageState, bytes: &[u8]) -> bool {
    let Some([flags, return_speed, duty, duty_angle, duty_speed, ..]) =
        float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::TuneTilt)
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
        config.set_duty_pushback_angle(WireByte::new(*duty_angle).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_duty_pushback_speed(WireByte::new(*duty_speed).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngularVelocity::from_degrees_per_second,
        )),
    ]
    .into_iter()
    .all(core::convert::identity);
    if *return_speed != 0 {
        updated &= config.set_tiltback_return_speed(WireByte::new(*return_speed).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngularVelocity::from_degrees_per_second,
        ));
    }
    debug_assert!(updated);
    state.refresh_config_runtime_state();
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::THREE));
    true
}

struct OtherTunePacket<'a> {
    flags: u8,
    startup_speed: u8,
    pitch_tolerance: u8,
    roll_tolerance: u8,
    brake_current: u8,
    click_current: u8,
    tilt_constant: u8,
    nose_speed: u8,
    constant_erpm: u8,
    variable_rate: u8,
    variable_max: u8,
    variable_erpm: u8,
    optional_input: &'a [u8],
}

impl<'a> OtherTunePacket<'a> {
    fn parse(payload: &'a [u8]) -> Option<Self> {
        let [
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
        ] = payload
        else {
            return None;
        };
        Some(Self {
            flags: *flags,
            startup_speed: *startup_speed,
            pitch_tolerance: *pitch_tolerance,
            roll_tolerance: *roll_tolerance,
            brake_current: *brake_current,
            click_current: *click_current,
            tilt_constant: *tilt_constant,
            nose_speed: *nose_speed,
            constant_erpm: *constant_erpm,
            variable_rate: *variable_rate,
            variable_max: *variable_max,
            variable_erpm: *variable_erpm,
            optional_input,
        })
    }
}

fn apply_other_tune_base(
    config: &mut crate::config::FloatOutBoyConfigEditor<'_>,
    packet: &OtherTunePacket<'_>,
) -> bool {
    [
        config.set_beeper_enabled(packet.flags & 0x02 != 0),
        config.set_reversestop_enabled(packet.flags & 0x04 != 0),
        config.set_dual_switch(packet.flags & 0x08 != 0),
        config.set_darkride_enabled(packet.flags & 0x10 != 0),
        config.set_dirty_landings_enabled(packet.flags & 0x20 != 0),
        config.set_simplestart_enabled(packet.flags & 0x40 != 0),
        config.set_pushstart_enabled(packet.flags & 0x80 != 0),
        config.set_startup_speed(WireByte::new(packet.startup_speed).scaled(
            1.0,
            0.0,
            AngularVelocity::from_degrees_per_second,
        )),
        config.set_startup_pitch_tolerance(WireByte::new(packet.pitch_tolerance).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_startup_roll_tolerance(WireByte::new(packet.roll_tolerance).scaled(
            1.0,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_brake_current(MotorCurrent::new(
            WireByte::new(packet.brake_current).scaled(0.5, 0.0, Current::from_amps),
        )),
        config.set_startup_click_current(WireByte::new(packet.click_current)),
    ]
    .into_iter()
    .all(core::convert::identity)
}

fn apply_other_tiltback(
    config: &mut crate::config::FloatOutBoyConfigEditor<'_>,
    packet: &OtherTunePacket<'_>,
) -> bool {
    if !(80..=120).contains(&packet.tilt_constant) {
        return true;
    }
    let mut updated = [
        config.set_tiltback_constant(WireByte::new(packet.tilt_constant).scaled(
            0.5,
            -50.0,
            AngleDegrees::from_degrees,
        )),
        config.set_tiltback_constant_erpm(WireByte::new(packet.constant_erpm).scaled(
            100.0,
            0.0,
            electrical_speed,
        )),
        config.set_tiltback_variable(WireByte::new(packet.variable_rate).scaled_ratio(
            1.0,
            100.0,
            0.0,
            PidScale::new,
        )),
        config.set_tiltback_variable_max(WireByte::new(packet.variable_max).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngleDegrees::from_degrees,
        )),
        config.set_tiltback_variable_erpm(WireByte::new(packet.variable_erpm).scaled(
            100.0,
            0.0,
            electrical_speed,
        )),
    ]
    .into_iter()
    .all(core::convert::identity);
    if packet.nose_speed != 0 {
        updated &= config.set_nose_angling_speed(WireByte::new(packet.nose_speed).scaled_ratio(
            1.0,
            10.0,
            0.0,
            AngularVelocity::from_degrees_per_second,
        ));
    }
    updated
}

fn apply_other_input(
    config: &mut crate::config::FloatOutBoyConfigEditor<'_>,
    packet: &OtherTunePacket<'_>,
) -> bool {
    let [input, input_speed, ..] = packet.optional_input else {
        return true;
    };
    let remote_type = *input & 0x03;
    if remote_type > 2 {
        return true;
    }
    let mut updated = config.set_input_tilt_remote_type(WireByte::new(remote_type));
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
    updated
}

pub(super) fn handle_other_tune_packet(state: &mut FloatOutBoyPackageState, bytes: &[u8]) -> bool {
    let Some(payload) = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::TuneOther)
    else {
        return false;
    };
    let Some(packet) = OtherTunePacket::parse(payload) else {
        return false;
    };

    let mut config = state.serialized_config.editor();
    let updated = apply_other_tune_base(&mut config, &packet)
        && apply_other_tiltback(&mut config, &packet)
        && apply_other_input(&mut config, &packet);
    if !updated {
        return false;
    }
    state.refresh_balance_filter_config();
    state.refresh_config_runtime_state();
    true
}

pub(super) fn handle_booster_packet(state: &mut FloatOutBoyPackageState, bytes: &[u8]) -> bool {
    let Some(
        [
            booster,
            booster_current,
            brake_booster,
            brake_booster_current,
        ],
    ) = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::Booster)
    else {
        return false;
    };

    // C map: `cmd_booster` splits four bytes into low/high nibbles at
    // `third_party/float-out-boy/src/main.c:1448-1481`; only the low nibble of each
    // current byte is used.
    let mut config = state.serialized_config.editor();
    let updated = [
        config.set_booster_angle(
            FloatOutBoyTuneNibble::low(*booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_booster_ramp(
            FloatOutBoyTuneNibble::high(*booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_booster_current(FloatOutBoyTuneNibble::low(*booster_current).booster_current()),
        config.set_brake_booster_angle(
            FloatOutBoyTuneNibble::low(*brake_booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_brake_booster_ramp(
            FloatOutBoyTuneNibble::high(*brake_booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_brake_booster_current(
            FloatOutBoyTuneNibble::low(*brake_booster_current).booster_current(),
        ),
    ]
    .into_iter()
    .all(core::convert::identity);
    debug_assert!(updated);
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(FloatOutBoyBeeperCount::ONE));
    true
}

#[cfg(test)]
mod tests {
    use super::FloatOutBoyTuneNibble;
    use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent, PidScale, RateCurrentGain};

    #[test]
    fn tune_nibble_keeps_exact_endpoints_without_primitive_conversions() {
        assert_eq!(FloatOutBoyTuneNibble::low(0xF0), FloatOutBoyTuneNibble(0));
        assert_eq!(FloatOutBoyTuneNibble::high(0xF0), FloatOutBoyTuneNibble(15));
        assert_eq!(
            FloatOutBoyTuneNibble(0).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(5.0),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(15).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(20.0),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(0).booster_current(),
            MotorCurrent::new(Current::ZERO),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(15).booster_current(),
            MotorCurrent::new(Current::from_amps(38.0)),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(9).divided(10.0, 0.0, RateCurrentGain::new),
            RateCurrentGain::new(9.0 / 10.0),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(9).torque_tilt_strength(),
            PidScale::new((9.0 / 10.0) * 0.3),
        );
        assert_eq!(
            FloatOutBoyTuneNibble(6).brake_gain(),
            PidScale::new((6.0 + 1.0) / 10.0),
        );
    }
}
