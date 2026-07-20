//! Hidden implementation hooks for exported package macros.

/// Mark a function symbol whose PIC reference may materialize through the GOT.
#[doc(hidden)]
#[macro_export]
macro_rules! __vescpkg_image_offset {
    ($name:ident) => {
        #[cfg(all(not(test), target_arch = "arm"))]
        core::arch::global_asm!(
            concat!(".global __vescpkg_image_offset_", stringify!($name)),
            concat!(
                ".set __vescpkg_image_offset_",
                stringify!($name),
                ", ",
                stringify!($name)
            ),
        );
    };
}

/// Build package state access from a macro-generated firmware lookup.
///
/// # Safety
///
/// `firmware_state` must return the live state installed in `runtime`, and that state must
/// remain valid for the duration of each callback.
#[doc(hidden)]
pub const unsafe fn __package_state_access<T: Send + 'static>(
    runtime: &crate::PackageStateStore<T>,
    firmware_state: unsafe fn() -> Option<core::ptr::NonNull<T>>,
) -> crate::PackageStateAccess<'_, T> {
    unsafe { crate::PackageStateAccess::with_firmware_fallback(runtime, firmware_state) }
}

#[cfg(not(test))]
pub use crate::firmware::__firmware_package_state_ptr;
#[cfg(not(any(test, feature = "test-support")))]
pub use crate::firmware::app_data_callback;
pub use crate::firmware::{PackageAppDataCallback, PackageCustomConfigCallback};
pub use crate::imu::{PackageImuReadCallback, imu_read_callback};
pub use crate::init::__package_start_from_raw;
pub use crate::types::loader::{LoaderInfo, PackageProgramAddress};
