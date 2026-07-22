//! Refloat's allocation-free realtime circular recorder.

use super::{RefloatPackageState, refloat_command_payload};
use crate::domain::{
    REFLOAT_APP_DATA_PACKAGE_ID, REFLOAT_REALTIME_RECORDED_ITEMS, RefloatAppDataCommand,
    RefloatDataRecorderFlags,
};
#[cfg(any(test, target_arch = "arm"))]
use crate::domain::{RefloatRunState, RefloatWheelSlipState};
#[cfg(any(test, target_arch = "arm"))]
use crate::package::protocol::realtime_value;
#[cfg(any(test, target_arch = "arm"))]
use crate::package::protocol::wire::encode_refloat_float16;
use crate::package::protocol::wire::{refloat_realtime_push_u8, refloat_realtime_push_u32};
use vescpkg_rs::prelude::TimestampTicks;

#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::FirmwareDataRecorderBuffer;

const RECORDED_VALUE_COUNT: usize = REFLOAT_REALTIME_RECORDED_ITEMS.len();
const SAMPLE_SIZE: usize = 4 + 1 + 2 * RECORDED_VALUE_COUNT;
const HEADER_RESPONSE_LEN: usize = 159;
const DATA_RESPONSE_CAPACITY: usize = 511;
const DATA_RECORD_HEADER_COMMAND_ID: u8 = 42;
const DATA_RECORD_DATA_COMMAND_ID: u8 = 43;
#[cfg(test)]
const TEST_SAMPLE_CAPACITY: usize = 4;

vescpkg_rs::firmware_section_static!(
    ".text.refloat_data_record_header",
    static REFLOAT_DATA_RECORD_HEADER_BYTES: [u8; HEADER_RESPONSE_LEN] =
        build_data_record_header()
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DataRecorderSample {
    timestamp: TimestampTicks,
    flags: u8,
    values: [u16; RECORDED_VALUE_COUNT],
}

impl DataRecorderSample {
    fn encode(self) -> [u8; SAMPLE_SIZE] {
        let mut bytes = [0; SAMPLE_SIZE];
        bytes[..4].copy_from_slice(&self.timestamp.as_ticks().to_be_bytes());
        bytes[4] = self.flags;
        for (index, value) in self.values.into_iter().enumerate() {
            let offset = 5 + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
        }
        bytes
    }

    fn decode(bytes: [u8; SAMPLE_SIZE]) -> Self {
        let timestamp = TimestampTicks::from_ticks(u32::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]));
        let mut values = [0; RECORDED_VALUE_COUNT];
        for (index, value) in values.iter_mut().enumerate() {
            let offset = 5 + index * 2;
            *value = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
        }
        Self {
            timestamp,
            flags: bytes[4],
            values,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct DataRecorderState {
    enabled: bool,
    recording: bool,
    autostart: bool,
    autostop: bool,
    head: usize,
    tail: usize,
    empty: bool,
    #[cfg(test)]
    buffer: [u8; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE],
    #[cfg(all(not(test), target_arch = "arm"))]
    buffer: Option<FirmwareDataRecorderBuffer>,
}

#[cfg(test)]
impl Clone for DataRecorderState {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(test)]
impl Copy for DataRecorderState {}

impl Default for DataRecorderState {
    fn default() -> Self {
        Self {
            enabled: cfg!(test),
            recording: false,
            autostart: true,
            autostop: true,
            head: 0,
            tail: 0,
            empty: true,
            #[cfg(test)]
            buffer: [0; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE],
            #[cfg(all(not(test), target_arch = "arm"))]
            buffer: None,
        }
    }
}

impl DataRecorderState {
    #[cfg(all(not(test), target_arch = "arm"))]
    pub(super) fn initialize(&mut self, buffer: Option<FirmwareDataRecorderBuffer>) {
        self.enabled = buffer
            .as_ref()
            .is_some_and(|buffer| buffer.len() >= SAMPLE_SIZE);
        self.buffer = buffer;
        self.clear();
    }

    pub(super) const fn has_capability(&self) -> bool {
        self.enabled
    }

    pub(super) const fn flags(&self) -> RefloatDataRecorderFlags {
        let mut flags = RefloatDataRecorderFlags::inactive();
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

    pub(super) fn trigger(&mut self, engage: bool) {
        if !self.enabled {
            return;
        }
        if self.autostart && engage {
            self.start();
        } else if self.autostop && !engage {
            self.stop();
        }
    }

    fn start(&mut self) {
        self.clear();
        self.recording = self.enabled;
    }

    fn stop(&mut self) {
        self.recording = false;
    }

    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.empty = true;
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn sample(&mut self, sample: DataRecorderSample) {
        if !self.enabled || !self.recording {
            return;
        }
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }
        if !self.empty && self.head == self.tail {
            self.tail = (self.tail + 1) % capacity;
        }
        if self.write(self.head * SAMPLE_SIZE, &sample.encode()) {
            self.head = (self.head + 1) % capacity;
            self.empty = false;
        }
    }

    fn sample_count(&self) -> usize {
        if self.empty {
            return 0;
        }
        let capacity = self.capacity();
        if self.head == self.tail {
            capacity
        } else if self.head > self.tail {
            self.head - self.tail
        } else {
            self.head + capacity - self.tail
        }
    }

    fn sample_at(&self, index: usize) -> Option<DataRecorderSample> {
        let capacity = self.capacity();
        if index >= self.sample_count() || capacity == 0 {
            return None;
        }
        let slot = (self.tail + index) % capacity;
        let mut bytes = [0; SAMPLE_SIZE];
        self.read(slot * SAMPLE_SIZE, &mut bytes)
            .then(|| DataRecorderSample::decode(bytes))
    }

    fn capacity(&self) -> usize {
        #[cfg(test)]
        {
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
            0
        }
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        #[cfg(test)]
        {
            let Some(target) = self
                .buffer
                .get_mut(offset..offset.saturating_add(bytes.len()))
            else {
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
        #[cfg(all(not(test), not(target_arch = "arm")))]
        {
            let _ = (offset, bytes);
            false
        }
    }

    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        #[cfg(test)]
        {
            let Some(source) = self.buffer.get(offset..offset.saturating_add(bytes.len())) else {
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
            let _ = (offset, bytes);
            false
        }
    }
}

impl RefloatPackageState {
    #[cfg(all(not(test), target_arch = "arm"))]
    pub(crate) fn initialize_data_recorder(&mut self, buffer: Option<FirmwareDataRecorderBuffer>) {
        self.data_recorder.initialize(buffer);
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
                RefloatWheelSlipState::Detected
            )) << 1
            | u8::from(matches!(ride_state.run_state(), RefloatRunState::Running));
        let values = REFLOAT_REALTIME_RECORDED_ITEMS.map(|item| {
            encode_refloat_float16(realtime_value(
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
        send: &mut impl FnMut(&[u8]) -> bool,
        bytes: &[u8],
    ) -> bool {
        if refloat_command_payload(bytes, RefloatAppDataCommand::Experiment).is_some() {
            return true;
        }
        let Some(payload) =
            refloat_command_payload(bytes, RefloatAppDataCommand::DataRecordRequest)
        else {
            return false;
        };
        if !self.data_recorder.enabled {
            return true;
        }
        match payload {
            [1, submode, value, ..] => match submode {
                1 if *value > 0 => self.data_recorder.start(),
                1 => self.data_recorder.stop(),
                2 => self.data_recorder.autostart = *value > 0,
                3 => self.data_recorder.autostop = *value > 0,
                _ => {}
            },
            [2, 1, ..] => {
                self.data_recorder.stop();
                let mut response = REFLOAT_DATA_RECORD_HEADER_BYTES;
                response[2..6]
                    .copy_from_slice(&(self.data_recorder.sample_count() as u32).to_be_bytes());
                let _ = send(&response);
            }
            [2, 2, a, b, c, d, ..] => {
                let offset = u32::from_be_bytes([*a, *b, *c, *d]);
                let mut response = [0; DATA_RESPONSE_CAPACITY];
                let mut index = 0;
                refloat_realtime_push_u8(
                    &mut response,
                    &mut index,
                    REFLOAT_APP_DATA_PACKAGE_ID.get(),
                );
                refloat_realtime_push_u8(&mut response, &mut index, DATA_RECORD_DATA_COMMAND_ID);
                refloat_realtime_push_u32(&mut response, &mut index, offset);
                let mut sample_index = offset as usize;
                while index + SAMPLE_SIZE <= response.len() {
                    let Some(sample) = self.data_recorder.sample_at(sample_index) else {
                        break;
                    };
                    let encoded = sample.encode();
                    response[index..index + SAMPLE_SIZE].copy_from_slice(&encoded);
                    index += SAMPLE_SIZE;
                    sample_index += 1;
                }
                if self.data_recorder.sample_count() > 0 {
                    let _ = send(&response[..index]);
                }
            }
            _ => {}
        }
        true
    }
}

const fn build_data_record_header() -> [u8; HEADER_RESPONSE_LEN] {
    let mut bytes = [0; HEADER_RESPONSE_LEN];
    let mut index = 0;
    index = push_const(&mut bytes, index, REFLOAT_APP_DATA_PACKAGE_ID.get());
    index = push_const(&mut bytes, index, DATA_RECORD_HEADER_COMMAND_ID);
    index += 4;
    index = push_const(&mut bytes, index, RECORDED_VALUE_COUNT as u8);
    index = push_id(&mut bytes, index, b"motor.erpm");
    index = push_id(&mut bytes, index, b"motor.dir_current");
    index = push_id(&mut bytes, index, b"motor.duty_cycle");
    index = push_id(&mut bytes, index, b"motor.batt_voltage");
    index = push_id(&mut bytes, index, b"imu.pitch");
    index = push_id(&mut bytes, index, b"imu.balance_pitch");
    index = push_id(&mut bytes, index, b"setpoint");
    index = push_id(&mut bytes, index, b"atr.setpoint");
    index = push_id(&mut bytes, index, b"torque_tilt.setpoint");
    let _ = push_id(&mut bytes, index, b"balance_current");
    bytes
}

const fn push_id<const N: usize>(
    bytes: &mut [u8; HEADER_RESPONSE_LEN],
    index: usize,
    id: &[u8; N],
) -> usize {
    let mut next = push_const(bytes, index, N as u8);
    let mut offset = 0;
    while offset < N {
        next = push_const(bytes, next, id[offset]);
        offset += 1;
    }
    next
}

const fn push_const(bytes: &mut [u8; HEADER_RESPONSE_LEN], index: usize, value: u8) -> usize {
    bytes[index] = value;
    index + 1
}
