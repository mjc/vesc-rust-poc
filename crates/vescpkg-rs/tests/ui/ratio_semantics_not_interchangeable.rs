use vescpkg_rs::{DutyCycle, Pwm, SignedRatio};

fn set_pwm(_: Pwm) {}

fn main() {
    let duty = DutyCycle::new(SignedRatio::from_ratio(-0.25).expect("valid duty command"));

    set_pwm(duty);
}
