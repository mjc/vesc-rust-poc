use super::super::protocol::realtime_value;
use super::FloatOutBoyPackageState;
use crate::domain::{
    FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID, FLOAT_OUT_BOY_REALTIME_RECORDED_ITEMS,
    FloatOutBoyAppDataCommand, FloatOutBoyRunState, FloatOutBoyWheelSlipState,
};
use vescpkg_rs::{DataRecorder, DataRecorderProtocol, SampleRate, TimestampTicks};

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
pub(super) struct DataRecorderState(pub(super) DataRecorder<DataRecorderStorage, SAMPLE_SIZE>);

pub(super) const fn default_data_recorder() -> DataRecorderState {
    DataRecorderState(DataRecorder::new(
        #[cfg(test)]
        Some([0; TEST_SAMPLE_CAPACITY * SAMPLE_SIZE]),
        #[cfg(not(test))]
        None,
        DEFAULT_SAMPLE_RATE_HZ,
    ))
}

#[cfg(test)]
impl Default for DataRecorderState {
    fn default() -> Self {
        default_data_recorder()
    }
}

impl FloatOutBoyPackageState {
    pub(crate) fn stop_data_recorder(&mut self) {
        self.data_recorder.0.replace_storage(None);
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    pub(crate) fn initialize_data_recorder(
        &mut self,
        buffer: Option<vescpkg_rs::FirmwareDataRecorderBuffer>,
    ) {
        self.data_recorder.0.initialize(
            buffer.filter(|buffer| buffer.len() >= SAMPLE_SIZE),
            TARGET_RECORDING_SECONDS,
        );
    }

    #[cfg(test)]
    pub(super) fn disable_data_recorder_for_test(&mut self) {
        self.data_recorder.0.replace_storage(None);
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn initialize_data_recorder_sample_rate(&mut self, sample_rate: SampleRate) {
        self.data_recorder
            .0
            .records_mut()
            .configure_sample_rate(sample_rate, Some(TARGET_RECORDING_SECONDS));
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn refresh_data_recorder_sample_rate(&mut self, sample_rate: SampleRate) {
        self.data_recorder
            .0
            .records_mut()
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
        let _ = self.data_recorder.0.sample(sample);
    }

    pub(super) fn trigger_data_recorder(&mut self, engage: bool) {
        self.data_recorder.0.trigger(engage);
    }

    pub(super) fn handle_data_recorder_packet(
        &mut self,
        reply: &mut impl FnMut(&[u8]) -> bool,
        payload: &[u8],
    ) -> bool {
        #[cfg(test)]
        let reported_capacity = Some(TEST_SAMPLE_CAPACITY);
        #[cfg(not(test))]
        let reported_capacity = None;
        self.data_recorder.0.reply_to_request(
            payload,
            &DATA_RECORDER_PROTOCOL,
            reported_capacity,
            reply,
        );
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

const DATA_RECORDER_PROTOCOL: DataRecorderProtocol<HEADER_RESPONSE_LEN, DATA_RESPONSE_CAPACITY> =
    DataRecorderProtocol::new(
        FLOAT_OUT_BOY_APP_DATA_PACKAGE_ID,
        FloatOutBoyAppDataCommand::DataRecordRequest.id(),
        FloatOutBoyAppDataCommand::DataRecordHeader.id(),
        FloatOutBoyAppDataCommand::DataRecordData.id(),
        DATA_RECORD_HEADER_BYTES,
    );
