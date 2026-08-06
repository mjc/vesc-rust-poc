use super::{
    FloatOutBoyAppDataCommand, FloatOutBoyBeeperAlert, FloatOutBoyPackageState,
    float_out_boy_command_payload,
};
use crate::config::FloatOutBoyConfigEditor;
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, IntegralCurrentGain,
    MahonyPitchGain, MotorCurrent, PidScale, RateCurrentGain, Ratio, Rpm, TimestampTicks, WireByte,
};

fn tune_angle_from(value: WireByte, base: AngleDegrees) -> AngleDegrees {
    base + AngleDegrees::from_degrees(f32::from(value.as_u8()))
}

fn tune_booster_current(value: WireByte) -> MotorCurrent {
    MotorCurrent::new(match value.as_u8() {
        0 => Current::ZERO,
        value => Current::from_amps(f32::from(value) * 2.0 + 8.0),
    })
}

fn tune_integral_gain(value: WireByte) -> IntegralCurrentGain {
    match value.as_u8() {
        0 => IntegralCurrentGain::new(0.0),
        1 => IntegralCurrentGain::new(0.005),
        value => WireByte::new(value.saturating_sub(1)).scaled_ratio(
            1.0,
            100.0,
            0.0,
            IntegralCurrentGain::new,
        ),
    }
}

fn tune_integral_limit(value: WireByte) -> MotorCurrent {
    if value.as_u8() == 0 {
        MotorCurrent::new(Current::ZERO)
    } else {
        value.scaled(1.0, 19.0, motor_current)
    }
}

fn tune_atr_strength(value: WireByte) -> PidScale {
    if value.as_u8() == 0 {
        PidScale::new(0.0)
    } else {
        value.divided(10.0, 0.5, PidScale::new)
    }
}

fn tune_torque_tilt_strength(value: WireByte) -> PidScale {
    value
        .divided(10.0, 0.0, PidScale::new)
        .scaled_by(PidScale::new(0.3))
}

fn tune_brake_gain(value: WireByte) -> PidScale {
    WireByte::new(value.as_u8().saturating_add(1)).divided(10.0, 0.0, PidScale::new)
}

fn motor_current(amps: f32) -> MotorCurrent {
    MotorCurrent::new(Current::from_amps(amps))
}

fn electrical_speed(erpm: f32) -> ElectricalSpeed {
    ElectricalSpeed::new(Rpm::from_revolutions_per_minute(erpm))
}

fn update_active_config(
    state: &mut FloatOutBoyPackageState,
    update: impl FnOnce(&mut FloatOutBoyConfigEditor<'_>) -> bool,
) -> bool {
    let mut config = state.serialized_config;
    let updated = update(&mut config.editor());
    if updated {
        state.replace_active_config(&config);
    }
    updated
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

    let pid_low = WireByte::low_nibble(*pid);
    let pid_high = WireByte::high_nibble(*pid);
    let integral_low = WireByte::low_nibble(*integral);
    let integral_high = WireByte::high_nibble(*integral);
    let booster_low = WireByte::low_nibble(*booster);
    let booster_high = WireByte::high_nibble(*booster);
    let booster_current = WireByte::low_nibble(*booster_turn);
    let turn_strength = WireByte::high_nibble(*booster_turn);
    let turn = WireByte::low_nibble(*turn_filter);
    let mahony = WireByte::high_nibble(*turn_filter);
    let atr_up = WireByte::low_nibble(*atr_strength);
    let atr_down = WireByte::high_nibble(*atr_strength);
    let atr_speed_sign = WireByte::low_nibble(*atr_speed);
    let atr_speed_amount = WireByte::high_nibble(*atr_speed);
    let atr_angle = WireByte::low_nibble(*atr_limits);
    let atr_speeds = WireByte::high_nibble(*atr_limits);
    let response_boost = WireByte::low_nibble(*atr_boost);
    let transition_boost = WireByte::high_nibble(*atr_boost);
    let accel_ratio = WireByte::low_nibble(*atr_ratios);
    let decel_ratio = WireByte::high_nibble(*atr_ratios);
    let brake_strength = WireByte::low_nibble(*brake_tilt);
    let brake_lingering = WireByte::high_nibble(*brake_tilt);
    let speed_boost_numerator = match atr_speed_sign.as_u8() {
        0 => 5.0,
        _ => -5.0,
    };

    update_active_config(state, |config| {
        [
            config.set_kp(pid_low.scaled(1.0, 15.0, AngleCurrentGain::new)),
            config.set_kp2(pid_high.divided(10.0, 0.0, RateCurrentGain::new)),
            config.set_ki(tune_integral_gain(integral_low)),
            config.set_ki_limit(tune_integral_limit(integral_high)),
            config.set_booster_angle(tune_angle_from(
                booster_low,
                AngleDegrees::from_degrees(5.0),
            )),
            config.set_booster_ramp(tune_angle_from(
                booster_high,
                AngleDegrees::from_degrees(2.0),
            )),
            config.set_booster_current(tune_booster_current(booster_current)),
            config.set_turn_tilt_strength(turn_strength.scaled(1.0, 0.0, PidScale::new)),
            config.set_turn_tilt_angle_limit(WireByte::new(turn.as_u8() & 0x03).scaled(
                1.0,
                2.0,
                AngleDegrees::from_degrees,
            )),
            config.set_turn_tilt_start_erpm(WireByte::new(turn.as_u8() >> 2).scaled(
                500.0,
                1000.0,
                electrical_speed,
            )),
            config.set_mahony_kp(mahony.divided(10.0, 1.5, MahonyPitchGain::new)),
            config.set_atr_strength_up(tune_atr_strength(atr_up)),
            config.set_atr_strength_down(tune_atr_strength(atr_down)),
            config.set_atr_speed_boost(atr_speed_amount.scaled_ratio(
                speed_boost_numerator,
                100.0,
                0.0,
                PidScale::new,
            )),
            config.set_atr_angle_limit(tune_angle_from(atr_angle, AngleDegrees::from_degrees(5.0))),
            config.set_atr_on_speed(WireByte::new(atr_speeds.as_u8() & 0x03).scaled(
                1.0,
                3.0,
                AngularVelocity::from_degrees_per_second,
            )),
            config.set_atr_off_speed(WireByte::new(atr_speeds.as_u8() >> 2).scaled(
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
    })
}

fn apply_torque_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some([threshold, torque, torque_limits, torque_speeds]) = payload.get(12..16) else {
        return true;
    };
    let threshold_up = WireByte::low_nibble(*threshold);
    let threshold_down = WireByte::high_nibble(*threshold);
    let torque_up = WireByte::low_nibble(*torque);
    let torque_down = WireByte::high_nibble(*torque);
    let torque_angle = WireByte::low_nibble(*torque_limits);
    let torque_current = WireByte::high_nibble(*torque_limits);
    let torque_on = WireByte::low_nibble(*torque_speeds);
    let torque_off = WireByte::high_nibble(*torque_speeds);
    update_active_config(state, |config| {
        [
            config.set_atr_threshold_up(threshold_up.scaled(0.5, 0.0, AngleDegrees::from_degrees)),
            config.set_atr_threshold_down(threshold_down.scaled(
                0.5,
                0.0,
                AngleDegrees::from_degrees,
            )),
            config.set_torque_tilt_strength(tune_torque_tilt_strength(torque_up)),
            config.set_torque_tilt_regen_strength(tune_torque_tilt_strength(torque_down)),
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
    })
}

fn apply_brake_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some(brake) = payload.get(16) else {
        return true;
    };
    let updated = update_active_config(state, |config| {
        [
            config.set_kp_brake(tune_brake_gain(WireByte::low_nibble(*brake))),
            config.set_kp2_brake(WireByte::high_nibble(*brake).divided(10.0, 0.0, PidScale::new)),
        ]
        .into_iter()
        .all(core::convert::identity)
    });
    if updated {
        state.alert_beeper(FloatOutBoyBeeperAlert::Long(1));
    }
    updated
}

pub(super) fn handle_runtime_tune_packet(
    state: &mut FloatOutBoyPackageState,
    now: &mut impl FnMut() -> TimestampTicks,
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
    state.refresh_idle_epoch(now());
    true
}

pub(super) fn handle_tilt_tune_packet(state: &mut FloatOutBoyPackageState, bytes: &[u8]) -> bool {
    let Some([flags, return_speed, duty, duty_angle, duty_speed, ..]) =
        float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::TuneTilt)
    else {
        return false;
    };

    let updated = update_active_config(state, |config| {
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
        updated
    });
    if !updated {
        return false;
    }
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(3));
    true
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered Tune Other transaction is smaller and preserves progressive write gates"
)]
pub(super) fn handle_other_tune_packet(
    state: &mut FloatOutBoyPackageState,
    now: &mut impl FnMut() -> TimestampTicks,
    bytes: &[u8],
) -> bool {
    let Some(payload) = float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::TuneOther)
    else {
        return false;
    };
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
        return false;
    };

    let updated = update_active_config(state, |config| {
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
            config.set_startup_pitch_tolerance(WireByte::new(*pitch_tolerance).scaled_ratio(
                1.0,
                10.0,
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

        if updated && (80..=120).contains(tilt_constant) {
            updated = [
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
                config.set_tiltback_variable(WireByte::new(*variable_rate).scaled_ratio(
                    1.0,
                    100.0,
                    0.0,
                    PidScale::new,
                )),
                config.set_tiltback_variable_max(WireByte::new(*variable_max).scaled_ratio(
                    1.0,
                    10.0,
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
                updated &= config.set_nose_angling_speed(WireByte::new(*nose_speed).scaled_ratio(
                    1.0,
                    10.0,
                    0.0,
                    AngularVelocity::from_degrees_per_second,
                ));
            }
        }

        if updated && let [input, input_speed, ..] = optional_input {
            let remote_type = *input & 0x03;
            if remote_type <= 2 {
                updated = config.set_input_tilt_remote_type(WireByte::new(remote_type));
                if remote_type != 0 {
                    updated &= config.set_input_tilt_angle_limit(
                        WireByte::new(*input >> 2).scaled(1.0, 0.0, AngleDegrees::from_degrees),
                    );
                    updated &= config.set_input_tilt_speed(WireByte::new(*input_speed).scaled(
                        1.0,
                        0.0,
                        AngularVelocity::from_degrees_per_second,
                    ));
                }
            }
        }
        updated
    });
    if !updated {
        return false;
    }
    state.refresh_idle_epoch(now());
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
    let updated = update_active_config(state, |config| {
        [
            config.set_booster_angle(tune_angle_from(
                WireByte::low_nibble(*booster),
                AngleDegrees::from_degrees(5.0),
            )),
            config.set_booster_ramp(tune_angle_from(
                WireByte::high_nibble(*booster),
                AngleDegrees::from_degrees(2.0),
            )),
            config
                .set_booster_current(tune_booster_current(WireByte::low_nibble(*booster_current))),
            config.set_brake_booster_angle(tune_angle_from(
                WireByte::low_nibble(*brake_booster),
                AngleDegrees::from_degrees(5.0),
            )),
            config.set_brake_booster_ramp(tune_angle_from(
                WireByte::high_nibble(*brake_booster),
                AngleDegrees::from_degrees(2.0),
            )),
            config.set_brake_booster_current(tune_booster_current(WireByte::low_nibble(
                *brake_booster_current,
            ))),
        ]
        .into_iter()
        .all(core::convert::identity)
    });
    if !updated {
        return false;
    }
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
    true
}

#[cfg(test)]
mod tests;
