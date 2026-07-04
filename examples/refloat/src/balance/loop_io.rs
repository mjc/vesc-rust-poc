use crate::domain::{RefloatDarkRideState, RefloatMode, RefloatRealtimeRuntimeSetpoint};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, ImuRoll,
    IntegralCurrentGain, MotorCurrent, PidScale, RateCurrentGain, SampleRate,
};

/// Config inputs consumed by one Refloat RUNNING balance-current step.
///
/// Source map: upstream reads these values from `RefloatConfig` in
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/main.c:924-942`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopConfig {
    pub(crate) kp: AngleCurrentGain,
    pub(crate) kp2: RateCurrentGain,
    pub(crate) ki: IntegralCurrentGain,
    pub(crate) kp_brake: PidScale,
    pub(crate) kp2_brake: PidScale,
    pub(crate) ki_limit: MotorCurrent,
    pub(crate) booster_angle: AngleDegrees,
    pub(crate) booster_ramp: AngleDegrees,
    pub(crate) booster_current: MotorCurrent,
    pub(crate) brkbooster_angle: AngleDegrees,
    pub(crate) brkbooster_ramp: AngleDegrees,
    pub(crate) brkbooster_current: MotorCurrent,
    pub(crate) hertz: SampleRate,
}

/// Runtime inputs consumed by one Refloat RUNNING balance-current step.
///
/// Source map: upstream combines setpoint, IMU, motor, mode, darkride, and
/// traction state in `third_party/refloat/src/main.c:918-956`; pitch-rate input
/// is prepared by `third_party/refloat/src/imu.c:43-53`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopInput {
    pub(crate) setpoint: RefloatRealtimeRuntimeSetpoint,
    pub(crate) brake_tilt_setpoint: RefloatRealtimeRuntimeSetpoint,
    pub(crate) balance_pitch: AngleDegrees,
    pub(crate) raw_pitch: AngleDegrees,
    pub(crate) roll: ImuRoll,
    pub(crate) gyro_pitch: AngularVelocity,
    pub(crate) gyro_yaw: AngularVelocity,
    pub(crate) motor_erpm: ElectricalSpeed,
    pub(crate) motor_current: MotorCurrent,
    pub(crate) motor_current_max: MotorCurrent,
    pub(crate) motor_current_min: MotorCurrent,
    pub(crate) mode: RefloatMode,
    pub(crate) darkride: RefloatDarkRideState,
    pub(crate) traction_control: bool,
}

/// Mutable PID/booster/current state for one Refloat balance-current step.
///
/// Source map: upstream stores these fields in `Data.pid`, `Data.booster`,
/// `Data.softstart_pid_limit`, and `Data.balance_current` while running
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/main.c:924-954`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopState {
    pub(crate) balance_current: MotorCurrent,
    pub(crate) booster_current: MotorCurrent,
    pub(crate) pid_integral_current: MotorCurrent,
    pub(crate) pid_kp_brake_scale: PidScale,
    pub(crate) pid_kp2_brake_scale: PidScale,
    pub(crate) pid_kp_accel_scale: PidScale,
    pub(crate) pid_kp2_accel_scale: PidScale,
    pub(crate) softstart_pid_limit: MotorCurrent,
}

impl LoopState {
    /// Initial Refloat balance-loop state after package startup.
    pub(crate) fn source_startup() -> Self {
        let zero_current = MotorCurrent::new(Current::ZERO);
        Self {
            balance_current: zero_current,
            booster_current: zero_current,
            pid_integral_current: zero_current,
            pid_kp_brake_scale: PidScale::new(1.0),
            pid_kp2_brake_scale: PidScale::new(1.0),
            pid_kp_accel_scale: PidScale::new(1.0),
            pid_kp2_accel_scale: PidScale::new(1.0),
            softstart_pid_limit: zero_current,
        }
    }
}

/// Result of one Refloat RUNNING balance-current step.
///
/// Source map: upstream stores `d->balance_current` and immediately requests it
/// from motor control at `third_party/refloat/src/main.c:949-956`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopOutput {
    pub(crate) state: LoopState,
    pub(crate) requested_current: MotorCurrent,
}
