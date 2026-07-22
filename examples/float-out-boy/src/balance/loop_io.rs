use crate::domain::{
    FloatOutBoyDarkRideState, FloatOutBoyMode, FloatOutBoyRealtimeRuntimeSetpoint,
};
use vescpkg_rs::prelude::{
    AngleCurrentGain, AngleDegrees, AngularVelocity, Current, ElectricalSpeed, ImuRoll,
    IntegralCurrentGain, MotorCurrent, MotorCurrentLimit, PidScale, RateCurrentGain, SampleRate,
};

/// Config inputs consumed by one Float Out Boy RUNNING balance-current step.
///
/// Source map: upstream reads these values from `FloatOutBoyConfig` in
/// `third_party/float-out-boy/src/pid.c:37-73`,
/// `third_party/float-out-boy/src/booster.c:32-75`, and
/// `third_party/float-out-boy/src/main.c:924-942`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopConfig {
    pub(crate) kp: AngleCurrentGain,
    pub(crate) kp2: RateCurrentGain,
    pub(crate) ki: IntegralCurrentGain,
    pub(crate) kp_brake: PidScale,
    pub(crate) kp2_brake: PidScale,
    pub(crate) ki_limit: MotorCurrentLimit,
    pub(crate) booster_angle: AngleDegrees,
    pub(crate) booster_ramp: AngleDegrees,
    pub(crate) booster_current: MotorCurrent,
    pub(crate) brkbooster_angle: AngleDegrees,
    pub(crate) brkbooster_ramp: AngleDegrees,
    pub(crate) brkbooster_current: MotorCurrent,
    pub(crate) hertz: SampleRate,
}

/// Runtime inputs consumed by one Float Out Boy RUNNING balance-current step.
///
/// Source map: upstream combines setpoint, IMU, motor, mode, darkride, and
/// traction state in `third_party/float-out-boy/src/main.c:918-956`; pitch-rate input
/// is prepared by `third_party/float-out-boy/src/imu.c:43-53`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopInput {
    pub(crate) setpoint: FloatOutBoyRealtimeRuntimeSetpoint,
    pub(crate) brake_tilt_setpoint: FloatOutBoyRealtimeRuntimeSetpoint,
    pub(crate) balance_pitch: AngleDegrees,
    pub(crate) raw_pitch: AngleDegrees,
    pub(crate) roll: ImuRoll,
    pub(crate) gyro_pitch: AngularVelocity,
    pub(crate) gyro_yaw: AngularVelocity,
    pub(crate) motor_erpm: ElectricalSpeed,
    pub(crate) motor_current: MotorCurrent,
    pub(crate) motor_current_max: MotorCurrentLimit,
    pub(crate) motor_current_min: MotorCurrentLimit,
    pub(crate) mode: FloatOutBoyMode,
    pub(crate) darkride: FloatOutBoyDarkRideState,
    pub(crate) traction_control: bool,
}

/// Mutable PID state for one Float Out Boy balance-current step.
///
/// Source map: upstream stores these fields together in `Data.pid` and updates
/// them at `third_party/float-out-boy/src/pid.c:31-73`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PidState {
    pub(crate) integral_current: MotorCurrent,
    pub(crate) kp_brake_scale: PidScale,
    pub(crate) kp2_brake_scale: PidScale,
    pub(crate) kp_accel_scale: PidScale,
    pub(crate) kp2_accel_scale: PidScale,
}

impl PidState {
    pub(crate) fn source_startup() -> Self {
        Self {
            integral_current: MotorCurrent::new(Current::ZERO),
            kp_brake_scale: PidScale::new(1.0),
            kp2_brake_scale: PidScale::new(1.0),
            kp_accel_scale: PidScale::new(1.0),
            kp2_accel_scale: PidScale::new(1.0),
        }
    }
}

/// Mutable control-loop state surrounding Float Out Boy's PID state.
///
/// Source map: upstream stores these fields in `Data.pid`, `Data.booster`,
/// `Data.softstart_pid_limit`, and `Data.balance_current` while running
/// `third_party/float-out-boy/src/main.c:924-954`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopState {
    pub(crate) balance_current: MotorCurrent,
    pub(crate) booster_current: MotorCurrent,
    pub(crate) pid: PidState,
    pub(crate) softstart_pid_limit: MotorCurrent,
}

impl LoopState {
    /// Initial Float Out Boy balance-loop state after package startup.
    pub(crate) fn source_startup() -> Self {
        let zero_current = MotorCurrent::new(Current::ZERO);
        Self {
            balance_current: zero_current,
            booster_current: zero_current,
            pid: PidState::source_startup(),
            softstart_pid_limit: zero_current,
        }
    }

    /// Reset transient PID state like upstream `pid_init`.
    pub(crate) fn reset_pid(&mut self) {
        self.pid = PidState::source_startup();
    }
}

/// Result of one Float Out Boy RUNNING balance-current step.
///
/// Source map: upstream stores `d->balance_current` and immediately requests it
/// from motor control at `third_party/float-out-boy/src/main.c:949-956`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopOutput {
    pub(crate) state: LoopState,
    pub(crate) requested_current: MotorCurrent,
}
