use crate::{
    domain::FloatOutBoyDarkRideState,
    lcm::FloatOutBoyHardwareLedsConfig,
    leds::{
        FloatOutBoyLedFrameUpdate, FloatOutBoyLedRenderer, FloatOutBoyLedStatusUpdate,
        FloatOutBoyLedUpdate, FloatOutBoyLedsConfig,
    },
};
use vescpkg_rs::MotorTelemetry;

use super::FloatOutBoyPackageState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FloatOutBoyInternalLedRuntime {
    renderer: FloatOutBoyLedRenderer,
    config: FloatOutBoyLedsConfig,
}

impl FloatOutBoyPackageState {
    /// Replace the pure internal LED runtime during setup or reconfiguration.
    pub(crate) fn configure_internal_leds(
        &mut self,
        hardware: FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
    ) {
        self.internal_leds = Some(FloatOutBoyInternalLedRuntime {
            renderer: FloatOutBoyLedRenderer::new(hardware, config, 0.0),
            config,
        });
    }

    pub(super) fn refresh_internal_leds_from_config(&mut self) {
        self.internal_leds = None;
        let Some((hardware, config)) = self.serialized_config.led_configs() else {
            return;
        };
        if hardware.uses_internal_leds() {
            self.configure_internal_leds(hardware, config);
        }
    }

    /// Sample one coherent firmware snapshot, render it, and expose it for one paint.
    pub(crate) fn render_internal_leds(
        &mut self,
        telemetry: &impl MotorTelemetry,
        current_time: f32,
        paint: impl FnOnce(&FloatOutBoyLedRenderer),
    ) {
        let base = self.all_data_payloads.base();
        let ride_state = base.status().ride_state();
        let frame = FloatOutBoyLedFrameUpdate::new(
            FloatOutBoyLedUpdate {
                run_state: ride_state.run_state(),
                mode: ride_state.mode(),
                darkride: matches!(ride_state.darkride(), FloatOutBoyDarkRideState::Active),
                footpad: base.footpad().state(),
                pitch_degrees: crate::wire::degrees(base.attitude().pitch().angle()),
                distance: telemetry.signed_trip_distance().distance().as_meters(),
            },
            FloatOutBoyLedStatusUpdate {
                battery_level: telemetry.battery_level().as_fraction(),
                duty_cycle: telemetry.duty_cycle().ratio().as_ratio(),
                moving: telemetry
                    .electrical_speed()
                    .rpm()
                    .as_revolutions_per_minute()
                    .abs()
                    > 100.0,
            },
        );
        if let Some(runtime) = self.internal_leds.as_mut() {
            runtime.renderer.update(runtime.config, frame, current_time);
            paint(&runtime.renderer);
        }
    }
}
