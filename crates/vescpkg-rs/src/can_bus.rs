//! Safe CAN transport and status snapshots.

use crate::types::{
    AmpHoursCharged, AmpHoursDischarged, CanControllerId, CanExtendedId, CanPayloadLen,
    CanStandardId, CurrentRelative, DutyCycle, ElectricalSpeed, MotorCurrent, PidPosition,
    WattHoursCharged, WattHoursDischarged,
};
use crate::units::{Charge, Current, Energy, Rpm, SignedRatio, TimestampTicks};

/// Failure returned by a CAN operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanError {
    /// The firmware table does not expose the requested CAN operation.
    Unsupported,
    /// A classic CAN frame contained more than eight payload bytes.
    PayloadTooLong,
    /// The remote controller did not answer the ping request.
    PingFailed,
}

impl core::fmt::Display for CanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "firmware does not expose this CAN operation",
            Self::PayloadTooLong => "CAN payload exceeds eight bytes",
            Self::PingFailed => "remote CAN controller did not answer",
        })
    }
}

/// Hardware family reported by a remote VESC controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanHardwareType {
    /// Standard VESC motor controller.
    Vesc,
    /// VESC BMS controller.
    VescBms,
    /// Custom VESC module.
    CustomModule,
    /// Firmware-specific hardware type not known by this SDK.
    Unknown(i32),
}

impl CanHardwareType {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Vesc,
            1 => Self::VescBms,
            2 => Self::CustomModule,
            value => Self::Unknown(value),
        }
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

/// A copied snapshot of CAN status message 2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus2 {
    controller: CanControllerId,
    amp_hours_discharged: AmpHoursDischarged,
    amp_hours_charged: AmpHoursCharged,
}

/// A copied snapshot of CAN status message 3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus3 {
    controller: CanControllerId,
    watt_hours_discharged: WattHoursDischarged,
    watt_hours_charged: WattHoursCharged,
}

impl CanStatus3 {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return discharged watt-hours reported by the remote controller.
    pub const fn watt_hours_discharged(self) -> WattHoursDischarged {
        self.watt_hours_discharged
    }

    /// Return charged watt-hours reported by the remote controller.
    pub const fn watt_hours_charged(self) -> WattHoursCharged {
        self.watt_hours_charged
    }
}

impl CanStatus2 {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return discharged amp-hours reported by the remote controller.
    pub const fn amp_hours_discharged(self) -> AmpHoursDischarged {
        self.amp_hours_discharged
    }

    /// Return charged amp-hours reported by the remote controller.
    pub const fn amp_hours_charged(self) -> AmpHoursCharged {
        self.amp_hours_charged
    }
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

    /// Ping a remote controller and return its reported hardware family.
    pub fn ping(&self, controller: CanControllerId) -> Result<CanHardwareType, CanError> {
        let (ok, hardware) =
            unsafe { crate::ffi::can_ping(controller.as_u8()) }.ok_or(CanError::Unsupported)?;
        ok.then(|| CanHardwareType::from_raw(hardware.0))
            .ok_or(CanError::PingFailed)
    }

    /// Transmit a standard 11-bit CAN frame.
    pub fn transmit_standard(&self, id: CanStandardId, payload: &[u8]) -> Result<(), CanError> {
        let len = CanPayloadLen::try_new(u8::try_from(payload.len()).unwrap_or(u8::MAX))
            .map_err(|_| CanError::PayloadTooLong)?
            .as_u8();
        (unsafe { crate::ffi::can_transmit_sid(u32::from(id.as_u16()), payload.as_ptr(), len) }
            .is_some())
        .then_some(())
        .ok_or(CanError::Unsupported)
    }

    /// Transmit an extended 29-bit CAN frame.
    pub fn transmit_extended(&self, id: CanExtendedId, payload: &[u8]) -> Result<(), CanError> {
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

    /// Send a remote motor relative-current command.
    pub fn set_current_relative(
        &self,
        controller: CanControllerId,
        current: CurrentRelative,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_current_rel(controller.as_u8(), current.ratio().as_ratio()) }
            .map(|_| ())
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor duty command.
    pub fn set_duty(&self, controller: CanControllerId, duty: DutyCycle) -> Result<(), CanError> {
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
            crate::ffi::can_set_rpm(controller.as_u8(), rpm.rpm().as_revolutions_per_minute())
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
                SignedRatio::from_ratio(raw.duty).unwrap_or(SignedRatio::from_ratio_const(0.0)),
            ),
        })
    }

    /// Copy CAN status message 2 for one remote controller.
    pub fn status2(&self, controller: CanControllerId) -> Option<CanStatus2> {
        let raw = unsafe { crate::ffi::can_status_msg_2_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus2 {
            controller,
            amp_hours_discharged: AmpHoursDischarged::new(Charge::from_amp_hours(raw.amp_hours)),
            amp_hours_charged: AmpHoursCharged::new(Charge::from_amp_hours(raw.amp_hours_charged)),
        })
    }

    /// Copy CAN status message 3 for one remote controller.
    pub fn status3(&self, controller: CanControllerId) -> Option<CanStatus3> {
        let raw = unsafe { crate::ffi::can_status_msg_3_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus3 {
            controller,
            watt_hours_discharged: WattHoursDischarged::new(Energy::from_watt_hours(
                raw.watt_hours,
            )),
            watt_hours_charged: WattHoursCharged::new(Energy::from_watt_hours(
                raw.watt_hours_charged,
            )),
        })
    }
}
