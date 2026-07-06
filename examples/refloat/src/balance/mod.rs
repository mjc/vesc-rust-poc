//! Refloat RUNNING balance-current calculation.
//!
//! Source map: upstream executes this path from `refloat_thd` at
//! `third_party/refloat/src/main.c:918-956`, with PID math in
//! `third_party/refloat/src/pid.c:37-73`, booster math in
//! `third_party/refloat/src/booster.c:32-75`, and pitch-rate input from
//! `third_party/refloat/src/imu.c:43-53`.

mod filter;
mod step;
mod types;

pub(crate) use filter::RefloatBalanceFilter;
pub(crate) use step::refloat_balance_loop_step;
pub(crate) use types::{
    RefloatBalanceLoopConfig, RefloatBalanceLoopInput, RefloatBalanceLoopState,
};
