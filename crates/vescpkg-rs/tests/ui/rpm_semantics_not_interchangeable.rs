use vescpkg_rs::types::{ElectricalSpeed, MechanicalSpeed};
use vescpkg_rs::units::Rpm;

fn set_electrical_speed(_: ElectricalSpeed) {}

fn main() {
    let mechanical = MechanicalSpeed::new(Rpm::from_revolutions_per_minute(3000.0));

    set_electrical_speed(mechanical);
}
