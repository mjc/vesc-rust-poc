//! Native VESC package loader helpers shared across package payloads.

use crate::ffi;

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let _ = crate::Firmware::new().clear_package_callbacks();
    }

    #[cfg(test)]
    {
        record_stop_call_for_tests();
    }
}

/// Install the package stop hook into loader metadata.
fn install_stop_hook(info: *mut ffi::LibInfo) -> Result<(), PackageStartError> {
    let Some(info) = (unsafe { crate::loader_info_mut(info) }) else {
        return Err(PackageStartError::LoaderUnavailable);
    };
    info.stop_fun = Some(crate::firmware::stop_handler_for_loader(info, stop_package));
    Ok(())
}

/// Failure while preparing package startup against firmware loader metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStartError {
    /// The firmware did not provide loader metadata for this package.
    LoaderUnavailable,
}

#[doc(hidden)]
pub trait LoaderInfoPointer {
    fn into_loader_info(self) -> *mut crate::LoaderInfo;
}

impl LoaderInfoPointer for *mut crate::LoaderInfo {
    fn into_loader_info(self) -> *mut crate::LoaderInfo {
        self
    }
}

impl LoaderInfoPointer for &mut crate::LoaderInfo {
    fn into_loader_info(self) -> *mut crate::LoaderInfo {
        self
    }
}

#[cfg(any(test, feature = "test-support"))]
impl LoaderInfoPointer for &mut vescpkg_rs_sys::LibInfo {
    fn into_loader_info(self) -> *mut crate::LoaderInfo {
        (self as *mut vescpkg_rs_sys::LibInfo).cast()
    }
}

/// Safe startup context for package authors.
pub struct PackageStart {
    info: *mut crate::LoaderInfo,
}

/// Package-local app-data callback rebased into its loaded firmware image.
///
/// Construct this through [`PackageStart::app_data_callback`], then register it
/// after any callbacks that firmware requires to be installed first.
pub struct LoadedAppDataCallback {
    handler: ffi::AppDataHandler,
}

impl LoadedAppDataCallback {
    /// Register this callback with live firmware.
    ///
    /// C map: Refloat registers `on_command_received` after custom config at
    /// `third_party/refloat/src/main.c:2455-2456`; VESC validates and stores the
    /// callback at `third_party/vesc/comm/commands.c:1820-1828`.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn register(self) -> Result<(), crate::AppDataHandlerRegistrationError> {
        self.register_with_bindings(&crate::bindings::RealBindings)
    }

    /// Register this callback through explicit test bindings.
    #[cfg(test)]
    pub(crate) fn register_with<B: crate::bindings::AppDataBindings>(
        self,
        bindings: &B,
    ) -> Result<(), crate::AppDataHandlerRegistrationError> {
        self.register_with_bindings(bindings)
    }

    #[inline(always)]
    fn register_with_bindings<B: crate::bindings::AppDataBindings>(
        self,
        bindings: &B,
    ) -> Result<(), crate::AppDataHandlerRegistrationError> {
        unsafe { bindings.set_app_data_handler(self.handler) }
            .then_some(())
            .ok_or(crate::AppDataHandlerRegistrationError::FirmwareRejected)
    }
}

impl PackageStart {
    /// Build a startup context from the firmware ABI pointer.
    pub(crate) fn from_raw<I: LoaderInfoPointer>(info: I) -> Self {
        Self {
            info: info.into_loader_info(),
        }
    }

    fn raw_info_mut(&mut self) -> Option<&mut ffi::LibInfo> {
        unsafe { crate::loader_info_mut(self.info.cast()) }
    }

    /// Install the default package stop hook into loader metadata.
    ///
    /// C map: the loader stores this callback in `LibInfo.stop_fun` at
    /// `third_party/refloat/vesc_pkg_lib/vesc_c_if.h:675-677`; the matching
    /// package state is later exposed through `ARG` at `:698-699`.
    pub fn install_stop_hook(&mut self) -> Result<(), PackageStartError> {
        install_stop_hook(self.info.cast())
    }

    /// Borrow typed loader metadata.
    pub(crate) fn loader_info_mut(&mut self) -> Option<&mut crate::LoaderInfo> {
        (!self.info.is_null()).then(|| unsafe { &mut *self.info })
    }

    /// Borrow the loader-backed native image identity for this package.
    pub(crate) fn native_image(&mut self) -> Option<ffi::NativeImage> {
        self.raw_info_mut()
            .map(|info| ffi::NativeImage::from_info(&*info))
    }

    /// Register typed state-backed custom-config callbacks for this package.
    ///
    /// The callback macro supplies concrete package-local symbols. VESC validates and
    /// stores those rebased addresses at `third_party/vesc/conf_custom.c:34-42`.
    pub(crate) fn register_stateful_custom_config_with_bindings<B, T, const LEN: usize>(
        &mut self,
        bindings: &B,
    ) -> bool
    where
        B: crate::bindings::CustomConfigBindings,
        T: crate::PackageCustomConfigCallback<LEN>,
    {
        let Some(image) = self.native_image() else {
            return false;
        };
        let (get, set, xml) = T::image_addresses();
        unsafe {
            crate::register_custom_config_callbacks_from_image(
                bindings,
                image,
                core::mem::transmute::<usize, ffi::CustomConfigGet>(get),
                core::mem::transmute::<usize, ffi::CustomConfigSet>(set),
                core::mem::transmute::<usize, ffi::CustomConfigXml>(xml),
            )
        }
    }

    /// Return the loaded package program identity, when firmware supplied one.
    pub fn program(&mut self) -> Option<crate::PackageProgram> {
        self.loader_info_mut()
            .map(|info| crate::PackageProgram::new(info.program_address()))
    }

    /// Load a typed app-data callback from this package image.
    ///
    /// The callback macro keeps the ABI entry in the package image. Loading it
    /// before other firmware registrations preserves the package-local pointer
    /// that Refloat later passes to `set_app_data_handler`.
    ///
    /// C map: Refloat's `on_command_received` is a package-local function at
    /// `third_party/refloat/src/main.c:2142-2143` and is registered at `:2456`.
    #[inline(always)]
    pub fn app_data_callback<T: crate::PackageAppDataCallback>(
        &mut self,
    ) -> Option<LoadedAppDataCallback> {
        let image = self.native_image()?;
        let address = image.rebase_addr(T::image_address());
        let handler = unsafe { core::mem::transmute::<usize, ffi::AppDataHandler>(address) };
        Some(LoadedAppDataCallback { handler })
    }

    /// Register a concrete package-local typed IMU callback.
    ///
    /// C map: Refloat registers `imu_ref_callback` at
    /// `third_party/refloat/src/main.c:2454`; VESC stores and invokes it through
    /// `third_party/vesc/imu/imu.c:581-582` and `:704-727`.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn register_imu_read_callback<T: crate::PackageImuReadCallback>(
        &mut self,
    ) -> Result<(), PackageStartError> {
        self.register_imu_read_callback_with_bindings::<T, _>(&crate::bindings::RealBindings)
    }

    #[inline(always)]
    fn register_imu_read_callback_with_bindings<T, B>(
        &mut self,
        bindings: &B,
    ) -> Result<(), PackageStartError>
    where
        T: crate::PackageImuReadCallback,
        B: crate::bindings::ImuReadCallbackBindings,
    {
        let Some(image) = self.native_image() else {
            return Err(PackageStartError::LoaderUnavailable);
        };
        let address = image.rebase_addr(T::image_address());
        let callback = unsafe { core::mem::transmute::<usize, ffi::ImuReadCallback>(address) };
        bindings.set_imu_read_callback_handler(callback);
        Ok(())
    }

    /// Register this package's typed custom config and app-data callback.
    ///
    /// The app-data callback is rebased before either firmware call, while
    /// firmware receives custom config before app data.
    ///
    /// C map: Refloat registers custom config and then `on_command_received` at
    /// `third_party/refloat/src/main.c:2455-2456`. VESC stores the app-data
    /// callback at `third_party/vesc/comm/commands.c:1820-1828`.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn register_callbacks<C, A, const LEN: usize>(
        &mut self,
    ) -> Result<(), crate::AppDataHandlerRegistrationError>
    where
        C: crate::PackageCustomConfigCallback<LEN>,
        A: crate::PackageAppDataCallback,
    {
        self.register_callbacks_with_bindings::<C, A, LEN, _>(&crate::bindings::RealBindings)
    }

    #[inline(always)]
    fn register_callbacks_with_bindings<C, A, const LEN: usize, B>(
        &mut self,
        bindings: &B,
    ) -> Result<(), crate::AppDataHandlerRegistrationError>
    where
        C: crate::PackageCustomConfigCallback<LEN>,
        A: crate::PackageAppDataCallback,
        B: crate::bindings::AppDataBindings + crate::bindings::CustomConfigBindings,
    {
        let callback = self
            .app_data_callback::<A>()
            .ok_or(crate::AppDataHandlerRegistrationError::FirmwareRejected)?;
        if !self.register_stateful_custom_config_with_bindings::<B, C, LEN>(bindings) {
            return Err(crate::AppDataHandlerRegistrationError::FirmwareRejected);
        }
        if let Err(error) = callback.register_with_bindings(bindings) {
            let _ = unsafe { bindings.clear_custom_configs() };
            return Err(error);
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    /// Register extension descriptors using loader metadata for this package image.
    fn register_extensions_with_lifecycle<B: crate::bindings::LbmBindings>(
        &mut self,
        lifecycle: &crate::lifecycle_core::PackageLifecycle<B>,
        descriptors: impl IntoIterator<Item = crate::ExtensionDescriptor>,
    ) -> Result<(), crate::RegisterError> {
        let info = self
            .raw_info_mut()
            .ok_or(crate::RegisterError::LoaderUnavailable)?;
        // SAFETY: `info` is the loader metadata just installed for this package
        // image, so extension descriptor pointers are rebased against the image
        // that owns them.
        unsafe {
            lifecycle.register_extensions_from_image(ffi::NativeImage::from_info(info), descriptors)
        }
    }

    /// Register extension descriptors through the test-support lifecycle seam.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn register_extensions_with<B: crate::bindings::LbmBindings>(
        &mut self,
        lifecycle: &crate::lifecycle_core::PackageLifecycle<B>,
        descriptors: impl IntoIterator<Item = crate::ExtensionDescriptor>,
    ) -> Result<(), crate::RegisterError> {
        self.register_extensions_with_lifecycle(lifecycle, descriptors)
    }

    /// Register extension descriptors with the live firmware bindings.
    #[cfg(all(not(test), target_arch = "arm"))]
    pub fn register_extensions(
        &mut self,
        descriptors: impl IntoIterator<Item = crate::ExtensionDescriptor>,
    ) -> Result<(), crate::RegisterError> {
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(crate::bindings::RealBindings);
        self.register_extensions_with_lifecycle(&lifecycle, descriptors)
    }
}

/// Construct the startup context for the exported package entry trampoline.
#[doc(hidden)]
pub fn __package_start_from_raw(info: *mut crate::LoaderInfo) -> PackageStart {
    PackageStart::from_raw(info)
}

/// Define the VESC firmware entrypoints for a package start function.
#[macro_export]
macro_rules! package_start {
    ($start:path) => {
        #[cfg(all(not(test), target_arch = "arm"))]
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".program_ptr")]
        pub(crate) static prog_ptr: u32 = 0;

        /// Firmware loader entrypoint that runs the package start function.
        #[cfg(any(test, all(not(test), target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        extern "C" fn package_lib_init(info: *mut $crate::LoaderInfo) -> bool {
            let mut start = $crate::__macro_support::__package_start_from_raw(info);
            $start(&mut start)
        }

        /// Host-linking loader shim for package crates.
        #[cfg(all(not(test), not(target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        extern "C" fn package_lib_init(info: *mut $crate::LoaderInfo) -> bool {
            let mut start = $crate::__macro_support::__package_start_from_raw(info);
            let _ = start.install_stop_hook();
            true
        }

        /// ARM package initializer placed in the firmware init section.
        #[cfg(all(not(test), target_arch = "arm"))]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".init_fun")]
        pub extern "C" fn init(info: *mut $crate::LoaderInfo) -> bool {
            package_lib_init(info)
        }
    };
}

#[cfg(test)]
mod test_state {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static INIT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static STOP_CALLS: AtomicUsize = AtomicUsize::new(0);

    pub fn record_init_call() {
        INIT_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn record_stop_call() {
        STOP_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    pub fn reset() {
        INIT_CALLS.store(0, Ordering::SeqCst);
        STOP_CALLS.store(0, Ordering::SeqCst);
    }

    pub fn init_calls() -> usize {
        INIT_CALLS.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn stop_calls() -> usize {
        STOP_CALLS.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
fn record_stop_call_for_tests() {
    test_state::record_stop_call();
}

/// Test helper that mirrors the device `package_lib_init` stop-hook path.
#[cfg(test)]
pub fn init_for_tests(info: *mut ffi::LibInfo) -> Result<(), PackageStartError> {
    let result = install_stop_hook(info);
    test_state::record_init_call();
    result
}

/// Resets the package init call counter used by tests.
#[cfg(test)]
pub fn reset_init_call_count_for_tests() {
    test_state::reset();
}

/// Returns how many times the package init entrypoint has been called in tests.
#[cfg(test)]
pub fn init_call_count_for_tests() -> usize {
    test_state::init_calls()
}

/// Returns how many times the package stop hook has been called in tests.
#[cfg(test)]
pub fn stop_call_count_for_tests() -> usize {
    test_state::stop_calls()
}

#[cfg(test)]
mod tests {
    use super::{
        PackageStartError, init_for_tests, install_stop_hook, reset_init_call_count_for_tests,
    };
    use crate::ffi;
    use crate::test_support::FakeAppDataBindings;

    struct TestPackageImuRead;

    static TEST_IMU_STATE: crate::PackageStateStore<()> = crate::PackageStateStore::new();

    impl crate::ImuReadHandler for TestPackageImuRead {
        type State = ();

        fn state_source() -> crate::PackageStateAccess<'static, Self::State> {
            crate::PackageStateAccess::runtime(&TEST_IMU_STATE)
        }

        fn read(_state: &mut Self::State, _sample: crate::ImuReadSample) {}
    }

    impl crate::PackageImuReadCallback for TestPackageImuRead {
        fn image_address() -> usize {
            crate::test_support::stubs::imu_read_callback as *const () as usize
        }
    }

    #[test]
    fn package_init_records_device_initialization() {
        reset_init_call_count_for_tests();

        assert_eq!(
            init_for_tests(core::ptr::null_mut()),
            Err(PackageStartError::LoaderUnavailable)
        );
        assert_eq!(super::init_call_count_for_tests(), 1);
    }

    #[test]
    fn package_init_installs_a_stop_hook() {
        reset_init_call_count_for_tests();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert_eq!(init_for_tests(&mut info), Ok(()));

        let stop_fun = info.stop_fun.expect("stop hook");
        unsafe {
            stop_fun(info.arg);
        }
        assert_eq!(super::stop_call_count_for_tests(), 1);
    }

    #[test]
    fn install_stop_hook_rejects_null_loader_metadata() {
        assert_eq!(
            install_stop_hook(core::ptr::null_mut()),
            Err(PackageStartError::LoaderUnavailable)
        );
    }

    #[test]
    fn package_start_exposes_loader_native_image_identity() {
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut start = super::PackageStart::from_raw(&mut info);

        let image = start.native_image().expect("native image");
        assert_eq!(image.rebase_addr(0x31), 0x2031);

        let program = start.program().expect("package program");
        assert_eq!(program.address(), crate::PackageProgramAddress::new(0x2000));
        assert_eq!(
            start.program().map(|program| program.address().get()),
            Some(0x2000)
        );
    }

    #[test]
    fn package_start_registers_typed_app_data_callback_from_loader_image() {
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        struct Callback;

        impl crate::PackageAppDataCallback for Callback {
            fn image_address() -> usize {
                handler as *const () as usize
            }
        }

        let bindings = FakeAppDataBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_raw(&mut info);

        let callback = start
            .app_data_callback::<Callback>()
            .expect("loaded callback");
        assert_eq!(callback.register_with(&bindings), Ok(()));
        // C map: Refloat stores package state before registering its app-data
        // callback at `third_party/refloat/src/main.c:2431-2456`; VESC retains
        // the rebased callback at `third_party/vesc/comm/commands.c:1820-1828`.
        assert_eq!(bindings.handler_calls.get(), 1);
        assert_eq!(
            bindings.last_handler.get(),
            ffi::NativeImage::from_info(&info).rebase_addr(handler as *const () as usize)
        );
    }

    #[test]
    fn package_start_stops_callback_registration_when_custom_config_is_rejected() {
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        struct Callback;
        struct Config;

        static CONFIG_STATE: crate::PackageStateStore<()> = crate::PackageStateStore::new();

        impl crate::PackageAppDataCallback for Callback {
            fn image_address() -> usize {
                handler as *const () as usize
            }
        }

        impl crate::SourceCustomConfigCallback<1> for Config {
            type State = ();

            fn state_source() -> crate::PackageStateAccess<'static, Self::State> {
                crate::PackageStateAccess::runtime(&CONFIG_STATE)
            }

            fn default_config() -> crate::ConfigBytes<'static> {
                crate::ConfigBytes::new(&[0])
            }

            fn current_config(_state: &Self::State) -> Option<crate::ConfigBytes<'_>> {
                Some(crate::ConfigBytes::new(&[0]))
            }

            fn set_config(_state: &mut Self::State, _config: crate::ConfigBytes<'_>) -> bool {
                true
            }

            fn config_xml() -> crate::ConfigXml<'static> {
                crate::ConfigXml::new(b"<Config/>")
            }
        }
        crate::firmware_stateful_custom_config_callbacks!(
            test_config_get,
            test_config_set,
            test_config_xml,
            Config,
            1
        );

        let bindings = FakeAppDataBindings::with_register_custom_config_result(false);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.register_callbacks_with_bindings::<Config, Callback, 1, _>(&bindings),
            Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
        );
        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.handler_calls.get(), 0);

        let bindings = FakeAppDataBindings::with_set_handler_result(false);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.register_callbacks_with_bindings::<Config, Callback, 1, _>(&bindings),
            Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
        );
        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.handler_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 1);
    }

    #[test]
    fn package_start_registers_typed_imu_callback_from_loader_image() {
        let bindings = FakeAppDataBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let image = ffi::NativeImage::from_info(&info);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.register_imu_read_callback_with_bindings::<TestPackageImuRead, _>(&bindings),
            Ok(())
        );
        // C map: Refloat registers `imu_ref_callback` at
        // `third_party/refloat/src/main.c:2454`; VESC stores the rebased pointer
        // at `third_party/vesc/imu/imu.c:581-582`.
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);
        assert_eq!(
            bindings.last_imu_read_callback.get(),
            image.rebase_addr(
                <TestPackageImuRead as crate::PackageImuReadCallback>::image_address()
            )
        );
    }

    #[test]
    fn package_start_rejects_imu_callback_without_loader_metadata() {
        let bindings = FakeAppDataBindings::new();
        let mut start = super::PackageStart::from_raw(core::ptr::null_mut());

        assert_eq!(
            start.register_imu_read_callback_with_bindings::<TestPackageImuRead, _>(&bindings),
            Err(PackageStartError::LoaderUnavailable)
        );
        assert_eq!(bindings.imu_read_callback_calls.get(), 0);
    }

    #[test]
    fn package_start_registers_extensions_from_loader_metadata() {
        use crate::test_support::{FakeBindings, stubs};

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let bindings = FakeBindings::new();
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);
        let descriptor = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-start-probe"),
            stubs::extension_handler,
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [descriptor]),
            Ok(())
        );
        assert_eq!(bindings.add_calls.get(), 1);
        assert_eq!(
            bindings.last_name.get(),
            descriptor.name().as_cstr().as_ptr() as usize
        );
        assert_eq!(
            bindings.last_handler.get(),
            descriptor.handler() as usize + 0x2000
        );

        let mut rejected_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let rejecting_bindings = FakeBindings::rejecting();
        let rejecting_lifecycle = crate::lifecycle_core::PackageLifecycle::new(&rejecting_bindings);
        let mut rejecting_start = super::PackageStart::from_raw(&mut rejected_info);

        assert_eq!(
            rejecting_start.register_extensions_with(&rejecting_lifecycle, [descriptor]),
            Err(crate::RegisterError::FirmwareRejected)
        );
    }
}
