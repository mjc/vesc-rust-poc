use crate::{
    domain::FloatOutBoyDarkRideState,
    lcm::FloatOutBoyHardwareLedsConfig,
    leds::{
        FloatOutBoyLedFrameUpdate, FloatOutBoyLedPin, FloatOutBoyLedRenderer,
        FloatOutBoyLedStatusUpdate, FloatOutBoyLedUpdate, FloatOutBoyLedsConfig,
    },
};
use vescpkg_rs::{MotorTelemetry, SYSTEM_TICK_RATE_HZ, TimestampTicks};

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
pub(super) use hardware::RuntimeAllocation;

#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_arch = "arm"), derive(Clone, Copy))]
pub(super) struct FloatOutBoyInternalLedRuntime {
    renderer: FloatOutBoyLedRenderer,
    config: FloatOutBoyLedsConfig,
    driver: FloatOutBoyInternalLedDriver,
}

impl FloatOutBoyPackageState {
    pub(super) fn start_internal_led_confirmation(&mut self, system_time_ticks: TimestampTicks) {
        let [low0, low1, high0, high1] = system_time_ticks.as_ticks().to_le_bytes();
        let low = f32::from(u16::from_le_bytes([low0, low1]));
        let high = f32::from(u16::from_le_bytes([high0, high1]));
        let ticks_per_second = u16::try_from(SYSTEM_TICK_RATE_HZ).map_or(f32::NAN, f32::from);
        let current_time = (high * 65_536.0 + low) / ticks_per_second;
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self
            .internal_leds
            .as_mut()
            .and_then(RuntimeAllocation::runtime_mut);
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
        #[cfg(target_arch = "arm")]
        let Some(allocation) = RuntimeAllocation::allocate() else {
            return;
        };
        let mut runtime = FloatOutBoyInternalLedRuntime {
            renderer: FloatOutBoyLedRenderer::new(hardware, config, 0.0),
            config,
            driver: FloatOutBoyInternalLedDriver::new(hardware),
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
                self.internal_leds = Some(allocation.initialize(runtime));
            }
        } else {
            #[cfg(target_arch = "arm")]
            allocation.release_uninitialized();
        }
    }

    pub(super) fn refresh_internal_leds_from_config(
        &mut self,
        hardware: FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
    ) {
        if hardware.uses_internal_leds()
            && !matches!(
                self.all_data_payloads.base().footpad().state(),
                crate::FloatOutBoyFootpadState::Both
            )
        {
            self.configure_internal_leds(hardware, config);
        }
    }

    pub(super) fn update_internal_led_config(&mut self, config: FloatOutBoyLedsConfig) {
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self
            .internal_leds
            .as_mut()
            .and_then(RuntimeAllocation::runtime_mut);
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
        let destroyed = self.internal_leds.as_mut().is_none_or(|runtime| {
            runtime
                .runtime_mut()
                .is_none_or(|runtime| runtime.driver.destroy(teardown))
        });

        if destroyed {
            #[cfg(test)]
            {
                self.internal_leds = None;
            }
            #[cfg(target_arch = "arm")]
            if let Some(runtime) = self.internal_leds.take() {
                runtime.release();
            }
        }
        destroyed
    }

    pub(crate) fn internal_leds_operational(&self) -> bool {
        #[cfg(test)]
        let runtime = self.internal_leds.as_ref();
        #[cfg(target_arch = "arm")]
        let runtime = self
            .internal_leds
            .as_ref()
            .and_then(RuntimeAllocation::runtime);
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
        #[cfg(test)]
        let runtime = self.internal_leds.as_mut();
        #[cfg(target_arch = "arm")]
        let runtime = self
            .internal_leds
            .as_mut()
            .and_then(RuntimeAllocation::runtime_mut);
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
