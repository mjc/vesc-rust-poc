use crate::{
    domain::FloatOutBoyDarkRideState,
    lcm::FloatOutBoyHardwareLedsConfig,
    leds::{
        FloatOutBoyLedPin, FloatOutBoyLedRenderer, FloatOutBoyLedUpdate, FloatOutBoyLedsConfig,
    },
};
use vescpkg_rs::{MotorTelemetry, TimestampTicks};

use super::FloatOutBoyPackageState;

mod driver;
#[cfg_attr(
    target_arch = "arm",
    expect(
        unsafe_code,
        reason = "the ARM-only adapter owns the source-mapped STM32 timer, DMA, and pad MMIO boundary"
    )
)]
mod hardware;

use driver::FloatOutBoyInternalLedDriver;
#[cfg(target_arch = "arm")]
pub(super) type RuntimeAllocation = vescpkg_rs::FallibleBox<FloatOutBoyInternalLedRuntime>;

#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub(super) struct FloatOutBoyInternalLedRuntime {
    renderer: FloatOutBoyLedRenderer,
    config: FloatOutBoyLedsConfig,
    driver: FloatOutBoyInternalLedDriver,
}

impl FloatOutBoyPackageState {
    pub(super) fn request_internal_led_refresh(&mut self) {
        self.internal_led_refresh_pending = true;
    }

    pub(crate) fn apply_pending_internal_led_refresh(&mut self) {
        if !core::mem::take(&mut self.internal_led_refresh_pending) {
            return;
        }
        if !self.destroy_internal_leds() {
            self.internal_led_refresh_pending = true;
            return;
        }
        if let Some((hardware, config)) = self.effective_led_config() {
            self.refresh_internal_leds_from_config(hardware, config);
        }
        if let Some(timestamp) = self.internal_led_confirmation_pending.take() {
            self.start_internal_led_confirmation(timestamp);
        }
    }

    pub(crate) fn start_internal_led_confirmation(&mut self, system_time_ticks: TimestampTicks) {
        if self.internal_led_refresh_pending {
            self.internal_led_confirmation_pending
                .get_or_insert(system_time_ticks);
            return;
        }
        let current_time = system_time_ticks.as_vesc_seconds().as_seconds();
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self.internal_leds.as_deref_mut();
        if let Some(runtime) = runtime {
            runtime.renderer.start_confirmation(current_time);
        }
    }

    /// Replace the pure internal LED runtime during setup or reconfiguration.
    #[cfg_attr(target_arch = "arm", inline(never))]
    pub(crate) fn configure_internal_leds(
        &mut self,
        hardware: FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
    ) {
        let runtime = FloatOutBoyInternalLedRuntime {
            renderer: FloatOutBoyLedRenderer::new(hardware, config, 0.0),
            config,
            driver: FloatOutBoyInternalLedDriver::new(hardware),
        };
        #[cfg(test)]
        let mut runtime = runtime;
        #[cfg(target_arch = "arm")]
        let Ok(mut runtime) = vescpkg_rs::FallibleBox::try_new(runtime) else {
            return;
        };
        if runtime.driver.setup(hardware::setup, |pin| {
            let _ = hardware::teardown(pin);
        }) {
            #[cfg(test)]
            {
                self.internal_leds = Some(runtime);
            }
            #[cfg(target_arch = "arm")]
            {
                self.internal_leds = Some(runtime);
            }
        }
    }

    pub(super) fn refresh_internal_leds_from_config(
        &mut self,
        hardware: FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
    ) {
        if matches!(
            hardware.mode,
            crate::lcm::FloatOutBoyLedMode::Internal | crate::lcm::FloatOutBoyLedMode::Both
        ) {
            self.configure_internal_leds(hardware, config);
        }
    }

    pub(super) fn update_internal_led_config(&mut self, config: FloatOutBoyLedsConfig) {
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self.internal_leds.as_deref_mut();
        if let Some(runtime) = runtime {
            runtime.config = config;
        }
    }

    pub(crate) fn destroy_internal_leds(&mut self) -> bool {
        self.destroy_internal_leds_with(hardware::teardown)
    }

    pub(crate) fn destroy_internal_leds_with(
        &mut self,
        teardown: impl FnOnce(FloatOutBoyLedPin) -> bool,
    ) -> bool {
        #[cfg(test)]
        let destroyed = self
            .internal_leds
            .as_mut()
            .is_none_or(|runtime| runtime.driver.destroy(teardown));
        #[cfg(target_arch = "arm")]
        let destroyed = self
            .internal_leds
            .as_deref_mut()
            .is_none_or(|runtime| runtime.driver.destroy(teardown));

        if destroyed {
            self.internal_leds = None;
        }
        destroyed
    }

    pub(crate) fn internal_leds_operational(&self) -> bool {
        #[cfg(test)]
        let runtime = self.internal_leds.as_ref();
        #[cfg(target_arch = "arm")]
        let runtime = self.internal_leds.as_deref();
        runtime.is_some_and(|runtime| runtime.driver.is_operational())
    }

    #[cfg(test)]
    pub(crate) fn internal_led_renderer_for_test(&self) -> Option<FloatOutBoyLedRenderer> {
        self.internal_leds.as_ref().map(|runtime| runtime.renderer)
    }

    #[cfg(test)]
    pub(crate) fn internal_led_confirmation_start_for_test(&self) -> Option<f32> {
        self.internal_leds
            .as_ref()
            .map(|runtime| runtime.renderer.confirmation_start_for_test())
            .or_else(|| {
                self.internal_led_confirmation_pending
                    .map(|timestamp| timestamp.as_vesc_seconds().as_seconds())
            })
    }

    /// Sample one coherent firmware snapshot, render it, and expose it for one paint.
    pub(crate) fn render_internal_leds(
        &mut self,
        telemetry: &impl MotorTelemetry,
        current_time: f32,
        paint: impl FnOnce(&FloatOutBoyLedRenderer),
    ) {
        let payloads = self.all_data_payloads;
        let ride_state = payloads.ride_state();
        let filtered_current = payloads.filtered_motor_current().current().current();
        let motor_limit = if payloads.motor_current().is_negative() {
            self.motor_current_min
        } else {
            self.motor_current_max
        };
        let motor_current_saturation =
            vescpkg_rs::current_limit_saturation(filtered_current, motor_limit.current())
                .as_ratio();
        let battery_current = payloads.battery_current().current();
        let battery_limit = if battery_current.is_negative() {
            self.battery_current_min
        } else {
            self.battery_current_max
        };
        let battery_current_saturation =
            vescpkg_rs::current_limit_saturation(battery_current, battery_limit.current())
                .as_ratio();
        let frame = FloatOutBoyLedUpdate {
            run_state: ride_state.run_state(),
            mode: ride_state.mode(),
            darkride: ride_state.darkride() == FloatOutBoyDarkRideState::Active,
            footpad: payloads.footpad().state(),
            pitch_degrees: crate::wire::degrees(payloads.pitch().angle()),
            distance: telemetry.signed_trip_distance().distance().as_meters(),
            battery_level: telemetry.battery_level().as_fraction(),
            duty_cycle: payloads.duty_cycle().ratio().as_ratio(),
            motor_current_saturation,
            battery_current_saturation,
            moving: telemetry
                .electrical_speed()
                .rpm()
                .as_revolutions_per_minute()
                .abs()
                > 100.0,
        };
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self.internal_leds.as_deref_mut();
        if let Some(runtime) = runtime {
            if runtime.renderer.update(runtime.config, frame, current_time)
                && runtime
                    .driver
                    .paint(&runtime.renderer, hardware::quiesce, hardware::restart)
            {
                paint(&runtime.renderer);
            }
        }
    }
}
