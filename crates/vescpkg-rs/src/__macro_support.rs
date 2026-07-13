//! Hidden implementation hooks for exported package macros.

#[cfg(not(any(test, feature = "test-support")))]
pub use crate::firmware::app_data_callback;
#[cfg(not(test))]
pub use crate::firmware::{__firmware_package_state_mut, __firmware_package_state_ptr};
pub use crate::imu::imu_read_callback;
pub use crate::init::__package_start_from_raw;
