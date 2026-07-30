use super::{
    FloatOutBoyAppDataCommand, FloatOutBoyBeeperAlert, FloatOutBoyPackageState,
    float_out_boy_command_payload,
};
use crate::config::{
    FloatOutBoyBalanceConfig as B, FloatOutBoyConfigEditor, FloatOutBoyConfigImage as C,
    FloatOutBoyFaultConfig as F, FloatOutBoyFilterConfig as H, FloatOutBoyMotorControlConfig as M,
    FloatOutBoyParkingBrakeMode, FloatOutBoyStartupConfig as S,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, IntegralCurrentGain,
    MahonyPitchGain, MotorCurrent, PidScale, RateCurrentGain, Ratio, Rpm, TimestampTicks, WireByte,
};

fn tune_angle_from(value: WireByte, base: AngleDegrees) -> AngleDegrees {
    base + AngleDegrees::from_degrees(f32::from(value.as_u8()))
}

fn tune_variable_tilt_maximum(value: u8) -> AngleDegrees {
    if value > 100 {
        WireByte::new(value.saturating_sub(100)).scaled_ratio(
            -1.0,
            10.0,
            0.0,
            AngleDegrees::from_degrees,
        )
    } else {
        WireByte::new(value).scaled_ratio(1.0, 10.0, 0.0, AngleDegrees::from_degrees)
    }
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

use vescpkg_rs::set_custom_config_fields as write_fields;

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
    let (_, transition_boost) = WireByte::nibbles(*atr_boost);
    let (accel_ratio, decel_ratio) = WireByte::nibbles(*atr_ratios);
    let (brake_strength, brake_lingering) = WireByte::nibbles(*brake_tilt);
    let speed_boost_numerator = match atr_speed_sign.as_u8() {
        0 => 5.0,
        _ => -5.0,
    };

    update_active_config(state, |config| {
        let updated = write_fields!(config;
            B::KP_FIELD => pid_low.scaled(1.0, 15.0, AngleCurrentGain::new),
            B::KP2_FIELD => pid_high.divided(10.0, 0.0, RateCurrentGain::new),
            B::KI_FIELD => tune_integral_gain(integral_low),
            B::KI_LIMIT_FIELD => tune_integral_limit(integral_high),
            B::BOOSTER_ANGLE_FIELD => tune_angle_from(booster_low, AngleDegrees::from_degrees(5.0)),
            B::BOOSTER_RAMP_FIELD => tune_angle_from(booster_high, AngleDegrees::from_degrees(2.0)),
            B::BOOSTER_CURRENT_FIELD => tune_booster_current(booster_current),
            B::TURN_TILT_STRENGTH_FIELD => turn_strength.scaled(1.0, 0.0, PidScale::new),
            B::TURN_TILT_ANGLE_LIMIT_FIELD => WireByte::new(turn.as_u8() & 0x03).scaled(1.0, 2.0, AngleDegrees::from_degrees),
            B::TURN_TILT_START_ERPM_FIELD => WireByte::new(turn.as_u8() >> 2).scaled(500.0, 1000.0, electrical_speed),
            H::MAHONY_KP_FIELD => mahony.divided(10.0, 1.5, MahonyPitchGain::new),
            B::ATR_STRENGTH_UP_FIELD => tune_atr_strength(atr_up),
            B::ATR_STRENGTH_DOWN_FIELD => tune_atr_strength(atr_down),
            B::ATR_SPEED_BOOST_FIELD => atr_speed_amount.scaled_ratio(speed_boost_numerator, 100.0, 0.0, PidScale::new),
            B::ATR_ANGLE_LIMIT_FIELD => tune_angle_from(atr_angle, AngleDegrees::from_degrees(5.0)),
            B::ATR_TRANSITION_BOOST_FIELD => transition_boost.divided(5.0, 1.0, PidScale::new),
            B::ATR_AMPS_ACCEL_RATIO_FIELD => accel_ratio.scaled(1.0, 5.0, PidScale::new),
            B::ATR_AMPS_DECEL_RATIO_FIELD => decel_ratio.scaled(1.0, 5.0, PidScale::new),
            B::BRAKE_TILT_STRENGTH_FIELD => brake_strength.scaled(1.0, 0.0, PidScale::new),
            B::BRAKE_TILT_LINGERING_FIELD => brake_lingering.scaled(1.0, 0.0, PidScale::new),
        );
        let speeds_updated = atr_speeds.as_u8() == 0
            || write_fields!(config;
                B::ATR_FILTER_ON_SPEED_LIMIT_FIELD => tune_angular_velocity(WireByte::new(atr_speeds.as_u8() & 0x03), 3.0),
                B::ATR_FILTER_OFF_SPEED_LIMIT_FIELD => tune_angular_velocity(WireByte::new(atr_speeds.as_u8() >> 2), 2.0),
            );
        updated && speeds_updated
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
    let torque_on = if payload.len() >= 19 {
        torque_on.scaled(1.0, 3.0, AngularVelocity::from_degrees_per_second)
    } else {
        torque_on.scaled(0.5, 0.0, AngularVelocity::from_degrees_per_second)
    };
    update_active_config(state, |config| {
        write_fields!(config;
            B::ATR_THRESHOLD_UP_FIELD => threshold_up.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            B::ATR_THRESHOLD_DOWN_FIELD => threshold_down.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            B::TORQUE_TILT_STRENGTH_FIELD => tune_torque_tilt_strength(torque_up),
            B::TORQUE_TILT_REGEN_STRENGTH_FIELD => tune_torque_tilt_strength(torque_down),
            B::TORQUE_TILT_ANGLE_LIMIT_FIELD => torque_angle.scaled(0.5, 0.0, AngleDegrees::from_degrees),
            B::TORQUE_TILT_START_CURRENT_FIELD => torque_current.scaled(1.0, 15.0, motor_current),
            B::TORQUE_TILT_FILTER_ON_SPEED_LIMIT_FIELD => torque_on,
            B::TORQUE_TILT_FILTER_OFF_SPEED_LIMIT_FIELD => torque_off.scaled(1.0, 3.0, AngularVelocity::from_degrees_per_second),
        )
    })
}

fn apply_extended_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some([orientation, atr_speeds]) = payload.get(17..19) else {
        return true;
    };
    let (roll_gain, turn_start_angle) = WireByte::nibbles(*orientation);
    let (atr_on, atr_off) = WireByte::nibbles(*atr_speeds);

    update_active_config(state, |config| {
        let mut updated = true;
        if roll_gain.as_u8() > 0 {
            updated &= config.set(
                H::MAHONY_KP_ROLL_FIELD,
                roll_gain.divided(10.0, 1.0, vescpkg_rs::MahonyRollGain::new),
            );
        }
        if turn_start_angle.as_u8() > 0 {
            updated &= config.set(
                B::TURN_TILT_START_ANGLE_FIELD,
                turn_start_angle.scaled(1.0, 0.0, AngleDegrees::from_degrees),
            );
        }
        if atr_on.as_u8() > 0 && atr_off.as_u8() > 0 {
            updated &= config.set(
                B::ATR_FILTER_ON_SPEED_LIMIT_FIELD,
                atr_on.scaled(2.0, 0.0, AngularVelocity::from_degrees_per_second),
            );
            updated &= config.set(
                B::ATR_FILTER_OFF_SPEED_LIMIT_FIELD,
                atr_off.scaled(2.0, 0.0, AngularVelocity::from_degrees_per_second),
            );
        }
        updated
    })
}

fn apply_brake_runtime_tune(state: &mut FloatOutBoyPackageState, payload: &[u8]) -> bool {
    let Some(brake) = payload.get(16) else {
        return true;
    };
    let (brake_low, brake_high) = WireByte::nibbles(*brake);
    let updated = update_active_config(state, |config| {
        write_fields!(config;
            B::KP_BRAKE_FIELD => tune_brake_gain(brake_low),
            B::KP2_BRAKE_FIELD => brake_high.divided(10.0, 0.0, PidScale::new),
        )
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
        && apply_brake_runtime_tune(state, payload)
        && apply_extended_runtime_tune(state, payload);
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
        let mut updated = write_fields!(config;
            C::DUTY_BEEP_ENABLED_FIELD => *flags & 0x01 != 0,
            C::DUTY_PUSHBACK_THRESHOLD_FIELD => WireByte::new(*duty).scaled_ratio(1.0, 100.0, 0.0, Ratio::from_ratio_const),
            C::DUTY_PUSHBACK_ANGLE_FIELD => WireByte::new(*duty_angle).scaled_ratio(1.0, 10.0, 0.0, AngleDegrees::from_degrees),
            C::DUTY_PUSHBACK_SPEED_FIELD => WireByte::new(*duty_speed).scaled_ratio(1.0, 10.0, 0.0, AngularVelocity::from_degrees_per_second),
        );
        if *return_speed != 0 {
            updated &= config.set(
                C::TILTBACK_RETURN_SPEED_FIELD,
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
    if updated {
        state.alert_beeper(FloatOutBoyBeeperAlert::Short(3));
    }
    updated
}

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
        let mut updated = write_fields!(config;
            C::BEEPER_ENABLED_FIELD => *flags & 0x02 != 0,
            F::REVERSESTOP_FIELD => *flags & 0x04 != 0,
            F::DUAL_SWITCH_FIELD => *flags & 0x08 != 0,
            F::DARKRIDE_FIELD => *flags & 0x10 != 0,
            S::DIRTY_LANDINGS_FIELD => *flags & 0x20 != 0,
            S::SIMPLESTART_FIELD => *flags & 0x40 != 0,
            S::PUSHSTART_FIELD => *flags & 0x80 != 0,
            S::SPEED_FIELD => WireByte::new(*startup_speed).scaled(1.0, 0.0, AngularVelocity::from_degrees_per_second),
            S::PITCH_TOLERANCE_FIELD => WireByte::new(*pitch_tolerance).scaled_ratio(1.0, 10.0, 0.0, AngleDegrees::from_degrees),
            S::ROLL_TOLERANCE_FIELD => WireByte::new(*roll_tolerance).scaled(1.0, 0.0, AngleDegrees::from_degrees),
            M::BRAKE_CURRENT_FIELD => MotorCurrent::new(WireByte::new(*brake_current).scaled(0.5, 0.0, Current::from_amps)),
            S::CLICK_CURRENT_FIELD => WireByte::new(*click_current),
        );

        if updated && (80..=120).contains(tilt_constant) {
            updated = write_fields!(config;
                C::TILTBACK_CONSTANT_ANGLE_FIELD => WireByte::new(*tilt_constant).scaled(0.5, -50.0, AngleDegrees::from_degrees),
                C::TILTBACK_CONSTANT_ERPM_FIELD => WireByte::new(*constant_erpm).scaled(100.0, 0.0, electrical_speed),
                C::TILTBACK_VARIABLE_RATE_FIELD => WireByte::new(*variable_rate).scaled_ratio(1.0, 100.0, 0.0, PidScale::new),
                C::TILTBACK_VARIABLE_MAX_FIELD => tune_variable_tilt_maximum(*variable_max),
                C::TILTBACK_VARIABLE_ERPM_FIELD => WireByte::new(*variable_erpm).scaled(100.0, 0.0, electrical_speed),
            );
            if *nose_speed != 0 {
                updated &= config.set(
                    C::NOSE_ANGLING_SPEED_FIELD,
                    WireByte::new(*nose_speed).scaled_ratio(
                        1.0,
                        10.0,
                        0.0,
                        AngularVelocity::from_degrees_per_second,
                    ),
                );
            }
        }

        if updated && let [input, _input_speed, ..] = optional_input {
            let remote_type = *input & 0x03;
            if remote_type <= 2 {
                updated = config.set(C::INPUT_TILT_REMOTE_TYPE_FIELD, WireByte::new(remote_type));
                if remote_type != 0 {
                    updated &= config.set(
                        C::INPUT_TILT_ANGLE_LIMIT_FIELD,
                        WireByte::new(*input >> 2).scaled(1.0, 0.0, AngleDegrees::from_degrees),
                    );
                }
            }
        }
        if let Some(flags) = optional_input.get(2).copied() {
            let flags_updated = write_fields!(config;
                F::MOVING_FAULT_DISABLED_FIELD => flags & 0x01 != 0,
                C::FOOT_BEEP_ENABLED_FIELD => flags & 0x02 != 0,
            );
            let parking_updated = M::PARKING_BRAKE_MODE_FIELD
                .write(
                    config,
                    FloatOutBoyParkingBrakeMode::from((flags >> 2) & 0x03),
                )
                .is_some();
            updated = updated && flags_updated && parking_updated;
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
        write_fields!(config;
            B::BOOSTER_ANGLE_FIELD => tune_angle_from(booster_angle, AngleDegrees::from_degrees(5.0)),
            B::BOOSTER_RAMP_FIELD => tune_angle_from(booster_ramp, AngleDegrees::from_degrees(2.0)),
            B::BOOSTER_CURRENT_FIELD => tune_booster_current(booster_current),
            B::BRAKE_BOOSTER_ANGLE_FIELD => tune_angle_from(brake_angle, AngleDegrees::from_degrees(5.0)),
            B::BRAKE_BOOSTER_RAMP_FIELD => tune_angle_from(brake_ramp, AngleDegrees::from_degrees(2.0)),
            B::BRAKE_BOOSTER_CURRENT_FIELD => tune_booster_current(brake_current),
        )
    });
    if updated {
        state.alert_beeper(FloatOutBoyBeeperAlert::Short(1));
    }
    updated
}

#[cfg(test)]
mod tests;
