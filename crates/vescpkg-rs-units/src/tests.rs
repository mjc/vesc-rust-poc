use super::{
    AccelerationG, AngleDegrees, AngleRadians, AngularVelocity, Charge, Current, Distance,
    DistancePerEnergy, Energy, EnergyPerDistance, Frequency, Latitude, Longitude, Percent, Power,
    Ratio, Rpm, SampleRate, Speed, SystemTicks, Temperature, TimestampTicks, VescSeconds, Voltage,
};

#[test]
fn scalar_units_round_trip_through_named_accessors() {
    assert_eq!(Voltage::from_volts(50.5).as_volts(), 50.5);
    assert_eq!(Current::from_amps(12.25).as_amps(), 12.25);
    assert_eq!(Power::from_watts(600.0).as_watts(), 600.0);
    assert_eq!(Energy::from_watt_hours(42.0).as_watt_hours(), 42.0);
    assert_eq!(Charge::from_amp_hours(3.2).as_amp_hours(), 3.2);
    assert_eq!(
        Rpm::from_revolutions_per_minute(12_000.0).as_revolutions_per_minute(),
        12_000.0
    );
    assert_eq!(
        Speed::from_meters_per_second(4.5).as_meters_per_second(),
        4.5
    );
    assert_eq!(
        Temperature::from_degrees_celsius(23.0).as_degrees_celsius(),
        23.0
    );
    let latitude_degrees: f64 = Latitude::from_degrees(40.015).as_degrees();
    let longitude_degrees: f64 = Longitude::from_degrees(-105.2705).as_degrees();
    assert_eq!(latitude_degrees, 40.015);
    assert_eq!(longitude_degrees, -105.2705);
    assert_eq!(Frequency::from_hertz(1000.0).as_hertz(), 1000.0);
    assert_eq!(SampleRate::from_hertz(200.0).as_hertz(), 200.0);
    assert_eq!(VescSeconds::from_seconds(2.5).as_seconds(), 2.5);
    assert_eq!(AngleRadians::from_radians(1.5).as_radians(), 1.5);
    assert_eq!(AccelerationG::from_g(1.0).as_g(), 1.0);
    assert_eq!(
        AngularVelocity::from_degrees_per_second(90.0).as_degrees_per_second(),
        90.0
    );
}

#[test]
fn local_unit_conversions_stay_in_the_embedded_units_layer() {
    assert_eq!(Energy::from_watt_hours(2.0).as_joules(), 7200.0);
    assert_eq!(Energy::from_joules(7200.0).as_watt_hours(), 2.0);
    assert_eq!(Charge::from_amp_hours(3.2).as_amp_hours(), 3.2);
    assert_eq!(
        Speed::from_kilometers_per_hour(36.0).as_meters_per_second(),
        10.0
    );
    assert_eq!(Speed::from_miles_per_hour(60.0).as_miles_per_hour(), 60.0);
}

#[test]
fn angle_units_convert_between_degrees_and_radians_once_at_boundaries() {
    let right_angle_degrees = AngleDegrees::from_radians(core::f32::consts::FRAC_PI_2);
    let right_angle_radians = AngleRadians::from_degrees(90.0);

    assert!((right_angle_degrees.as_degrees() - 90.0).abs() < f32::EPSILON);
    assert!((right_angle_radians.as_radians() - core::f32::consts::FRAC_PI_2).abs() < f32::EPSILON);
    assert_eq!(
        AngleDegrees::from(right_angle_radians).as_degrees(),
        right_angle_degrees.as_degrees()
    );
    assert_eq!(
        AngleRadians::from(right_angle_degrees).as_radians(),
        right_angle_radians.as_radians()
    );
}

#[test]
fn angular_velocity_units_convert_between_degrees_and_radians_once_at_boundaries() {
    let right_angle_per_second =
        AngularVelocity::from_radians_per_second(core::f32::consts::FRAC_PI_2);

    assert!((right_angle_per_second.as_degrees_per_second() - 90.0).abs() < f32::EPSILON);
    assert!(
        (AngularVelocity::from_degrees_per_second(180.0).as_radians_per_second()
            - core::f32::consts::PI)
            .abs()
            < f32::EPSILON
    );
}

#[test]
fn angular_velocity_over_time_is_an_angle_in_radians() {
    let rate = AngularVelocity::from_radians_per_second(2.0);
    let duration = VescSeconds::from_seconds(0.25);

    assert!(((rate * duration).as_radians() - 0.5).abs() < f32::EPSILON);
    assert!(((duration * rate).as_radians() - 0.5).abs() < f32::EPSILON);
}

#[test]
fn sample_rate_reports_one_sample_period() {
    assert_eq!(
        SampleRate::from_hertz(200.0)
            .sample_period()
            .unwrap()
            .as_seconds(),
        0.005
    );
    assert_eq!(
        SampleRate::from_hertz(0.5)
            .sample_period()
            .unwrap()
            .as_seconds(),
        2.0
    );
    assert_eq!(SampleRate::from_hertz(0.0).sample_period(), None);
    assert_eq!(SampleRate::from_hertz(f32::NAN).sample_period(), None);
    assert_eq!(
        SampleRate::from_hertz(f32::from_bits(1)).sample_period(),
        None
    );
}

#[test]
fn scalar_units_support_same_unit_arithmetic_traits() {
    let angle = AngleDegrees::from_degrees(8.0) - AngleDegrees::from_degrees(3.0);
    let rate = -AngularVelocity::from_degrees_per_second(12.0);
    let current = Current::from_amps(10.0) * 0.25 + Current::from_amps(1.0);

    assert_eq!(angle.as_degrees(), 5.0);
    assert_eq!(rate.as_degrees_per_second(), -12.0);
    assert_eq!(current.as_amps(), 3.5);
    assert_eq!((current / 2.0).as_amps(), 1.75);
    assert_eq!((-angle).abs().as_degrees(), 5.0);
    assert_eq!(
        Current::from_amps(-0.0).abs().as_amps().to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(rate.signum(), -1.0);
    assert_eq!(Current::ZERO.signum(), 1.0);
    assert_eq!(Current::from_amps(-0.0).signum(), 1.0);
    assert_eq!(Current::from_amps(f32::NAN).signum(), 1.0);
    assert!(current.is_positive());
    assert!(Current::ZERO.is_zero());
    assert_eq!(
        Current::from_amps(4.0)
            .min(Current::from_amps(6.0))
            .as_amps(),
        4.0
    );
    assert_eq!(
        Current::from_amps(6.0)
            .min(Current::from_amps(f32::NAN))
            .as_amps(),
        6.0
    );
    assert_eq!(
        Current::from_amps(4.0)
            .max(Current::from_amps(f32::NAN))
            .as_amps(),
        4.0
    );
    assert_eq!(
        AngleDegrees::from_degrees(6.0) / AngleDegrees::from_degrees(3.0),
        2.0
    );
}

#[test]
fn efficiency_units_remain_generic_ratios_between_energy_and_distance() {
    let energy_per_distance = EnergyPerDistance::from_watt_hours_per_meter(12.5);
    let distance_per_energy = DistancePerEnergy::from_meters_per_watt_hour(0.08);

    assert_eq!(energy_per_distance.as_watt_hours_per_meter(), 12.5);
    assert_eq!(distance_per_energy.as_meters_per_watt_hour(), 0.08);
    assert_eq!(
        (Energy::from_watt_hours(25.0) / Distance::from_meters(2.0)).as_watt_hours_per_meter(),
        12.5
    );
    assert_eq!(
        (Distance::from_meters(2.0) / Energy::from_watt_hours(25.0)).as_meters_per_watt_hour(),
        0.08
    );
}

#[test]
fn energy_per_distance_uses_human_scale_ride_efficiency_units() {
    let city_efficiency = EnergyPerDistance::from_watt_hours_per_kilometer(18.0);
    let highway_efficiency = EnergyPerDistance::from_watt_hours_per_mile(32.0);

    assert_eq!(city_efficiency.as_watt_hours_per_kilometer(), 18.0);
    assert_eq!(city_efficiency.as_watt_hours_per_meter(), 0.018);
    assert_eq!(highway_efficiency.as_watt_hours_per_mile(), 32.0);
    assert_eq!(
        highway_efficiency.as_watt_hours_per_meter(),
        32.0 / 1609.344
    );
}

#[test]
fn energy_divided_by_distance_reports_ride_efficiency_without_manual_conversion() {
    let commute_energy = Energy::from_watt_hours(540.0);
    let commute_distance = Distance::from_meters(30_000.0);
    let efficiency = commute_energy / commute_distance;

    assert_eq!(efficiency.as_watt_hours_per_kilometer(), 18.0);
    assert!((efficiency.as_watt_hours_per_mile() - 28.968_191).abs() < 0.000_01);
}

#[test]
fn electrical_units_support_obvious_no_panic_arithmetic() {
    let voltage = Voltage::from_volts(50.4);
    let current = Current::from_amps(10.0);
    let power = voltage * current;

    assert_eq!(power.as_watts(), 504.0);
    assert_eq!((current * voltage).as_watts(), 504.0);
    assert_eq!((power / voltage).as_amps(), 10.0);
    assert_eq!((power / current).as_volts(), 50.4);
    assert_eq!((voltage / current).as_ohms(), 5.04);
}

#[test]
fn package_author_estimates_energy_from_pack_power_over_tick_window() {
    use super::prelude::*;

    let pack_voltage = Voltage::from_volts(50.4);
    let pack_current = Current::from_amps(10.0);
    let sample_window = SystemTicks::from_ticks(20_000);
    let energy = (pack_voltage * pack_current) * sample_window;

    assert!((energy.as_watt_hours() - (1008.0 / 3600.0)).abs() < 0.000_01);
}

#[test]
fn package_author_derives_speed_from_distance_and_tick_window() {
    use super::prelude::*;

    let distance = Distance::from_meters(42.0);
    let sample_window = SystemTicks::from_ticks(30_000);
    let speed = distance / sample_window;

    assert_eq!(speed.as_meters_per_second(), 14.0);
    assert_eq!((speed * sample_window).as_meters(), 42.0);
}

#[test]
fn package_author_reports_ride_efficiency_without_float_handoff() {
    use super::prelude::*;

    let energy = Energy::from_watt_hours(540.0);
    let distance = Distance::from_meters(30_000.0);
    let efficiency = energy / distance;

    assert_eq!(efficiency.as_watt_hours_per_kilometer(), 18.0);
}

#[test]
fn package_author_uses_known_good_bounded_constants_for_commands() {
    use super::prelude::*;

    const DUTY_LIMIT: Ratio = Ratio::from_ratio_const(0.85);
    const REGEN_LIMIT: SignedRatio = SignedRatio::from_ratio_const(-0.25);
    const BATTERY_WARNING: Percent = Percent::from_percent_const(20.0);

    assert_eq!(DUTY_LIMIT.as_ratio(), 0.85);
    assert_eq!(REGEN_LIMIT.as_ratio(), -0.25);
    assert_eq!(BATTERY_WARNING.as_percent(), 20.0);
}

#[test]
fn package_author_keeps_trip_math_typed_until_display() {
    use super::prelude::*;

    let cruise_speed = Speed::from_kilometers_per_hour(36.0);
    let elapsed = SystemTicks::from_ticks(60_000);
    let distance = cruise_speed * elapsed;
    let energy = Power::from_watts(180.0) * elapsed;
    let efficiency = energy / distance;

    assert!((distance.as_meters() - 60.0).abs() < 0.000_01);
    assert!((energy.as_watt_hours() - (3.0 / 10.0)).abs() < 0.000_01);
    assert!((efficiency.as_watt_hours_per_kilometer() - 5.0).abs() < 0.000_01);
}

#[test]
fn bounded_units_reject_out_of_range_values() {
    assert_eq!(Ratio::from_ratio(0.5).expect("valid").as_ratio(), 0.5);

    let low = Ratio::from_ratio(-0.1).expect_err("too low");
    assert_eq!(low.value(), -0.1);
    assert_eq!(low.min(), 0.0);
    assert_eq!(low.max(), 1.0);

    let high = Percent::from_percent(101.0).expect_err("too high");
    assert_eq!(high.value(), 101.0);
    assert_eq!(high.min(), 0.0);
    assert_eq!(high.max(), 100.0);
}

#[test]
fn bounded_units_support_known_good_package_constants() {
    const CENTER_DUTY: Ratio = Ratio::from_ratio_const(0.5);
    const FULL_SCALE: Percent = Percent::from_percent_const(100.0);

    assert_eq!(CENTER_DUTY.as_ratio(), 0.5);
    assert_eq!(FULL_SCALE.as_percent(), 100.0);
}

#[test]
#[should_panic(expected = "invalid normalized ratio constant")]
fn bounded_unit_constant_constructor_rejects_runtime_misuse() {
    let _ = Ratio::from_ratio_const(2.0);
}

#[test]
fn bounded_units_clamp_without_panicking() {
    assert_eq!(Ratio::clamped(-1.0).as_ratio(), 0.0);
    assert_eq!(Ratio::clamped(2.0).as_ratio(), 1.0);
    assert_eq!(Ratio::clamped(0.25).as_ratio(), 0.25);
    assert_eq!(Ratio::clamped(f32::NAN).as_ratio(), 0.0);
}

#[test]
fn fugit_timer_aliases_model_vesc_system_ticks() {
    let ticks = SystemTicks::from_ticks(10_000);

    assert_eq!(ticks.as_ticks(), 10_000);
    assert_eq!(ticks.as_millis(), 1_000);
}

#[test]
fn timestamp_delta_preserves_vesc_unsigned_wraparound() {
    let then = TimestampTicks::from_ticks(u32::MAX - 4);
    let now = TimestampTicks::from_ticks(5);

    assert_eq!(now.wrapping_duration_since(then).as_ticks(), 10);
}

#[test]
fn system_tick_duration_arithmetic_stays_typed() {
    let two_seconds = SystemTicks::from_ticks(20_000);

    assert_eq!(
        (Speed::from_meters_per_second(2.0) * two_seconds).as_meters(),
        4.0
    );
    assert_eq!(
        (Distance::from_meters(4.0) / two_seconds).as_meters_per_second(),
        2.0
    );
    assert_eq!(
        (Power::from_watts(100.0) * two_seconds).as_watt_hours(),
        200.0 / 3600.0
    );
}
