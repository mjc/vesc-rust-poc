use super::*;

impl LcmState {
    pub(in crate::package::state) const fn hardware_mode(self) -> u8 {
        self.hardware_mode
    }
}

impl FloatOutBoyPackageState {
    pub(super) fn set_lcm_hardware_mode_for_test(&mut self, mode: u8) {
        self.lcm.set_hardware_mode(mode);
    }
}
