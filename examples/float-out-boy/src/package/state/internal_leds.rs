use crate::{
    domain::FloatOutBoyDarkRideState,
    lcm::FloatOutBoyHardwareLedsConfig,
    leds::{
        FloatOutBoyLedFrameUpdate, FloatOutBoyLedPin, FloatOutBoyLedRenderer,
        FloatOutBoyLedStatusUpdate, FloatOutBoyLedUpdate, FloatOutBoyLedsConfig,
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
#[cfg(test)]
mod test_support;

use driver::FloatOutBoyInternalLedDriver;
#[cfg(target_arch = "arm")]
pub(super) type RuntimeAllocation = vescpkg_rs::FallibleBox<FloatOutBoyInternalLedRuntime>;

#[cfg(target_arch = "arm")]
pub(in crate::package) struct FloatOutBoyInternalLedAuxWork {
    runtime: Option<RuntimeAllocation>,
    effective_config: Option<(FloatOutBoyHardwareLedsConfig, FloatOutBoyLedsConfig)>,
    frame: FloatOutBoyLedFrameUpdate,
    current_time: f32,
    refresh: bool,
    confirmation: Option<TimestampTicks>,
}

#[cfg(target_arch = "arm")]
pub(in crate::package) struct FloatOutBoyInternalLedAuxResult {
    runtime: Option<RuntimeAllocation>,
    retry_refresh: bool,
    deferred_confirmation: Option<TimestampTicks>,
    operational: bool,
}

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

    #[cfg(target_arch = "arm")]
    pub(crate) fn start_internal_led_confirmation(&mut self, system_time_ticks: TimestampTicks) {
        self.internal_led_confirmation_pending
            .get_or_insert(system_time_ticks);
    }

    #[cfg(test)]
    pub(crate) fn start_internal_led_confirmation(&mut self, system_time_ticks: TimestampTicks) {
        if self.internal_led_refresh_pending {
            self.internal_led_confirmation_pending
                .get_or_insert(system_time_ticks);
            return;
        }
        let current_time = system_time_ticks.as_vesc_seconds().as_seconds();
        let runtime = self.internal_leds.as_mut();
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
                self.internal_leds_operational = true;
            }
        }
    }

    pub(super) fn refresh_internal_leds_from_config(
        &mut self,
        hardware: FloatOutBoyHardwareLedsConfig,
        config: FloatOutBoyLedsConfig,
    ) {
        if hardware.uses_internal_leds() {
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
            #[cfg(target_arch = "arm")]
            {
                self.internal_leds_operational = false;
            }
        }
        destroyed
    }

    #[cfg(test)]
    pub(crate) fn internal_leds_operational(&self) -> bool {
        self.internal_leds
            .as_ref()
            .is_some_and(|runtime| runtime.driver.is_operational())
    }

    #[cfg(all(not(test), target_arch = "arm"))]
    pub(crate) const fn internal_leds_operational(&self) -> bool {
        self.internal_leds_operational
    }

    #[cfg(target_arch = "arm")]
    pub(in crate::package) fn prepare_internal_led_aux_work(
        &mut self,
        telemetry: &impl MotorTelemetry,
        current_time: f32,
    ) -> FloatOutBoyInternalLedAuxWork {
        FloatOutBoyInternalLedAuxWork {
            runtime: self.internal_leds.take(),
            effective_config: self.effective_led_config(),
            frame: self.internal_led_frame(telemetry),
            current_time,
            refresh: core::mem::take(&mut self.internal_led_refresh_pending),
            confirmation: self.internal_led_confirmation_pending.take(),
        }
    }

    #[cfg(target_arch = "arm")]
    pub(in crate::package) fn commit_internal_led_aux_work(
        &mut self,
        result: FloatOutBoyInternalLedAuxResult,
    ) {
        self.internal_leds = result.runtime;
        self.internal_led_refresh_pending |= result.retry_refresh;
        if let Some(timestamp) = result.deferred_confirmation {
            self.internal_led_confirmation_pending
                .get_or_insert(timestamp);
        }
        self.internal_leds_operational = result.operational;
    }

    /// Sample one coherent firmware snapshot, render it, and expose it for one paint.
    #[cfg(test)]
    pub(crate) fn render_internal_leds(
        &mut self,
        telemetry: &impl MotorTelemetry,
        current_time: f32,
        paint: impl FnOnce(&FloatOutBoyLedRenderer),
    ) {
        let frame = self.internal_led_frame(telemetry);
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

    fn internal_led_frame(&self, telemetry: &impl MotorTelemetry) -> FloatOutBoyLedFrameUpdate {
        let base = self.all_data_payloads.base();
        let ride_state = base.status().ride_state();
        let filtered_current = base.motor().filtered_motor_current().current().current();
        let motor_limit = if base.motor().motor_current().is_negative() {
            self.motor_current_min
        } else {
            self.motor_current_max
        };
        let motor_current_saturation = super::haptic_feedback::normalized_current_saturation(
            filtered_current,
            motor_limit.current(),
        );
        let battery_current = base.motor().battery_current().current();
        let battery_limit = if battery_current.is_negative() {
            self.battery_current_min
        } else {
            self.battery_current_max
        };
        let battery_current_saturation = super::haptic_feedback::normalized_current_saturation(
            battery_current,
            battery_limit.current(),
        );
        FloatOutBoyLedFrameUpdate::new(
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
                duty_cycle: base.motor().duty_cycle().ratio().as_ratio(),
                motor_current_saturation,
                battery_current_saturation,
                moving: telemetry
                    .electrical_speed()
                    .rpm()
                    .as_revolutions_per_minute()
                    .abs()
                    > 100.0,
            },
        )
    }
}

#[cfg(target_arch = "arm")]
impl FloatOutBoyInternalLedAuxWork {
    pub(in crate::package) fn execute(mut self) -> FloatOutBoyInternalLedAuxResult {
        let retry_refresh = self.refresh && !self.refresh_runtime();
        let deferred_confirmation = self.confirmation.filter(|_| retry_refresh);
        if let (Some(timestamp), Some(runtime)) = (
            self.confirmation.filter(|_| !retry_refresh),
            self.runtime.as_deref_mut(),
        ) {
            runtime
                .renderer
                .start_confirmation(timestamp.as_vesc_seconds().as_seconds());
        }
        if let Some(runtime) = self.runtime.as_deref_mut() {
            if let Some((_, config)) = self.effective_config {
                runtime.config = config;
            }
            if runtime
                .renderer
                .update(runtime.config, self.frame, self.current_time)
            {
                let _ =
                    runtime
                        .driver
                        .paint(&runtime.renderer, hardware::quiesce, hardware::restart);
            }
        }
        let operational = self
            .runtime
            .as_deref()
            .is_some_and(|runtime| runtime.driver.is_operational());
        FloatOutBoyInternalLedAuxResult {
            runtime: self.runtime,
            retry_refresh,
            deferred_confirmation,
            operational,
        }
    }

    fn refresh_runtime(&mut self) -> bool {
        if !self
            .runtime
            .as_deref_mut()
            .is_none_or(|runtime| runtime.driver.destroy(hardware::teardown))
        {
            return false;
        }
        self.runtime = self
            .effective_config
            .filter(|(hardware, _)| hardware.uses_internal_leds())
            .and_then(|(hardware, config)| {
                let runtime = FloatOutBoyInternalLedRuntime {
                    renderer: FloatOutBoyLedRenderer::new(hardware, config, 0.0),
                    config,
                    driver: FloatOutBoyInternalLedDriver::new(hardware),
                };
                let mut runtime = vescpkg_rs::FallibleBox::try_new(runtime).ok()?;
                runtime
                    .driver
                    .setup(hardware::setup, |pin| {
                        let _ = hardware::teardown(pin);
                    })
                    .then_some(runtime)
            });
        true
    }
}

#[cfg(target_arch = "arm")]
impl FloatOutBoyInternalLedAuxResult {
    pub(in crate::package) fn destroy_after_rejected_commit(mut self) {
        let Some(mut runtime) = self.runtime.take() else {
            return;
        };
        if !runtime.driver.destroy(hardware::teardown) {
            // DMA still owns the pulse buffer. Leaking it during package stop is
            // the only memory-safe outcome when hardware refuses to quiesce.
            let _ = core::mem::ManuallyDrop::new(runtime);
        }
    }
}
