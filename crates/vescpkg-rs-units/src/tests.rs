use super::{
    AccelerationG, AmpHours, AngleRadians, AngularVelocity, Charge, Current, Distance,
    DistancePerEnergy, Energy, EnergyPerDistance, Frequency, Latitude, Longitude, Percent, Power,
    Ratio, Rpm, SampleRate, Seconds, Speed, SystemTicks, Temperature, Voltage, WattHours,
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
    assert_eq!(Seconds::from_seconds(2.5).as_seconds(), 2.5);
    assert_eq!(AngleRadians::from_radians(1.5).as_radians(), 1.5);
    assert_eq!(AccelerationG::from_g(1.0).as_g(), 1.0);
    assert_eq!(
        AngularVelocity::from_degrees_per_second(90.0).as_degrees_per_second(),
        90.0
    );
}

#[test]
fn unit_name_energy_and_charge_aliases_are_compatibility_names() {
    let stored_energy: Energy = WattHours::from_watt_hours(70.0);
    let legacy_energy_name: WattHours = Energy::from_joules(7200.0);
    let stored_charge: Charge = AmpHours::from_amp_hours(3.2);
    let legacy_charge_name: AmpHours = Charge::from_amp_hours(1.25);

    assert_eq!(stored_energy.as_watt_hours(), 70.0);
    assert_eq!(legacy_energy_name.as_watt_hours(), 2.0);
    assert_eq!(stored_charge.as_amp_hours(), 3.2);
    assert_eq!(legacy_charge_name.as_amp_hours(), 1.25);
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
