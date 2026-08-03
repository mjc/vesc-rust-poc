#[cfg(any(test, target_arch = "arm"))]
use super::super::protocol::realtime_value;
#[cfg(any(test, target_arch = "arm"))]
use super::super::protocol::wire::encode_float_out_boy_float16;
use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS,
    FloatOutBoyAppDataCommand, FloatOutBoyDataRecorderFlags,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::{FloatOutBoyRunState, FloatOutBoyWheelSlipState};
use crate::wire::FloatOutBoyPacket;
#[cfg(any(test, target_arch = "arm"))]
use vescpkg_rs::TimestampTicks;

const RECORDED_VALUE_COUNT: usize = FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS.len();
const SAMPLE_SIZE: usize = 4 + 1 + 2 * RECORDED_VALUE_COUNT;
const HEADER_RESPONSE_LEN: usize = 159;
const DATA_RESPONSE_CAPACITY: usize = 511;
#[cfg(test)]
const TEST_SAMPLE_CAPACITY: usize = 24;

#[cfg(any(test, target_arch = "arm"))]
fn advance_ring_index(index: usize, capacity: usize) -> usize {
    index
        .checked_add(1)
        .filter(|next| *next < capacity)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(test, target_arch = "arm"))]
struct DataRecorderSample {
    timestamp: TimestampTicks,
    flags: u8,
    values: [u16; RECORDED_VALUE_COUNT],
}

#[cfg(any(test, target_arch = "arm"))]
impl DataRecorderSample {
    fn encode(self) -> [u8; SAMPLE_SIZE] {
        let mut bytes = [0; SAMPLE_SIZE];
        bytes[..4].copy_from_slice(&self.timestamp.as_ticks().to_be_bytes());
        bytes[4] = self.flags;
        if let Some(value_bytes) = bytes.get_mut(5..) {
            for (target, value) in value_bytes.chunks_exact_mut(2).zip(self.values) {
                target.copy_from_slice(&value.to_be_bytes());
            }
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataRecorderAvailability {
    Unavailable,
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataRecorderRequest {
    SetRecording(bool),
    SetAutostart(bool),
    SetAutostop(bool),
    SendHeader,
    SendData { offset: u32 },
    Ignore,
}

impl DataRecorderRequest {
    fn parse(payload: &[u8]) -> Self {
        match payload {
            [1, 1, value, ..] => Self::SetRecording(*value > 0),
            [1, 2, value, ..] => Self::SetAutostart(*value > 0),
            [1, 3, value, ..] => Self::SetAutostop(*value > 0),
            [2, 1, ..] => Self::SendHeader,
            [2, 2, a, b, c, d, ..] => Self::SendData {
                offset: u32::from_be_bytes([*a, *b, *c, *d]),
            },
            _ => Self::Ignore,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct DataRecorderRing {
    next: usize,
    len: usize,
}

impl DataRecorderRing {
    fn clear(&mut self) {
        *self = Self::default();
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn write_slot(self, capacity: usize) -> Option<usize> {
        (capacity > 0).then_some(self.next)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn commit_write(&mut self, capacity: usize) {
        self.next = advance_ring_index(self.next, capacity);
        self.len = self.len.saturating_add(1).min(capacity);
    }

    fn len(self, capacity: usize) -> usize {
        self.len.min(capacity)
    }

    fn slot_at(self, index: usize, capacity: usize) -> Option<usize> {
        let len = self.len(capacity);
        if index >= len || capacity == 0 {
            return None;
        }
        let oldest = self
            .next
            .checked_add(capacity)?
            .checked_sub(len)?
            .checked_rem(capacity)?;
        oldest.checked_add(index)?.checked_rem(capacity)
    }
}

#[cfg(test)]
mod ring_tests;

#[derive(Debug)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq, Eq))]
pub(super) struct DataRecorderState {
    availability: DataRecorderAvailability,
    recording: bool,
    autostart: bool,
    autostop: bool,
    ring: DataRecorderRing,
    #[cfg(test)]
    buffer: [u8; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE],
    #[cfg(all(not(test), target_arch = "arm"))]
    buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>,
}

impl Default for DataRecorderState {
    fn default() -> Self {
        Self {
            availability: if cfg!(test) {
                DataRecorderAvailability::Available
            } else {
                DataRecorderAvailability::Unavailable
            },
            recording: false,
            autostart: true,
            autostop: true,
            ring: DataRecorderRing::default(),
            #[cfg(test)]
            buffer: [0; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE],
            #[cfg(all(not(test), target_arch = "arm"))]
            buffer: None,
        }
    }
}

impl DataRecorderState {
    #[cfg(all(not(test), target_arch = "arm"))]
    fn initialize(&mut self, buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>) {
        self.buffer = buffer.filter(|buffer| buffer.len() >= SAMPLE_SIZE);
        self.availability = if self.buffer.is_some() {
            DataRecorderAvailability::Available
        } else {
            DataRecorderAvailability::Unavailable
        };
        self.stop();
        self.ring.clear();
    }

    pub(super) const fn has_capability(&self) -> bool {
        matches!(self.availability, DataRecorderAvailability::Available)
    }

    pub(super) const fn flags(&self) -> FloatOutBoyDataRecorderFlags {
        if !self.has_capability() {
            return FloatOutBoyDataRecorderFlags::inactive();
        }

        let mut flags = FloatOutBoyDataRecorderFlags::inactive();
        if self.recording {
            flags = flags.with_recording();
        }
        if self.autostart {
            flags = flags.with_autostart();
        }
        if self.autostop {
            flags = flags.with_autostop();
        }
        flags
    }

    fn trigger(&mut self, engage: bool) {
        if !self.has_capability() {
            return;
        }
        if self.autostart && engage {
            self.start();
        } else if self.autostop && !engage {
            self.stop();
        }
    }

    fn start(&mut self) {
        self.ring.clear();
        self.recording = self.has_capability();
    }

    fn stop(&mut self) {
        self.recording = false;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn shutdown(&mut self) {
        self.stop();
        self.availability = DataRecorderAvailability::Unavailable;
        self.ring.clear();
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            self.buffer = None;
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn sample(&mut self, sample: DataRecorderSample) {
        if !self.has_capability() || !self.recording {
            return;
        }
        let capacity = self.capacity();
        let Some(slot) = self.ring.write_slot(capacity) else {
            return;
        };
        let Some(offset) = slot.checked_mul(SAMPLE_SIZE) else {
            return;
        };
        if self.write(offset, &sample.encode()) {
            self.ring.commit_write(capacity);
        }
    }

    fn sample_count(&self) -> usize {
        self.ring.len(self.capacity())
    }

    fn capacity(&self) -> usize {
        #[cfg(test)]
        {
            let _ = self.availability;
            TEST_SAMPLE_CAPACITY
        }
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            self.buffer
                .as_ref()
                .map_or(0, |buffer| buffer.len() / SAMPLE_SIZE)
        }
        #[cfg(all(not(test), not(target_arch = "arm")))]
        {
            let _ = self.availability;
            0
        }
    }

    fn sample_at(&self, index: usize) -> Option<[u8; SAMPLE_SIZE]> {
        let capacity = self.capacity();
        let slot = self.ring.slot_at(index, capacity)?;
        let offset = slot.checked_mul(SAMPLE_SIZE)?;
        let mut bytes = [0; SAMPLE_SIZE];
        self.read(offset, &mut bytes).then_some(bytes)
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        #[cfg(test)]
        {
            let Some(end) = offset.checked_add(bytes.len()) else {
                return false;
            };
            let Some(target) = self.buffer.get_mut(offset..end) else {
                return false;
            };
            target.copy_from_slice(bytes);
            true
        }
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            self.buffer
                .as_mut()
                .is_some_and(|buffer| buffer.write(offset, bytes))
        }
    }

    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        #[cfg(test)]
        {
            let Some(end) = offset.checked_add(bytes.len()) else {
                return false;
            };
            let Some(source) = self.buffer.get(offset..end) else {
                return false;
            };
            bytes.copy_from_slice(source);
            true
        }
        #[cfg(all(not(test), target_arch = "arm"))]
        {
            self.buffer
                .as_ref()
                .is_some_and(|buffer| buffer.read(offset, bytes))
        }
        #[cfg(all(not(test), not(target_arch = "arm")))]
        {
            let _ = (self.availability, offset, bytes);
            false
        }
    }
}

impl FloatOutBoyPackageState {
    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn stop_data_recorder(&mut self) {
        self.data_recorder.shutdown();
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    pub(crate) fn initialize_data_recorder(
        &mut self,
        buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>,
    ) {
        self.data_recorder.initialize(buffer);
    }

    #[cfg(test)]
    pub(super) fn disable_data_recorder_for_test(&mut self) {
        self.data_recorder.availability = DataRecorderAvailability::Unavailable;
        self.data_recorder.stop();
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn sample_data_recorder(&mut self, timestamp: TimestampTicks) {
        let payloads = self.all_data_payloads;
        let base = payloads.base();
        let ride_state = base.status().ride_state();
        let flags = ride_state.setpoint_adjustment().id() << 4
            | base.footpad().state().id() << 2
            | u8::from(matches!(
                ride_state.wheelslip(),
                FloatOutBoyWheelSlipState::Detected
            )) << 1
            | u8::from(matches!(
                ride_state.run_state(),
                FloatOutBoyRunState::Running
            ));
        let values = FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS.map(|item| {
            encode_float_out_boy_float16(realtime_value(
                &payloads,
                item,
                self.remote_control.input(),
                self.ride_modifiers.atr_accel_diff(),
                self.ride_modifiers.atr_speed_boost(),
            ))
        });
        self.data_recorder.sample(DataRecorderSample {
            timestamp,
            flags,
            values,
        });
    }

    pub(super) fn trigger_data_recorder(&mut self, engage: bool) {
        self.data_recorder.trigger(engage);
    }

    pub(super) fn handle_data_recorder_packet(
        &mut self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        let Some(payload) =
            float_out_boy_command_payload(bytes, FloatOutBoyAppDataCommand::DataRecordRequest)
        else {
            return false;
        };
        if !self.data_recorder.has_capability() {
            return true;
        }

        match DataRecorderRequest::parse(payload) {
            DataRecorderRequest::SetRecording(true) => {
                self.data_recorder.start();
            }
            DataRecorderRequest::SetRecording(false) => {
                self.data_recorder.stop();
            }
            DataRecorderRequest::SetAutostart(enabled) => {
                self.data_recorder.autostart = enabled;
            }
            DataRecorderRequest::SetAutostop(enabled) => {
                self.data_recorder.autostop = enabled;
            }
            DataRecorderRequest::SendHeader => {
                self.data_recorder.stop();
                let mut response = DATA_RECORD_HEADER_BYTES;
                response[1] = FloatOutBoyAppDataCommand::DataRecordHeader.id();
                let sample_count =
                    u32::try_from(self.data_recorder.sample_count()).unwrap_or(u32::MAX);
                response[2..6].copy_from_slice(&sample_count.to_be_bytes());
                let _ = reply(&response);
            }
            DataRecorderRequest::SendData { offset } => {
                let mut response = FloatOutBoyPacket::<DATA_RESPONSE_CAPACITY>::new();
                response.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID.get());
                response.push(FloatOutBoyAppDataCommand::DataRecordData.id());
                response.push_u32(offset);
                let mut sample_index = usize::try_from(offset).unwrap_or(usize::MAX);
                while response.remaining() >= SAMPLE_SIZE {
                    let Some(sample) = self.data_recorder.sample_at(sample_index) else {
                        break;
                    };
                    response.extend(&sample);
                    sample_index = sample_index.saturating_add(1);
                }
                if self.data_recorder.sample_count() > 0 {
                    let _ = reply(response.as_bytes());
                }
            }
            DataRecorderRequest::Ignore => {}
        }
        true
    }
}

const DATA_RECORD_HEADER_BYTES: [u8; HEADER_RESPONSE_LEN] = *b"\x65\0\0\0\0\0\x0a\
    \x0amotor.erpm\
    \x11motor.dir_current\
    \x10motor.duty_cycle\
    \x12motor.batt_voltage\
    \x09imu.pitch\
    \x11imu.balance_pitch\
    \x08setpoint\
    \x0catr.setpoint\
    \x14torque_tilt.setpoint\
    \x0fbalance_current";
