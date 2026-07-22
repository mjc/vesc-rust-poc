//! Safe CAN transport and status snapshots.

use crate::types::{
    AdcVoltage, AmpHoursCharged, AmpHoursDischarged, BrakeCurrent, BrakeCurrentRelative,
    CanControllerId, CanExtendedId, CanPayloadLen, CanStandardId, CurrentOffDelay, CurrentRelative,
    DutyCycle, ElectricalSpeed, InputCurrent, InputVoltage, MosfetTemperature, MotorCurrent,
    MotorTemperature, PidPosition, PpmInput, TachometerSteps, WattHoursCharged,
    WattHoursDischarged,
};
use crate::units::{Charge, Current, Energy, Rpm, SignedRatio, SystemTicks, TimestampTicks};
use core::sync::atomic::{AtomicBool, Ordering};

/// Callback ABI used by VESC standard/extended CAN receive slots.
pub type CanReceiverCallback = unsafe extern "C" fn(u32, *mut u8, u8) -> bool;

static SID_RECEIVER_REGISTERED: AtomicBool = AtomicBool::new(false);
static EID_RECEIVER_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Failure returned by a CAN operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanError {
    /// The firmware table does not expose the requested CAN operation.
    Unsupported,
    /// A classic CAN frame contained more than eight payload bytes.
    PayloadTooLong,
    /// The remote controller did not answer the ping request.
    PingFailed,
    /// A receiver callback of this kind is already registered.
    ReceiverBusy,
}

impl core::fmt::Display for CanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "firmware does not expose this CAN operation",
            Self::PayloadTooLong => "CAN payload exceeds eight bytes",
            Self::PingFailed => "remote CAN controller did not answer",
            Self::ReceiverBusy => "a CAN receiver callback is already registered",
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

/// A copied snapshot of CAN status message 4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus4 {
    controller: CanControllerId,
    fet_temperature: MosfetTemperature,
    motor_temperature: MotorTemperature,
    input_current: InputCurrent,
    position: PidPosition,
}

/// A copied snapshot of CAN status message 5.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus5 {
    controller: CanControllerId,
    input_voltage: InputVoltage,
    tachometer: TachometerSteps,
}

/// A copied snapshot of CAN status message 6.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanStatus6 {
    controller: CanControllerId,
    adc_1: AdcVoltage,
    adc_2: AdcVoltage,
    adc_3: AdcVoltage,
    ppm: PpmInput,
}

impl CanStatus6 {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return ADC channel 1.
    pub const fn adc1(self) -> AdcVoltage {
        self.adc_1
    }

    /// Return ADC channel 2.
    pub const fn adc2(self) -> AdcVoltage {
        self.adc_2
    }

    /// Return ADC channel 3.
    pub const fn adc3(self) -> AdcVoltage {
        self.adc_3
    }

    /// Return the decoded PPM input ratio.
    pub const fn ppm(self) -> PpmInput {
        self.ppm
    }
}

impl CanStatus5 {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return the remote controller input voltage.
    pub const fn input_voltage(self) -> InputVoltage {
        self.input_voltage
    }

    /// Return the remote tachometer position.
    pub const fn tachometer(self) -> TachometerSteps {
        self.tachometer
    }
}

impl CanStatus4 {
    /// Return the controller whose status was queried.
    pub const fn controller(self) -> CanControllerId {
        self.controller
    }

    /// Return the remote FET temperature.
    pub const fn fet_temperature(self) -> MosfetTemperature {
        self.fet_temperature
    }

    /// Return the remote motor temperature.
    pub const fn motor_temperature(self) -> MotorTemperature {
        self.motor_temperature
    }

    /// Return the remote input current.
    pub const fn input_current(self) -> InputCurrent {
        self.input_current
    }

    /// Return the remote PID position.
    pub const fn position(self) -> PidPosition {
        self.position
    }
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

    /// Return the wrapping age of this snapshot at a current firmware tick.
    pub const fn age_at(self, now: TimestampTicks) -> SystemTicks {
        now.wrapping_duration_since(self.received_at)
    }

    /// Return whether this snapshot is older than the supplied tick budget.
    pub fn is_stale(self, now: TimestampTicks, max_age: SystemTicks) -> bool {
        self.age_at(now) > max_age
    }
}

/// Safe access to the firmware CAN table.
pub struct CanBus;

/// Owns one CAN receiver registration and unregisters it on drop.
pub struct CanReceiverGuard {
    extended: bool,
}

impl Drop for CanReceiverGuard {
    fn drop(&mut self) {
        let cleared = if self.extended {
            unsafe { crate::ffi::can_set_eid_callback(None) }
        } else {
            unsafe { crate::ffi::can_set_sid_callback(None) }
        };
        if cleared.is_some() {
            if self.extended {
                EID_RECEIVER_REGISTERED.store(false, Ordering::Release);
            } else {
                SID_RECEIVER_REGISTERED.store(false, Ordering::Release);
            }
        }
    }
}

impl CanBus {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Register the single standard-ID receive callback exposed by VESC.
    pub fn register_standard_receiver(
        &self,
        callback: CanReceiverCallback,
    ) -> Result<CanReceiverGuard, CanError> {
        SID_RECEIVER_REGISTERED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| CanError::ReceiverBusy)?;
        if unsafe { crate::ffi::can_set_sid_callback(Some(callback)) }.is_none() {
            SID_RECEIVER_REGISTERED.store(false, Ordering::Release);
            return Err(CanError::Unsupported);
        }
        Ok(CanReceiverGuard { extended: false })
    }

    /// Register the single extended-ID receive callback exposed by VESC.
    pub fn register_extended_receiver(
        &self,
        callback: CanReceiverCallback,
    ) -> Result<CanReceiverGuard, CanError> {
        EID_RECEIVER_REGISTERED
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| CanError::ReceiverBusy)?;
        if unsafe { crate::ffi::can_set_eid_callback(Some(callback)) }.is_none() {
            EID_RECEIVER_REGISTERED.store(false, Ordering::Release);
            return Err(CanError::Unsupported);
        }
        Ok(CanReceiverGuard { extended: true })
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
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor relative-current command.
    pub fn set_current_relative(
        &self,
        controller: CanControllerId,
        current: CurrentRelative,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_current_rel(controller.as_u8(), current.ratio().as_ratio()) }
            .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor duty command.
    pub fn set_duty(&self, controller: CanControllerId, duty: DutyCycle) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_duty(controller.as_u8(), duty.ratio().as_ratio()) }
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
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor relative-current command with an off-delay.
    pub fn set_current_relative_off_delay(
        &self,
        controller: CanControllerId,
        current: CurrentRelative,
        delay: CurrentOffDelay,
    ) -> Result<(), CanError> {
        unsafe {
            crate::ffi::can_set_current_rel_off_delay(
                controller.as_u8(),
                current.ratio().as_ratio(),
                delay.duration().as_seconds(),
            )
        }
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor brake-current command.
    pub fn set_brake_current(
        &self,
        controller: CanControllerId,
        current: BrakeCurrent,
    ) -> Result<(), CanError> {
        unsafe {
            crate::ffi::can_set_current_brake(controller.as_u8(), current.current().as_amps())
        }
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor relative brake-current command.
    pub fn set_brake_current_relative(
        &self,
        controller: CanControllerId,
        current: BrakeCurrentRelative,
    ) -> Result<(), CanError> {
        unsafe {
            crate::ffi::can_set_current_brake_rel(controller.as_u8(), current.ratio().as_ratio())
        }
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor current command with an off-delay.
    pub fn set_current_off_delay(
        &self,
        controller: CanControllerId,
        current: MotorCurrent,
        delay: CurrentOffDelay,
    ) -> Result<(), CanError> {
        unsafe {
            crate::ffi::can_set_current_off_delay(
                controller.as_u8(),
                current.current().as_amps(),
                delay.duration().as_seconds(),
            )
        }
        .ok_or(CanError::Unsupported)
    }

    /// Send a remote motor position command.
    pub fn set_position(
        &self,
        controller: CanControllerId,
        position: PidPosition,
    ) -> Result<(), CanError> {
        unsafe { crate::ffi::can_set_pos(controller.as_u8(), position.angle().as_degrees()) }
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

    /// Copy CAN status message 4 for one remote controller.
    pub fn status4(&self, controller: CanControllerId) -> Option<CanStatus4> {
        let raw = unsafe { crate::ffi::can_status_msg_4_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus4 {
            controller,
            fet_temperature: MosfetTemperature::new(crate::Temperature::from_degrees_celsius(
                raw.temp_fet,
            )),
            motor_temperature: MotorTemperature::new(crate::Temperature::from_degrees_celsius(
                raw.temp_motor,
            )),
            input_current: InputCurrent::new(Current::from_amps(raw.current_in)),
            position: PidPosition::new(crate::AngleDegrees::from_degrees(raw.pid_pos_now)),
        })
    }

    /// Copy CAN status message 5 for one remote controller.
    pub fn status5(&self, controller: CanControllerId) -> Option<CanStatus5> {
        let raw = unsafe { crate::ffi::can_status_msg_5_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus5 {
            controller,
            input_voltage: InputVoltage::new(crate::Voltage::from_volts(raw.v_in)),
            tachometer: TachometerSteps::new(crate::units::TachometerSteps::from_steps(
                raw.tacho_value,
            )),
        })
    }

    /// Copy CAN status message 6 for one remote controller.
    pub fn status6(&self, controller: CanControllerId) -> Option<CanStatus6> {
        let raw = unsafe { crate::ffi::can_status_msg_6_id(i32::from(controller.as_u8())) }?;
        Some(CanStatus6 {
            controller,
            adc_1: AdcVoltage::new(crate::Voltage::from_volts(raw.adc_1)),
            adc_2: AdcVoltage::new(crate::Voltage::from_volts(raw.adc_2)),
            adc_3: AdcVoltage::new(crate::Voltage::from_volts(raw.adc_3)),
            ppm: PpmInput::new(
                crate::SignedRatio::from_ratio(raw.ppm)
                    .unwrap_or(crate::SignedRatio::from_ratio_const(0.0)),
            ),
        })
    }
}

#[cfg(all(feature = "test-support", not(test)))]
pub(crate) fn reset_receiver_registrations() {
    SID_RECEIVER_REGISTERED.store(false, Ordering::Release);
    EID_RECEIVER_REGISTERED.store(false, Ordering::Release);
}
