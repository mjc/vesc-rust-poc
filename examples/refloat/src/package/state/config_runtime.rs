use super::RefloatPackageState;
use crate::domain::{
    RefloatAllDataBasePayload, RefloatAllDataPayloads, RefloatAllDataStatus, RefloatRideState,
    RefloatRunState,
};

pub(super) fn refresh(state: &mut RefloatPackageState) {
    let payloads = state.all_data_payloads;
    let base = payloads.base();
    let status = base.status();
    let ride_state = status.ride_state();
    let disabled = state.serialized_config.metadata().disabled();
    let run_state = match (ride_state.run_state(), disabled) {
        // C map: Refloat applies `float_conf.disabled` from `configure(d)` at
        // `third_party/refloat/src/main.c:184-190`; `state_set_disabled`
        // keeps RUNNING alive and toggles DISABLED/STARTUP at
        // `third_party/refloat/src/state.c:41-47`.
        (RefloatRunState::Running, true) => RefloatRunState::Running,
        (RefloatRunState::Disabled, false) => RefloatRunState::Startup,
        (_, true) => RefloatRunState::Disabled,
        (run_state, false) => run_state,
    };
    if run_state == ride_state.run_state() {
        return;
    }

    let ride_state = RefloatRideState::new(
        run_state,
        ride_state.mode(),
        ride_state.setpoint_adjustment(),
        ride_state.stop_condition(),
    )
    .with_charging(ride_state.charging())
    .with_wheelslip(ride_state.wheelslip())
    .with_darkride(ride_state.darkride());
    let base = RefloatAllDataBasePayload::new(
        base.balance_current(),
        base.attitude(),
        RefloatAllDataStatus::new(ride_state, status.beep_reason()),
        base.footpad(),
        base.setpoints(),
        base.booster_current(),
        base.motor(),
    );
    state.all_data_payloads =
        RefloatAllDataPayloads::new(base, payloads.mode2(), payloads.mode3(), payloads.mode4());
}
