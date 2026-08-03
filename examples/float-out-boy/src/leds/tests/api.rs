use super::*;

impl FloatOutBoyLedRenderer {
    pub(crate) const fn confirmation_start_for_test(&self) -> f32 {
        self.confirmation_start
    }
}

impl FloatOutBoyLedStripFrame {
    pub(crate) fn set_logical_pixel(&mut self, index: usize, pixel: FloatOutBoyLedPixel) -> bool {
        if index >= usize::from(self.config.count()) {
            return false;
        }
        let Some(target) = self.pixels.get_mut(index) else {
            return false;
        };
        *target = pixel;
        true
    }
}
