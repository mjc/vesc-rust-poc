use super::*;

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
