use super::*;

#[test]
fn float_out_boy_run_states_map_to_shared_motor_control_states() {
    assert_eq!(
        MotorControlRunState::from(FloatOutBoyRunState::Disabled),
        MotorControlRunState::Disabled
    );
    assert_eq!(
        MotorControlRunState::from(FloatOutBoyRunState::Startup),
        MotorControlRunState::Idle
    );
    assert_eq!(
        MotorControlRunState::from(FloatOutBoyRunState::Ready),
        MotorControlRunState::Idle
    );
    assert_eq!(
        MotorControlRunState::from(FloatOutBoyRunState::Running),
        MotorControlRunState::Running
    );
}
