use super::FloatOutBoyPackageState;
use crate::domain::FloatOutBoyRunState;

pub(super) fn refresh(state: &mut FloatOutBoyPackageState) {
    state
        .beeper
        .set_enabled(state.serialized_config.beeper_enabled());
    refresh_disabled_state(state);
}

pub(super) fn refresh_disabled_state(state: &mut FloatOutBoyPackageState) {
    let payloads = state.all_data_payloads;
    let ride_state = payloads.ride_state();
    let disabled = state.serialized_config.metadata().disabled();
    let run_state = match (ride_state.run_state(), disabled) {
        // C map: Float Out Boy applies `float_conf.disabled` from `configure(d)` at
        // `third_party/float-out-boy/src/main.c:184-190`; `state_set_disabled`
        // keeps RUNNING alive and toggles DISABLED/STARTUP at
        // `third_party/float-out-boy/src/state.c:41-47`.
        (FloatOutBoyRunState::Running, true) => FloatOutBoyRunState::Running,
        (FloatOutBoyRunState::Disabled, false) => FloatOutBoyRunState::Startup,
        (_, true) => FloatOutBoyRunState::Disabled,
        (run_state, false) => run_state,
    };
    if run_state == ride_state.run_state() {
        return;
    }

    state.all_data_payloads = payloads.with_ride_state(ride_state.with_run_state(run_state));
}

pub(super) fn refresh_leds(state: &mut FloatOutBoyPackageState) {
    state
        .lcm
        .set_hardware_mode(state.serialized_config.hardware_led_mode());
    if let Some((_, config)) = state.effective_led_config() {
        state.lcm.configure(config);
    }
    state.request_internal_led_refresh();
}

pub(super) fn refresh_led_effects(state: &mut FloatOutBoyPackageState) {
    if let Some((_, config)) = state.effective_led_config() {
        state.lcm.configure(config);
        state.update_internal_led_config(config);
    }
}
