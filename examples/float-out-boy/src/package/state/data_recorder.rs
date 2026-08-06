use super::super::protocol::realtime_value;
use super::{FloatOutBoyPackageState, float_out_boy_command_payload};
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS,
    FloatOutBoyAppDataCommand, FloatOutBoyDataRecorderFlags, FloatOutBoyRunState,
    FloatOutBoyWheelSlipState,
};
use crate::wire::FloatOutBoyPacket;
use vescpkg_rs::{SampleRate, TimestampTicks};

const RECORDED_VALUE_COUNT: usize = FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS.len();
const SAMPLE_SIZE: usize = 4 + 1 + 2 * RECORDED_VALUE_COUNT;
const HEADER_RESPONSE_LEN: usize = 172;
const DATA_RESPONSE_CAPACITY: usize = 511;
const DEFAULT_SAMPLE_RATE_HZ: u16 = 620;
const TARGET_RECORDING_SECONDS: u16 = 10;
#[cfg(test)]
const TEST_SAMPLE_CAPACITY: usize = 24;

#[cfg(test)]
type DataRecorderStorage = Option<[u8; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE]>;
#[cfg(all(not(test), target_arch = "arm"))]
type DataRecorderStorage = Option<vescpkg_rs::FirmwareDataRecorderBuffer>;

#[derive(Debug)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy, PartialEq, Eq))]
pub(super) struct DataRecorderState {
    flags: FloatOutBoyDataRecorderFlags,
    records: vescpkg_rs::DecimatedRecordRing<DataRecorderStorage, SAMPLE_SIZE>,
    last_timestamp: TimestampTicks,
}

impl Default for DataRecorderState {
    fn default() -> Self {
        Self {
            flags: FloatOutBoyDataRecorderFlags::AUTOSTART | FloatOutBoyDataRecorderFlags::AUTOSTOP,
            #[cfg(test)]
            records: vescpkg_rs::DecimatedRecordRing::new(
                Some([0; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE]),
                DEFAULT_SAMPLE_RATE_HZ,
            ),
            #[cfg(not(test))]
            records: vescpkg_rs::DecimatedRecordRing::new(None, DEFAULT_SAMPLE_RATE_HZ),
            last_timestamp: TimestampTicks::from_ticks(0),
        }
    }
}

impl DataRecorderState {
    #[cfg(all(not(test), target_arch = "arm"))]
    fn initialize(&mut self, buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>) {
        self.records
            .replace_storage(buffer.filter(|buffer| buffer.len() >= SAMPLE_SIZE));
        self.stop();
        self.records
            .recalculate_decimation(TARGET_RECORDING_SECONDS);
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

    fn trigger(&mut self, engage: bool) {
        if !self.has_capability() {
            return;
        }
        if self.flags.contains(FloatOutBoyDataRecorderFlags::AUTOSTART) && engage {
            self.start();
        } else if self.flags.contains(FloatOutBoyDataRecorderFlags::AUTOSTOP) && !engage {
            self.stop();
        }
    }

    fn start(&mut self) {
        self.records.reset();
        self.last_timestamp = TimestampTicks::from_ticks(0);
        self.flags.set(
            FloatOutBoyDataRecorderFlags::RECORDING,
            self.has_capability(),
        );
    }

    fn stop(&mut self) {
        self.flags.remove(FloatOutBoyDataRecorderFlags::RECORDING);
    }

    fn shutdown(&mut self) {
        self.stop();
        self.records.replace_storage(None);
    }

    fn sample(&mut self, mut sample: [u8; SAMPLE_SIZE]) {
        if !self.flags.contains(FloatOutBoyDataRecorderFlags::RECORDING) {
            return;
        }
        if !self.records.sample_due() {
            return;
        }
        let timestamp = u32::from_be_bytes(sample[..4].try_into().unwrap_or_default());
        let timestamp = if timestamp <= self.last_timestamp.as_ticks() {
            self.last_timestamp.as_ticks().wrapping_add(1)
        } else {
            timestamp
        };
        self.last_timestamp = TimestampTicks::from_ticks(timestamp);
        sample[..4].copy_from_slice(&timestamp.to_be_bytes());
        let _ = self.records.push(&sample);
    }

    fn status_response(&self) -> [u8; 7] {
        #[cfg(test)]
        let duration = self
            .records
            .recording_duration_centiseconds_at_capacity(TEST_SAMPLE_CAPACITY);
        #[cfg(not(test))]
        let duration = self.records.recording_duration_centiseconds();
        let duration = duration.to_be_bytes();
        [
            FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
            FloatOutBoyAppDataCommand::DataRecordRequest.id(),
            u8::from(self.has_capability()),
            self.flags.bits(),
            self.records.decimation(),
            duration[0],
            duration[1],
        ]
    }
}

impl FloatOutBoyPackageState {
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
    pub(crate) fn initialize_data_recorder_sample_rate(&mut self, sample_rate: SampleRate) {
        self.data_recorder
            .records
            .configure_sample_rate(sample_rate, Some(TARGET_RECORDING_SECONDS));
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_data_recorder_sample_rate(&mut self, sample_rate: SampleRate) {
        self.data_recorder
            .records
            .configure_sample_rate(sample_rate, None);
    }

    pub(crate) fn sample_data_recorder(&mut self, timestamp: TimestampTicks) {
        let payloads = self.all_data_payloads;
        let ride_state = payloads.ride_state();
        let flags = ride_state.setpoint_adjustment().id() << 4
            | payloads.footpad().state().id() << 2
            | u8::from(ride_state.wheelslip() == FloatOutBoyWheelSlipState::Detected) << 1
            | u8::from(ride_state.run_state() == FloatOutBoyRunState::Running);
        let values = FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS.map(|item| {
            vescpkg_rs::protocol_buffer::float16_auto_bits(realtime_value(
                &payloads,
                item,
                self.realtime_live_values(),
            ))
        });
        let mut sample = [0; SAMPLE_SIZE];
        sample[..4].copy_from_slice(&timestamp.as_ticks().to_be_bytes());
        sample[4] = flags;
        for (target, value) in sample[5..].chunks_exact_mut(2).zip(values) {
            target.copy_from_slice(&value.to_be_bytes());
        }
        self.data_recorder.sample(sample);
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

        match payload {
            [1, 1, value, ..] => {
                if *value > 0 {
                    self.data_recorder.start();
                } else {
                    self.data_recorder.stop();
                }
            }
            [1, 2, value, ..] => self
                .data_recorder
                .flags
                .set(FloatOutBoyDataRecorderFlags::AUTOSTART, *value > 0),
            [1, 3, value, ..] => self
                .data_recorder
                .flags
                .set(FloatOutBoyDataRecorderFlags::AUTOSTOP, *value > 0),
            [1, 4, value, ..] => self.data_recorder.records.set_decimation(*value),
            [1, 0, ..] | [1, _, _, ..] => {}
            [2, 1, ..] => {
                self.data_recorder.stop();
                let mut response = DATA_RECORD_HEADER_BYTES;
                response[1] = FloatOutBoyAppDataCommand::DataRecordHeader.id();
                let count = u32::try_from(self.data_recorder.records.len()).unwrap_or(u32::MAX);
                response[2..6].copy_from_slice(&count.to_be_bytes());
                let _ = reply(&response);
                return true;
            }
            [2, 2, a, b, c, d, ..] => {
                let offset = u32::from_be_bytes([*a, *b, *c, *d]);
                let mut response = FloatOutBoyPacket::<DATA_RESPONSE_CAPACITY>::new();
                response.push(FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID);
                response.push(FloatOutBoyAppDataCommand::DataRecordData.id());
                response.push_u32(offset);
                let mut sample_index = usize::try_from(offset).unwrap_or(usize::MAX);
                while response.remaining() >= SAMPLE_SIZE {
                    let Some(sample) = self.data_recorder.records.get(sample_index) else {
                        break;
                    };
                    response.extend(&sample);
                    sample_index = sample_index.saturating_add(1);
                }
                if !self.data_recorder.records.is_empty() {
                    let _ = reply(response.as_bytes());
                }
                return true;
            }
            _ => return true,
        }
        let _ = reply(&self.data_recorder.status_response());
        true
    }
}

const DATA_RECORD_HEADER_BYTES: [u8; HEADER_RESPONSE_LEN] = *b"\x65\0\0\0\0\0\x0d\
    \x0acontrol.dt\
    \x0ccontrol.freq\
    \x04erpm\
    \x0bdir_current\
    \x0aduty_cycle\
    \x0cbatt_voltage\
    \x05pitch\
    \x0dbalance_pitch\
    \x08setpoint\
    \x0catr.setpoint\
    \x14torque_tilt.setpoint\
    \x0fbalance_current\
    \x14atr.transition_boost";
