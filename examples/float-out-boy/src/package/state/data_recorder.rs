#[cfg(any(test, target_arch = "arm"))]
use super::super::protocol::realtime_value;
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

#[cfg(test)]
type DataRecorderStorage = Option<[u8; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE]>;
#[cfg(all(not(test), target_arch = "arm"))]
type DataRecorderStorage = Option<vescpkg_rs::FirmwareDataRecorderBuffer>;
#[cfg(all(not(test), not(target_arch = "arm")))]
type DataRecorderStorage = Option<[u8; 0]>;
#[derive(Debug)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq, Eq))]
pub(super) struct DataRecorderState {
    flags: FloatOutBoyDataRecorderFlags,
    records: vescpkg_rs::FixedRecordRing<DataRecorderStorage, SAMPLE_SIZE>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DataRecorderTrigger {
    Engage,
    Disengage,
}

impl Default for DataRecorderState {
    fn default() -> Self {
        Self {
            flags: FloatOutBoyDataRecorderFlags::AUTOSTART | FloatOutBoyDataRecorderFlags::AUTOSTOP,
            #[cfg(test)]
            records: vescpkg_rs::FixedRecordRing::new(Some(
                [0; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE],
            )),
            #[cfg(not(test))]
            records: vescpkg_rs::FixedRecordRing::new(None),
        }
    }
}

impl DataRecorderState {
    #[cfg(all(not(test), target_arch = "arm"))]
    fn initialize(&mut self, buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>) {
        self.records
            .replace_storage(buffer.filter(|buffer| buffer.len() >= SAMPLE_SIZE));
        self.stop();
    }

    pub(super) fn has_capability(&self) -> bool {
        self.records.capacity() > 0
    }

    pub(super) fn flags(&self) -> FloatOutBoyDataRecorderFlags {
        if self.has_capability() {
            self.flags
        } else {
            FloatOutBoyDataRecorderFlags::empty()
        }
    }

    fn trigger(&mut self, trigger: DataRecorderTrigger) {
        if !self.has_capability() {
            return;
        }

        match trigger {
            DataRecorderTrigger::Engage
                if self.flags.contains(FloatOutBoyDataRecorderFlags::AUTOSTART) =>
            {
                self.start();
            }
            DataRecorderTrigger::Disengage
                if self.flags.contains(FloatOutBoyDataRecorderFlags::AUTOSTOP) =>
            {
                self.stop();
            }
            DataRecorderTrigger::Engage | DataRecorderTrigger::Disengage => {}
        }
    }

    fn start(&mut self) {
        self.records.clear();
        self.flags.set(
            FloatOutBoyDataRecorderFlags::RECORDING,
            self.has_capability(),
        );
    }

    fn stop(&mut self) {
        self.flags.remove(FloatOutBoyDataRecorderFlags::RECORDING);
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn shutdown(&mut self) {
        self.stop();
        self.records.replace_storage(None);
    }

    #[cfg(any(test, target_arch = "arm"))]
    fn sample(&mut self, sample: &[u8; SAMPLE_SIZE]) {
        if self.flags.contains(FloatOutBoyDataRecorderFlags::RECORDING) {
            let _ = self.records.push(sample);
        }
    }

    fn sample_count(&self) -> usize {
        self.records.len()
    }

    fn sample_at(&self, index: usize) -> Option<[u8; SAMPLE_SIZE]> {
        self.records.get(index)
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
        self.data_recorder.records.replace_storage(None);
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
            vescpkg_rs::protocol_buffer::float16_auto_bits(realtime_value(
                &payloads,
                item,
                self.remote_control.input(),
                self.ride_modifiers.atr_accel_diff(),
                self.ride_modifiers.atr_speed_boost(),
            ))
        });
        let mut sample = [0; SAMPLE_SIZE];
        sample[..4].copy_from_slice(&timestamp.as_ticks().to_be_bytes());
        sample[4] = flags;
        for (target, value) in sample[5..].chunks_exact_mut(2).zip(values) {
            target.copy_from_slice(&value.to_be_bytes());
        }
        self.data_recorder.sample(&sample);
    }

    pub(super) fn trigger_data_recorder(&mut self, trigger: DataRecorderTrigger) {
        self.data_recorder.trigger(trigger);
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

        match payload {
            [1, 1, value, ..] if *value > 0 => {
                self.data_recorder.start();
            }
            [1, 1, ..] => {
                self.data_recorder.stop();
            }
            [1, 2, value, ..] => {
                self.data_recorder
                    .flags
                    .set(FloatOutBoyDataRecorderFlags::AUTOSTART, *value > 0);
            }
            [1, 3, value, ..] => {
                self.data_recorder
                    .flags
                    .set(FloatOutBoyDataRecorderFlags::AUTOSTOP, *value > 0);
            }
            [2, 1, ..] => {
                self.data_recorder.stop();
                let mut response = DATA_RECORD_HEADER_BYTES;
                response[1] = FloatOutBoyAppDataCommand::DataRecordHeader.id();
                let sample_count =
                    u32::try_from(self.data_recorder.sample_count()).unwrap_or(u32::MAX);
                response[2..6].copy_from_slice(&sample_count.to_be_bytes());
                let _ = reply(&response);
            }
            [2, 2, a, b, c, d, ..] => {
                let offset = u32::from_be_bytes([*a, *b, *c, *d]);
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
            _ => {}
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
