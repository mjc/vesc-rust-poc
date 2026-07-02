use vescpkg_rs::types::{DutyCycle, Pwm};
use vescpkg_rs::units::SignedRatio;

fn set_pwm(_: Pwm) {}

fn main() {
    let duty = DutyCycle::new(SignedRatio::from_ratio(-0.25).expect("valid duty command"));

    set_pwm(duty);
}
