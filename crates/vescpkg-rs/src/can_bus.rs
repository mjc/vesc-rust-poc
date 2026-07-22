//! Safe CAN transport and status snapshots.

use crate::types::{
    CanControllerId, CanExtendedId, CanPayloadLen, CanStandardId, DutyCycle, ElectricalSpeed,
    MotorCurrent, PidPosition,
};
use crate::units::{Current, Rpm, SignedRatio, TimestampTicks};

/// Failure returned by a CAN operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanError {
    /// The firmware table does not expose the requested CAN operation.
    Unsupported,
    /// A classic CAN frame contained more than eight payload bytes.
    PayloadTooLong,
}

impl core::fmt::Display for CanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "firmware does not expose this CAN operation",
            Self::PayloadTooLong => "CAN payload exceeds eight bytes",
        })
    }
}

impl core::error::Error for CanError {}

/// A copied snapshot of the firmware's primary CAN status record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus {
    controller: CanControllerId,
    received_at: TimestampTicks,
    electrical_speed: ElectricalSpeed,
    motor_current: MotorCurrent,
    duty_cycle: DutyCycle,
}

impl CanStatus {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return the firmware timestamp associated with the record.
    pub const fn received_at(self) -> TimestampTicks {
        self.received_at
    }

    /// Return the remote motor's electrical speed.
    pub const fn electrical_speed(self) -> ElectricalSpeed {
        self.electrical_speed
    }

    /// Return the remote motor current.
    pub const fn motor_current(self) -> MotorCurrent {
        self.motor_current
    }

    /// Return the remote motor duty cycle.
    pub const fn duty_cycle(self) -> DutyCycle {
        self.duty_cycle
    }
}

/// Safe access to the firmware CAN table.
pub struct CanBus;

impl CanBus {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Transmit a standard 11-bit CAN frame.
    pub fn transmit_standard(
        &self,
        id: CanStandardId,
        payload: &[u8],
    ) -> Result<(), CanError> {
        let len = CanPayloadLen::try_new(u8::try_from(payload.len()).unwrap_or(u8::MAX))
            .map_err(|_| CanError::PayloadTooLong)?
            .as_u8();
        (unsafe { crate::ffi::can_transmit_sid(u32::from(id.as_u16()), payload.as_ptr(), len) }
            .is_some())
        .then_some(())
        .ok_or(CanError::Unsupported)
    }

    /// Transmit an extended 29-bit CAN frame.
    pub fn transmit_extended(
        &self,
        id: CanExtendedId,
        payload: &[u8],
    ) -> Result<(), CanError> {
        let len = CanPayloadLen::try_new(u8::try_from(payload.len()).unwrap_or(u8::MAX))
            .map_err(|_| CanError::PayloadTooLong)?
            .as_u8();
        (unsafe { crate::ffi::can_transmit_eid(id.as_u32(), payload.as_ptr(), len) }.is_some())
            .then_some(())
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor current command.
    pub fn set_current(
        &self,
        controller: CanControllerId,
        current: MotorCurrent,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_current(controller.as_u8(), current.current().as_amps()) }
            .map(|_| ())
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor duty command.
    pub fn set_duty(
        &self,
        controller: CanControllerId,
        duty: DutyCycle,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_duty(controller.as_u8(), duty.ratio().as_ratio()) }
            .map(|_| ())
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor electrical-speed command.
    pub fn set_rpm(
        &self,
        controller: CanControllerId,
        rpm: ElectricalSpeed,
    ) -> Result<(), CanError> {
        unsafe {
            crate::ffi::can_set_rpm(
                controller.as_u8(),
                rpm.rpm().as_revolutions_per_minute(),
            )
        }
        .map(|_| ())
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor position command.
    pub fn set_position(
        &self,
        controller: CanControllerId,
        position: PidPosition,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_pos(controller.as_u8(), position.angle().as_degrees()) }
            .map(|_| ())
            .ok_or(CanError::Unsupported)
    }

    /// Copy the primary status record for one remote controller.
    pub fn status(&self, controller: CanControllerId) -> Option<CanStatus> {
        let raw = unsafe { crate::ffi::can_status_msg_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus {
            controller,
            received_at: TimestampTicks::from_ticks(raw.rx_time),
            electrical_speed: ElectricalSpeed::new(Rpm::from_revolutions_per_minute(raw.rpm)),
            motor_current: MotorCurrent::new(Current::from_amps(raw.current)),
            duty_cycle: DutyCycle::new(
                SignedRatio::from_ratio(raw.duty)
                    .unwrap_or(SignedRatio::from_ratio_const(0.0)),
            ),
        })
    }
}
