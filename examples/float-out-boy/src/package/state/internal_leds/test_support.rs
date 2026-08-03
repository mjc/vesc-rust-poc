use super::*;

impl FloatOutBoyPackageState {
    pub(crate) fn internal_led_renderer_for_test(&self) -> Option<FloatOutBoyLedRenderer> {
        self.internal_leds.as_ref().map(|runtime| runtime.renderer)
    }

    pub(crate) fn internal_led_confirmation_start_for_test(&self) -> Option<f32> {
        self.internal_leds
            .as_ref()
            .map(|runtime| runtime.renderer.confirmation_start_for_test())
            .or_else(|| {
                self.internal_led_confirmation_pending
                    .map(|timestamp| timestamp.as_vesc_seconds().as_seconds())
            })
    }
}
