use super::{
    FloatOutBoyAppDataCommand, FloatOutBoyBeeperAlert, FloatOutBoyPackageState,
    float_out_boy_command_payload,
};
use crate::config::{
    FloatOutBoyBalanceConfig as B, FloatOutBoyConfigEditor, FloatOutBoyConfigImage as C,
    FloatOutBoyFaultConfig as F, FloatOutBoyFilterConfig as H, FloatOutBoyMotorControlConfig as M,
    FloatOutBoyStartupConfig as S,
};
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

fn tune_angular_velocity(value: WireByte, offset: f32) -> AngularVelocity {
    value.scaled(1.0, offset, AngularVelocity::from_degrees_per_second)
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

fn all_updated<const N: usize>(updates: [bool; N]) -> bool {
    updates.into_iter().all(core::convert::identity)
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

    let (pid_low, pid_high) = WireByte::nibbles(*pid);
    let (integral_low, integral_high) = WireByte::nibbles(*integral);
    let (booster_low, booster_high) = WireByte::nibbles(*booster);
    let (booster_current, turn_strength) = WireByte::nibbles(*booster_turn);
    let (turn, mahony) = WireByte::nibbles(*turn_filter);
    let (atr_up, atr_down) = WireByte::nibbles(*atr_strength);
    let (atr_speed_sign, atr_speed_amount) = WireByte::nibbles(*atr_speed);
    let (atr_angle, atr_speeds) = WireByte::nibbles(*atr_limits);
    let (response_boost, transition_boost) = WireByte::nibbles(*atr_boost);
    let (accel_ratio, decel_ratio) = WireByte::nibbles(*atr_ratios);
    let (brake_strength, brake_lingering) = WireByte::nibbles(*brake_tilt);
    let speed_boost_numerator = match atr_speed_sign.as_u8() {
        0 => 5.0,
        _ => -5.0,
    };

    update_active_config(state, |config| {
        all_updated([
            B::set_kp(config, pid_low.scaled(1.0, 15.0, AngleCurrentGain::new)),
            B::set_kp2(config, pid_high.divided(10.0, 0.0, RateCurrentGain::new)),
            B::set_ki(config, tune_integral_gain(integral_low)),
            config.set_ki_limit(tune_integral_limit(integral_high)),
            B::set_booster_angle(
                config,
                tune_angle_from(booster_low, AngleDegrees::from_degrees(5.0)),
            ),
            B::set_booster_ramp(
                config,
                tune_angle_from(booster_high, AngleDegrees::from_degrees(2.0)),
            ),
            B::set_booster_current(config, tune_booster_current(booster_current)),
            B::set_turn_tilt_strength(config, turn_strength.scaled(1.0, 0.0, PidScale::new)),
            B::set_turn_tilt_angle_limit(
                config,
                WireByte::new(turn.as_u8() & 0x03).scaled(1.0, 2.0, AngleDegrees::from_degrees),
            ),
            B::set_turn_tilt_start_erpm(
                config,
                WireByte::new(turn.as_u8() >> 2).scaled(500.0, 1000.0, electrical_speed),
            ),
            H::set_mahony_kp(config, mahony.divided(10.0, 1.5, MahonyPitchGain::new)),
            B::set_atr_strength_up(config, tune_atr_strength(atr_up)),
            B::set_atr_strength_down(config, tune_atr_strength(atr_down)),
            B::set_atr_speed_boost(
                config,
                atr_speed_amount.scaled_ratio(speed_boost_numerator, 100.0, 0.0, PidScale::new),
            ),
            B::set_atr_angle_limit(
                config,
                tune_angle_from(atr_angle, AngleDegrees::from_degrees(5.0)),
            ),
            B::set_atr_on_speed(
                config,
                tune_angular_velocity(WireByte::new(atr_speeds.as_u8() & 0x03), 3.0),
            ),
            B::set_atr_off_speed(
                config,
                tune_angular_velocity(WireByte::new(atr_speeds.as_u8() >> 2), 2.0),
            ),
            B::set_atr_response_boost(config, response_boost.divided(10.0, 1.0, PidScale::new)),
            B::set_atr_transition_boost(config, transition_boost.divided(5.0, 1.0, PidScale::new)),
            B::set_atr_amps_accel_ratio(config, accel_ratio.scaled(1.0, 5.0, PidScale::new)),
            B::set_atr_amps_decel_ratio(config, decel_ratio.scaled(1.0, 5.0, PidScale::new)),
            B::set_brake_tilt_strength(config, brake_strength.scaled(1.0, 0.0, PidScale::new)),
            B::set_brake_tilt_lingering(config, brake_lingering.scaled(1.0, 0.0, PidScale::new)),
        ])
    })
}

fn apply_torque_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some([threshold, torque, torque_limits, torque_speeds]) = payload.get(12..16) else {
        return true;
    };
    let (threshold_up, threshold_down) = WireByte::nibbles(*threshold);
    let (torque_up, torque_down) = WireByte::nibbles(*torque);
    let (torque_angle, torque_current) = WireByte::nibbles(*torque_limits);
    let (torque_on, torque_off) = WireByte::nibbles(*torque_speeds);
    update_active_config(state, |config| {
        all_updated([
            B::set_atr_threshold_up(
                config,
                threshold_up.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            ),
            B::set_atr_threshold_down(
                config,
                threshold_down.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            ),
            B::set_torque_tilt_strength(config, tune_torque_tilt_strength(torque_up)),
            B::set_torque_tilt_regen_strength(config, tune_torque_tilt_strength(torque_down)),
            B::set_torque_tilt_angle_limit(
                config,
                torque_angle.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            ),
            B::set_torque_tilt_start_current(
                config,
                torque_current.scaled(1.0, 15.0, motor_current),
            ),
            B::set_torque_tilt_on_speed(
                config,
                torque_on.scaled(0.5, 0.0, AngularVelocity::from_degrees_per_second),
            ),
            B::set_torque_tilt_off_speed(
                config,
                torque_off.scaled(1.0, 3.0, AngularVelocity::from_degrees_per_second),
            ),
        ])
    })
}

fn apply_brake_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some(brake) = payload.get(16) else {
        return true;
    };
    let (brake_low, brake_high) = WireByte::nibbles(*brake);
    let updated = update_active_config(state, |config| {
        all_updated([
            B::set_kp_brake(config, tune_brake_gain(brake_low)),
            B::set_kp2_brake(config, brake_high.divided(10.0, 0.0, PidScale::new)),
        ])
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
        let mut updated = all_updated([
            C::set_duty_beep_enabled(config, *flags & 0x01 != 0),
            C::set_duty_pushback_threshold(
                config,
                WireByte::new(*duty).scaled_ratio(1.0, 100.0, 0.0, Ratio::from_ratio_const),
            ),
            C::set_duty_pushback_angle(
                config,
                WireByte::new(*duty_angle).scaled_ratio(1.0, 10.0, 0.0, AngleDegrees::from_degrees),
            ),
            C::set_duty_pushback_speed(
                config,
                WireByte::new(*duty_speed).scaled_ratio(
                    1.0,
                    10.0,
                    0.0,
                    AngularVelocity::from_degrees_per_second,
                ),
            ),
        ]);
        if *return_speed != 0 {
            updated &= C::set_tiltback_return_speed(
                config,
                WireByte::new(*return_speed).scaled_ratio(
                    1.0,
                    10.0,
                    0.0,
                    AngularVelocity::from_degrees_per_second,
                ),
            );
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
        let mut updated = all_updated([
            C::set_beeper_enabled(config, *flags & 0x02 != 0),
            F::set_reversestop_enabled(config, *flags & 0x04 != 0),
            F::set_dual_switch(config, *flags & 0x08 != 0),
            F::set_darkride_enabled(config, *flags & 0x10 != 0),
            config.set(S::DIRTY_LANDINGS_FIELD, *flags & 0x20 != 0),
            S::set_simplestart_enabled(config, *flags & 0x40 != 0),
            S::set_pushstart_enabled(config, *flags & 0x80 != 0),
            S::set_startup_speed(
                config,
                WireByte::new(*startup_speed).scaled(
                    1.0,
                    0.0,
                    AngularVelocity::from_degrees_per_second,
                ),
            ),
            S::set_startup_pitch_tolerance(
                config,
                WireByte::new(*pitch_tolerance).scaled_ratio(
                    1.0,
                    10.0,
                    0.0,
                    AngleDegrees::from_degrees,
                ),
            ),
            S::set_startup_roll_tolerance(
                config,
                WireByte::new(*roll_tolerance).scaled(1.0, 0.0, AngleDegrees::from_degrees),
            ),
            M::set_brake_current(
                config,
                MotorCurrent::new(WireByte::new(*brake_current).scaled(
                    0.5,
                    0.0,
                    Current::from_amps,
                )),
            ),
            config.set(S::CLICK_CURRENT_FIELD, WireByte::new(*click_current)),
        ]);

        if updated && (80..=120).contains(tilt_constant) {
            updated = all_updated([
                C::set_tiltback_constant(
                    config,
                    WireByte::new(*tilt_constant).scaled(0.5, -50.0, AngleDegrees::from_degrees),
                ),
                C::set_tiltback_constant_erpm(
                    config,
                    WireByte::new(*constant_erpm).scaled(100.0, 0.0, electrical_speed),
                ),
                C::set_tiltback_variable(
                    config,
                    WireByte::new(*variable_rate).scaled_ratio(1.0, 100.0, 0.0, PidScale::new),
                ),
                C::set_tiltback_variable_max(
                    config,
                    WireByte::new(*variable_max).scaled_ratio(
                        1.0,
                        10.0,
                        0.0,
                        AngleDegrees::from_degrees,
                    ),
                ),
                C::set_tiltback_variable_erpm(
                    config,
                    WireByte::new(*variable_erpm).scaled(100.0, 0.0, electrical_speed),
                ),
            ]);
            if *nose_speed != 0 {
                updated &= C::set_nose_angling_speed(
                    config,
                    WireByte::new(*nose_speed).scaled_ratio(
                        1.0,
                        10.0,
                        0.0,
                        AngularVelocity::from_degrees_per_second,
                    ),
                );
            }
        }

        if updated && let [input, input_speed, ..] = optional_input {
            let remote_type = *input & 0x03;
            if remote_type <= 2 {
                updated = config.set(C::INPUT_TILT_REMOTE_TYPE_FIELD, WireByte::new(remote_type));
                if remote_type != 0 {
                    updated &= C::set_input_tilt_angle_limit(
                        config,
                        WireByte::new(*input >> 2).scaled(1.0, 0.0, AngleDegrees::from_degrees),
                    );
                    updated &= C::set_input_tilt_speed(
                        config,
                        WireByte::new(*input_speed).scaled(
                            1.0,
                            0.0,
                            AngularVelocity::from_degrees_per_second,
                        ),
                    );
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
    let (booster_angle, booster_ramp) = WireByte::nibbles(*booster);
    let booster_current = WireByte::low_nibble(*booster_current);
    let (brake_angle, brake_ramp) = WireByte::nibbles(*brake_booster);
    let brake_current = WireByte::low_nibble(*brake_booster_current);
    let updated = update_active_config(state, |config| {
        all_updated([
            B::set_booster_angle(
                config,
                tune_angle_from(booster_angle, AngleDegrees::from_degrees(5.0)),
            ),
            B::set_booster_ramp(
                config,
                tune_angle_from(booster_ramp, AngleDegrees::from_degrees(2.0)),
            ),
            B::set_booster_current(config, tune_booster_current(booster_current)),
            B::set_brake_booster_angle(
                config,
                tune_angle_from(brake_angle, AngleDegrees::from_degrees(5.0)),
            ),
            B::set_brake_booster_ramp(
                config,
                tune_angle_from(brake_ramp, AngleDegrees::from_degrees(2.0)),
            ),
            B::set_brake_booster_current(config, tune_booster_current(brake_current)),
        ])
    });
    if !updated {
        return false;
    }
    state.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
    true
}

#[cfg(test)]
mod tests;
