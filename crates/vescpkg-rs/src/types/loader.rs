//! Native-package loader semantic wrappers.

use core::ffi::c_void;
use core::ptr::NonNull;

/// Loader metadata exposed to package code as a typed Rust context.
///
/// The representation is ABI-compatible with the firmware loader, but its
/// fields remain private. Package code should use the semantic accessors rather
/// than manipulating the loader layout.
#[repr(transparent)]
pub struct LoaderInfo(crate::ffi::LibInfo);

impl LoaderInfo {
    /// Create empty loader metadata for host-side tests.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub const fn new() -> Self {
        Self(crate::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        })
    }

    /// Return the package argument in host-side loader tests.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn argument(&self) -> Option<PackageArgument> {
        NonNull::new(self.0.arg).map(PackageArgument::new)
    }

    /// Return the loaded package program address.
    #[must_use]
    pub const fn program_address(&self) -> PackageProgramAddress {
        PackageProgramAddress::new(self.0.base_addr)
    }

    /// Report whether a stop callback is installed.
    #[must_use]
    pub const fn has_stop_handler(&self) -> bool {
        self.0.stop_fun.is_some()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn stop_for_test(&mut self) -> bool {
        self.0.stop_fun.take().is_some_and(|stop| {
            unsafe { stop(self.0.arg) };
            true
        })
    }

    /// Set the package argument for host-side loader tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_argument(&mut self, argument: Option<PackageArgument>) {
        self.0.arg = argument.map_or(core::ptr::null_mut(), PackageArgument::as_ptr);
    }

    /// Set the loaded package program address for host-side loader tests.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn set_program_address(&mut self, address: PackageProgramAddress) {
        self.0.base_addr = address.get();
    }

    #[cfg(test)]
    pub(crate) fn set_stop_handler(&mut self, stop_handler: crate::ffi::StopHandler) {
        self.0.stop_fun = Some(crate::firmware::stop_handler_for_loader(
            &self.0,
            stop_handler,
        ));
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Default for LoaderInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime address of the loaded native package program.
///
/// C map: this is the Rust package-author wrapper for `PROG_ADDR` from
/// `third_party/vesc_pkg_lib/vesc_c_if.h:697`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PackageProgramAddress(u32);

impl PackageProgramAddress {
    /// Wrap a firmware-provided package program address for macro expansion.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn new(address: u32) -> Self {
        Self(address)
    }

    /// Extract the loader address for internal ABI setup.
    #[inline]
    #[must_use]
    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// Loaded native package program identity.
///
/// This is the package-author-facing handle for VESC's `PROG_ADDR`; use it to
/// recover package-owned state instead of caching raw loader addresses.
///
/// C map: wraps `PROG_ADDR` from `third_party/vesc_pkg_lib/vesc_c_if.h:697`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PackageProgram(PackageProgramAddress);

impl PackageProgram {
    /// Wrap a firmware-provided package program address for macro expansion.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn new(address: PackageProgramAddress) -> Self {
        Self(address)
    }

    /// Return the typed program address for this package.
    #[inline]
    #[must_use]
    pub const fn address(self) -> PackageProgramAddress {
        self.0
    }
}

impl From<PackageProgramAddress> for PackageProgram {
    #[inline]
    fn from(address: PackageProgramAddress) -> Self {
        Self::new(address)
    }
}

/// Loader argument attached to a native package program.
///
/// C map: this is the Rust package-author wrapper for `ARG` from
/// `third_party/vesc_pkg_lib/vesc_c_if.h:700`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackageArgument(NonNull<c_void>);

impl PackageArgument {
    /// Wrap a non-null firmware ARG pointer.
    #[inline]
    #[must_use]
    pub(crate) const fn new(argument: NonNull<c_void>) -> Self {
        Self(argument)
    }

    /// Explicitly extract the raw firmware ARG pointer.
    #[cfg(any(test, feature = "test-support"))]
    #[inline]
    #[must_use]
    pub(crate) const fn as_ptr(self) -> *mut c_void {
        self.0.as_ptr()
    }

    /// View ARG as a typed package-state pointer.
    ///
    /// # Safety
    ///
    /// The firmware ARG must point to a live `T`. Use a scoped SDK callback
    /// helper rather than dereferencing this pointer directly.
    #[inline]
    #[must_use]
    pub(crate) unsafe fn state_ptr<T: 'static>(self) -> NonNull<T> {
        self.0.cast()
    }
}
