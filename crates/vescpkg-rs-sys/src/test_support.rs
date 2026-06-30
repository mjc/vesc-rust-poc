//! Injectable mock `VescIf` tables for host-side dispatch tests.

use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::raw::VescIf;

static CURRENT_TABLE: AtomicPtr<VescIf> = AtomicPtr::new(ptr::null_mut());

/// Returns a zeroed function table suitable for stub installation.
pub fn empty_table() -> VescIf {
    unsafe { core::mem::zeroed() }
}

/// Install `table` as the active firmware function table for subsequent `raw::*` calls.
pub fn set_table(table: *const VescIf) {
    CURRENT_TABLE.store(table.cast_mut(), Ordering::SeqCst);
}

/// Clear any installed mock table.
pub fn clear_table() {
    CURRENT_TABLE.store(ptr::null_mut(), Ordering::SeqCst);
}

/// Returns the installed mock table, if any.
pub fn current_table() -> Option<*const VescIf> {
    let table = CURRENT_TABLE.load(Ordering::SeqCst);
    if table.is_null() { None } else { Some(table) }
}

struct MockGuard;

impl Drop for MockGuard {
    fn drop(&mut self) {
        clear_table();
    }
}

/// Run `body` with `table` installed as the active `VescIf` pointer.
pub fn with_table<R>(table: &VescIf, body: impl FnOnce() -> R) -> R {
    set_table(table);
    let _guard = MockGuard;
    body()
}

#[cfg(test)]
mod tests {
    use super::{clear_table, current_table, empty_table, with_table};
    use crate::raw::VescIf;

    #[test]
    fn installs_and_clears_mock_table() {
        let table = empty_table();
        assert!(current_table().is_none());

        with_table(&table, || {
            assert_eq!(current_table(), Some(&table as *const VescIf));
        });

        assert!(current_table().is_none());
        clear_table();
    }
}
