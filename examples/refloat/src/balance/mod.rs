//! Refloat balance math.
//!
//! Source map: upstream reads balance pitch through the Mahony filter in
//! `third_party/refloat/src/balance_filter.c`, then executes the RUNNING
//! balance-current path from `refloat_thd` at
//! `third_party/refloat/src/main.c:918-956`.

mod filter;
mod loop_io;

mod booster;
mod current;
mod pid;
mod step;

pub(crate) use filter::BalanceFilter;
pub(crate) use loop_io::{LoopConfig, LoopInput, LoopState};
