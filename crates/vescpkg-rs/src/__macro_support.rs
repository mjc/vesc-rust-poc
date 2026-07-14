//! Hidden implementation hooks for exported package macros.

#[cfg(not(test))]
pub use crate::firmware::__firmware_package_state_ptr;
#[cfg(not(any(test, feature = "test-support")))]
pub use crate::firmware::app_data_callback;
pub use crate::firmware::{PackageAppDataCallback, PackageCustomConfigCallback};
pub use crate::imu::{PackageImuReadCallback, imu_read_callback};
pub use crate::init::__package_start_from_raw;
