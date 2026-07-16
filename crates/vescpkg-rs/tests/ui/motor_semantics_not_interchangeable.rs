use vescpkg_rs::{
    BrakeCurrent, Current, DCurrent, DutyCycle, InputCurrent, MotorCurrent, SignedRatio,
    TotalMotorCurrent,
};

fn total_motor_current(_: TotalMotorCurrent) {}
fn input_current(_: InputCurrent) {}
fn d_current(_: DCurrent) {}
fn duty_cycle(_: DutyCycle) {}
fn brake_current(_: BrakeCurrent) {}

fn main() {
    let current = MotorCurrent::new(Current::from_amps(1.0));

    total_motor_current(current);
    input_current(current);
    d_current(current);
    duty_cycle(SignedRatio::from_ratio_const(0.5));
    brake_current(current);
}
