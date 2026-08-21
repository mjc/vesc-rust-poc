use super::*;

impl FloatOutBoyPackageState {
    pub(in crate::package::state) fn disable_data_recorder_for_test(&mut self) {
        self.data_recorder.records.replace_storage(None);
        self.data_recorder.stop();
    }
}
