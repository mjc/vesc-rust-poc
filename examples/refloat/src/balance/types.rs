use crate::domain::{
    RefloatDarkRideState, RefloatMode, RefloatRealtimeBalancePitch, RefloatRealtimeRuntimeSetpoint,
};
use vescpkg_rs::prelude::{
    AngleDegrees, AngularVelocity, ElectricalSpeed, ImuPitch, ImuRoll, MotorCurrent, SampleRate,
};

/// Config inputs consumed by one Refloat RUNNING balance-current step.
///
/// Source map: upstream reads these values from `RefloatConfig` in
/// `third_party/refloat/src/pid.c:37-73`,
/// `third_party/refloat/src/booster.c:32-75`, and
/// `third_party/refloat/src/main.c:924-942`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopConfig {
    pub(crate) kp: f32,
    pub(crate) kp2: f32,
    pub(crate) ki: f32,
    pub(crate) kp_brake: f32,
    pub(crate) kp2_brake: f32,
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
pub(crate) struct RefloatBalanceLoopInput {
    pub(crate) setpoint: RefloatRealtimeRuntimeSetpoint,
    pub(crate) brake_tilt_setpoint: RefloatRealtimeRuntimeSetpoint,
    pub(crate) balance_pitch: RefloatRealtimeBalancePitch,
    pub(crate) raw_pitch: ImuPitch,
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
pub(crate) struct RefloatBalanceLoopState {
    pub(crate) balance_current: MotorCurrent,
    pub(crate) booster_current: MotorCurrent,
    pub(crate) pid_integral_current: MotorCurrent,
    pub(crate) pid_kp_brake_scale: f32,
    pub(crate) pid_kp2_brake_scale: f32,
    pub(crate) pid_kp_accel_scale: f32,
    pub(crate) pid_kp2_accel_scale: f32,
    pub(crate) softstart_pid_limit: MotorCurrent,
}

/// Result of one Refloat RUNNING balance-current step.
///
/// Source map: upstream stores `d->balance_current` and immediately requests it
/// from motor control at `third_party/refloat/src/main.c:949-956`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatBalanceLoopOutput {
    pub(crate) state: RefloatBalanceLoopState,
    pub(crate) requested_current: MotorCurrent,
}
