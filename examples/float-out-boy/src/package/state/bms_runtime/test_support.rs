use super::*;

impl BmsRuntimeState {
    pub(in crate::package::state) const fn sample(self) -> FloatOutBoyBmsSample {
        self.sample
    }

    pub(in crate::package::state) const fn faults(self) -> FloatOutBoyBmsFaults {
        self.faults
    }
}
