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
    /// Firmware memory could not hold the package state.
    AllocationFailed,
}

type FirmwareRuntimeStateGuard<'a, T, A> =
    PackageLoaderRuntimeStateGuard<'a, FirmwareLoaderStateGuard<'a, T, A>, T>;

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

/// Loader metadata installation cleared on drop unless committed.
pub struct PackageLoaderStateGuard {
    info: *mut ffi::LibInfo,
    committed: bool,
}

impl PackageLoaderStateGuard {
    fn new(info: *mut ffi::LibInfo) -> Self {
        Self {
            info,
            committed: false,
        }
    }

    /// Keep the installed loader metadata after the guard is dropped.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for PackageLoaderStateGuard {
    fn drop(&mut self) {
        if !self.committed {
            unsafe { crate::clear_loader_info(self.info) };
        }
    }
}

/// Firmware-allocated loader state cleared and freed on drop unless committed.
pub struct FirmwareLoaderStateGuard<'a, T, B: crate::alloc::AllocBindings> {
    info: *mut ffi::LibInfo,
    allocation: Option<crate::alloc::FirmwareAllocation<'a, T, B>>,
    committed: bool,
}

impl<'a, T, B: crate::alloc::AllocBindings> FirmwareLoaderStateGuard<'a, T, B> {
    fn new(
        info: *mut ffi::LibInfo,
        allocation: crate::alloc::FirmwareAllocation<'a, T, B>,
    ) -> Self {
        Self {
            info,
            allocation: Some(allocation),
            committed: false,
        }
    }

    /// Mutably borrow the installed firmware-owned state before commit.
    pub fn state_mut(&mut self) -> Option<&mut T> {
        let allocation = self.allocation.as_mut()?;
        Some(unsafe { &mut *allocation.as_mut_ptr() })
    }

    /// Keep the installed loader metadata and transfer allocation ownership to firmware.
    pub fn commit(mut self) {
        if let Some(allocation) = self.allocation.take() {
            let _ = allocation.into_raw();
        }
        self.committed = true;
    }
}

impl<T, B: crate::alloc::AllocBindings> Drop for FirmwareLoaderStateGuard<'_, T, B> {
    fn drop(&mut self) {
        if !self.committed {
            unsafe { crate::clear_loader_info(self.info) };
        }
    }
}

/// Loader metadata and runtime-slot state cleared on drop unless committed.
pub struct PackageLoaderRuntimeStateGuard<'a, L, T: 'static> {
    runtime: crate::PackageStateGuard<'a, T>,
    loader: L,
}

impl PackageLoaderRuntimeStateGuard<'_, PackageLoaderStateGuard, ()> {
    #[inline(always)]
    fn new<T: 'static>(
        loader: PackageLoaderStateGuard,
        runtime: crate::PackageStateGuard<'_, T>,
    ) -> PackageLoaderRuntimeStateGuard<'_, PackageLoaderStateGuard, T> {
        PackageLoaderRuntimeStateGuard { runtime, loader }
    }
}

impl<T: 'static> PackageLoaderRuntimeStateGuard<'_, PackageLoaderStateGuard, T> {
    /// Keep the installed loader metadata and runtime slot state.
    #[inline(always)]
    pub fn commit(self) {
        let Self { loader, runtime } = self;
        runtime.commit();
        loader.commit();
    }
}

impl<'a, T: 'static, B: crate::alloc::AllocBindings>
    PackageLoaderRuntimeStateGuard<'a, FirmwareLoaderStateGuard<'a, T, B>, T>
{
    #[inline(always)]
    fn new_firmware(
        loader: FirmwareLoaderStateGuard<'a, T, B>,
        runtime: crate::PackageStateGuard<'a, T>,
    ) -> Self {
        Self { runtime, loader }
    }

    /// Mutably borrow the installed firmware-owned state before commit.
    #[inline(always)]
    pub fn state_mut(&mut self) -> Option<&mut T> {
        self.loader.state_mut()
    }

    /// Keep the installed loader metadata, runtime slot state, and firmware allocation.
    #[inline(always)]
    pub fn commit(self) {
        let Self { loader, runtime } = self;
        runtime.commit();
        loader.commit();
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

    /// Run an operation with typed package state stored in loader metadata.
    ///
    /// # Safety
    ///
    /// Loader metadata must contain a live, exclusively borrowed `T`.
    pub unsafe fn with_state<T: 'static, R>(
        &mut self,
        operation: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        self.raw_info_mut()
            .and_then(|info| unsafe { crate::loader_state_mut::<T>(info) })
            .map(operation)
    }

    /// Run an operation with loader-owned state that may start package threads.
    #[inline(always)]
    ///
    /// # Safety
    ///
    /// Loader metadata must contain a live `T` that remains exclusively owned
    /// by the spawned stateful thread until it exits.
    pub unsafe fn with_thread_state<T: 'static, R>(
        &mut self,
        operation: impl FnOnce(PackageThreadState<'_, T>) -> R,
    ) -> Option<R> {
        self.raw_info_mut()
            .and_then(|info| unsafe { crate::loader_state_mut::<T>(info) })
            .map(|state| operation(PackageThreadState { state }))
    }

    /// Borrow typed package state stored in loader metadata.
    #[cfg(test)]
    pub(crate) unsafe fn loader_state_mut<T: 'static>(&mut self) -> Option<&mut T> {
        unsafe { crate::loader_state_mut(self.raw_info_mut()?) }
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

    /// Clear package state and stop metadata after a startup failure.
    pub(crate) fn clear_loader_info(&mut self) {
        unsafe { crate::clear_loader_info(self.info.cast()) };
    }

    /// Store package state and install a typed stop callback in loader metadata.
    ///
    /// C map: this writes the package state pointer and stop callback into the
    /// loader slots declared at `third_party/refloat/vesc_pkg_lib/vesc_c_if.h:675-677`.
    #[cfg(any(test, feature = "test-support"))]
    #[inline(always)]
    pub fn install_state<S: crate::StopCallback>(
        &mut self,
        state: &mut S::State,
    ) -> Result<(), PackageStartError> {
        self.install_state_with_handler(crate::stop_callback::<S>, state)
            .then_some(())
            .ok_or(PackageStartError::LoaderUnavailable)
    }

    /// Store package state and a stop hook in loader metadata.
    pub(crate) fn install_state_with_handler<T>(
        &mut self,
        stop_handler: ffi::StopHandler,
        state: &mut T,
    ) -> bool {
        if self.loader_info_mut().is_none() {
            return false;
        }
        unsafe { crate::install_loader_state(self.info.cast(), stop_handler, state) }
    }

    /// Store package state and clear it automatically unless the guard commits.
    pub(crate) fn install_state_guard_with_handler<T>(
        &mut self,
        stop_handler: ffi::StopHandler,
        state: &mut T,
    ) -> Option<PackageLoaderStateGuard> {
        self.install_state_with_handler(stop_handler, state)
            .then(|| PackageLoaderStateGuard::new(self.info.cast()))
    }

    /// Store package state and a typed stop callback in loader metadata and the runtime slot.
    #[inline(always)]
    pub fn install_runtime_state_guard<'a, S: crate::StopCallback>(
        &mut self,
        state: &'a mut S::State,
        runtime: &'a crate::PackageStateStore<S::State>,
    ) -> Option<PackageLoaderRuntimeStateGuard<'a, PackageLoaderStateGuard, S::State>> {
        self.install_runtime_state_guard_with_handler(crate::stop_callback::<S>, state, runtime)
    }

    /// Store package state in loader metadata and the runtime slot.
    ///
    /// Both are cleared unless the returned guard commits.
    #[inline(always)]
    pub(crate) fn install_runtime_state_guard_with_handler<'a, T: 'static>(
        &mut self,
        stop_handler: ffi::StopHandler,
        state: &'a mut T,
        runtime: &'a crate::PackageStateStore<T>,
    ) -> Option<PackageLoaderRuntimeStateGuard<'a, PackageLoaderStateGuard, T>> {
        let loader = self.install_state_guard_with_handler(stop_handler, state)?;
        let runtime = unsafe { runtime.install_guard(state) };
        Some(PackageLoaderRuntimeStateGuard::new(loader, runtime))
    }

    /// Store package state and a typed stop callback in loader metadata and the runtime slot.
    ///
    /// C map: this publishes `stop_fun` and `arg` in the loader slots at
    /// `third_party/refloat/vesc_pkg_lib/vesc_c_if.h:675-677`.
    #[inline(always)]
    pub fn install_runtime_state<S: crate::StopCallback>(
        &mut self,
        state: &mut S::State,
        runtime: &crate::PackageStateStore<S::State>,
    ) -> Result<(), PackageStartError> {
        self.install_runtime_state_with_handler(crate::stop_callback::<S>, state, runtime)
            .then_some(())
            .ok_or(PackageStartError::LoaderUnavailable)
    }

    /// Store package state in loader metadata and the runtime slot.
    #[inline(always)]
    pub(crate) fn install_runtime_state_with_handler<T: 'static>(
        &mut self,
        stop_handler: ffi::StopHandler,
        state: &mut T,
        runtime: &crate::PackageStateStore<T>,
    ) -> bool {
        if !self.install_state_with_handler(stop_handler, state) {
            return false;
        }
        unsafe { runtime.install(state) };
        true
    }

    /// Allocate package state in firmware memory and store it in loader metadata.
    #[cfg_attr(any(test, feature = "test-support"), allow(dead_code))]
    pub(crate) fn allocate_state_with_handler<A, T>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'_, A>,
        stop_handler: ffi::StopHandler,
        state: T,
    ) -> Result<(), PackageStartError>
    where
        A: crate::alloc::AllocBindings,
    {
        let Ok(mut allocation) = allocator.allocate_for::<T>(1) else {
            self.clear_loader_info();
            return Err(PackageStartError::AllocationFailed);
        };
        let state = allocation.write_first(state);

        if !self.install_state_with_handler(stop_handler, state) {
            self.clear_loader_info();
            return Err(PackageStartError::LoaderUnavailable);
        }

        let _ = allocation.into_raw();
        Ok(())
    }

    /// Allocate package state with firmware memory and clear it unless committed.
    #[cfg(not(any(test, feature = "test-support")))]
    #[inline(always)]
    pub fn allocate_state_guard<S: crate::StopCallback>(
        &mut self,
        state: S::State,
    ) -> Option<FirmwareLoaderStateGuard<'static, S::State, crate::bindings::RealBindings>>
    where
        S::State: 'static,
    {
        let allocator = crate::alloc::FirmwareAllocator::live();
        self.allocate_state_guard_with_handler(
            &allocator,
            crate::firmware::owned_stop_callback::<S>,
            state,
        )
    }

    /// Allocate package state through fake bindings for host-side tests.
    #[cfg(any(test, feature = "test-support"))]
    #[inline(always)]
    pub fn allocate_state_guard_with<'a, S, A>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'a, A>,
        state: S::State,
    ) -> Option<FirmwareLoaderStateGuard<'a, S::State, A>>
    where
        S: crate::StopCallback,
        S::State: 'static,
        A: crate::alloc::AllocBindings,
    {
        self.allocate_state_guard_with_handler(
            allocator,
            crate::firmware::owned_stop_callback::<S>,
            state,
        )
    }

    /// Allocate package state and clear/free it automatically unless the guard commits.
    pub(crate) fn allocate_state_guard_with_handler<'a, A, T>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'a, A>,
        stop_handler: ffi::StopHandler,
        state: T,
    ) -> Option<FirmwareLoaderStateGuard<'a, T, A>>
    where
        A: crate::alloc::AllocBindings,
    {
        let Ok(mut allocation) = allocator.allocate_for::<T>(1) else {
            self.clear_loader_info();
            return None;
        };
        {
            let state = allocation.write_first(state);
            if !self.install_state_with_handler(stop_handler, state) {
                self.clear_loader_info();
                return None;
            }
        }
        Some(FirmwareLoaderStateGuard::new(self.info.cast(), allocation))
    }

    /// Allocate package state and publish it in firmware and the runtime slot.
    #[cfg(not(any(test, feature = "test-support")))]
    #[inline(always)]
    pub fn allocate_runtime_state_guard<'a, S>(
        &mut self,
        state: S::State,
        runtime: &'a crate::PackageStateStore<S::State>,
    ) -> Option<FirmwareRuntimeStateGuard<'a, S::State, crate::bindings::RealBindings>>
    where
        S: crate::StopCallback,
        S::State: 'static,
    {
        let allocator = crate::alloc::FirmwareAllocator::live();
        self.allocate_runtime_state_guard_with_handler(
            &allocator,
            crate::firmware::owned_stop_callback::<S>,
            state,
            runtime,
        )
    }

    /// Allocate package state through fake bindings for host-side tests.
    #[cfg(any(test, feature = "test-support"))]
    #[inline(always)]
    pub fn allocate_runtime_state_guard_with<'a, S, A>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'a, A>,
        state: S::State,
        runtime: &'a crate::PackageStateStore<S::State>,
    ) -> Option<FirmwareRuntimeStateGuard<'a, S::State, A>>
    where
        S: crate::StopCallback,
        S::State: 'static,
        A: crate::alloc::AllocBindings,
    {
        self.allocate_runtime_state_guard_with_handler(
            allocator,
            crate::firmware::owned_stop_callback::<S>,
            state,
            runtime,
        )
    }

    /// Allocate package state, then publish it through loader metadata and the runtime slot.
    #[cfg_attr(any(test, feature = "test-support"), allow(dead_code))]
    ///
    /// Loader metadata, runtime slot state, and the firmware allocation are cleared/freed unless
    /// the returned guard commits.
    #[inline(always)]
    pub(crate) fn allocate_runtime_state_guard_with_handler<'a, A, T: 'static>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'a, A>,
        stop_handler: ffi::StopHandler,
        state: T,
        runtime: &'a crate::PackageStateStore<T>,
    ) -> Option<PackageLoaderRuntimeStateGuard<'a, FirmwareLoaderStateGuard<'a, T, A>, T>>
    where
        A: crate::alloc::AllocBindings,
    {
        let mut loader = self.allocate_state_guard_with_handler(allocator, stop_handler, state)?;
        let state = loader.state_mut()?;
        let runtime = unsafe { runtime.install_guard(state) };
        Some(PackageLoaderRuntimeStateGuard::new_firmware(
            loader, runtime,
        ))
    }

    /// Allocate package state and publish it in firmware and the runtime slot.
    #[cfg(not(any(test, feature = "test-support")))]
    #[inline(always)]
    pub fn allocate_runtime_state<S: crate::StopCallback>(
        &mut self,
        state: S::State,
        runtime: &crate::PackageStateStore<S::State>,
    ) -> Result<(), PackageStartError>
    where
        S::State: 'static,
    {
        let allocator = crate::alloc::FirmwareAllocator::live();
        self.allocate_runtime_state_with_handler(
            &allocator,
            crate::firmware::owned_stop_callback::<S>,
            state,
            runtime,
        )
    }

    /// Allocate package state, then publish it through loader metadata and the runtime slot.
    #[inline(always)]
    #[cfg_attr(any(test, feature = "test-support"), allow(dead_code))]
    pub(crate) fn allocate_runtime_state_with_handler<A, T: 'static>(
        &mut self,
        allocator: &crate::alloc::FirmwareAllocator<'_, A>,
        stop_handler: ffi::StopHandler,
        state: T,
        runtime: &crate::PackageStateStore<T>,
    ) -> Result<(), PackageStartError>
    where
        A: crate::alloc::AllocBindings,
    {
        self.allocate_state_with_handler(allocator, stop_handler, state)?;
        let info = self
            .raw_info_mut()
            .ok_or(PackageStartError::LoaderUnavailable)?;
        let state = unsafe { crate::loader_state_mut::<T>(info) }
            .ok_or(PackageStartError::LoaderUnavailable)?;
        unsafe { runtime.install(state) };
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

/// Loader-owned package state available during thread startup.
pub struct PackageThreadState<'a, T> {
    state: &'a mut T,
}

impl<T> core::ops::Deref for PackageThreadState<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<T> core::ops::DerefMut for PackageThreadState<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<T> PackageThreadState<'_, T> {
    /// Spawn package threads using this loader-owned state.
    ///
    /// C map: Refloat passes its published `Data *` to both thread spawns at
    /// `third_party/refloat/src/main.c:2438-2444`; VESC forwards the pointer
    /// unchanged at `third_party/vesc/lispBM/lispif_c_lib.c:98-125`.
    #[inline(always)]
    pub fn spawn_thread_pair(
        &mut self,
        threads: &impl crate::FirmwareThreads,
        pair: crate::ThreadPairSpec<T>,
    ) -> Option<crate::ThreadPair> {
        // SAFETY: PackageStart created this borrow from loader metadata, whose
        // state allocation remains stable until package stop.
        unsafe { crate::thread::spawn_thread_pair_with_state(threads, pair, self.state) }
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
    use core::cell::Cell;
    use core::ffi::c_void;
    use core::mem::MaybeUninit;

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

    struct TestAllocBindings {
        malloc_calls: Cell<usize>,
        free_calls: Cell<usize>,
        last_requested_len: Cell<usize>,
        next_ptr: Cell<*mut c_void>,
    }

    impl TestAllocBindings {
        fn new(next_ptr: *mut c_void) -> Self {
            Self {
                malloc_calls: Cell::new(0),
                free_calls: Cell::new(0),
                last_requested_len: Cell::new(0),
                next_ptr: Cell::new(next_ptr),
            }
        }

        fn failing() -> Self {
            Self::new(core::ptr::null_mut())
        }
    }

    impl crate::alloc::AllocBindings for TestAllocBindings {
        unsafe fn malloc(&self, bytes: usize) -> *mut c_void {
            self.malloc_calls.set(self.malloc_calls.get() + 1);
            self.last_requested_len.set(bytes);
            self.next_ptr.get()
        }

        unsafe fn free(&self, _ptr: *mut c_void) {
            self.free_calls.set(self.free_calls.get() + 1);
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
    fn package_start_context_installs_loader_state_without_raw_pointer() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(start.install_state_with_handler(super::stop_package, &mut state));
        assert!(
            start
                .loader_info_mut()
                .is_some_and(|info| info.has_stop_handler())
        );
        let loaded = unsafe { start.loader_state_mut::<State>() }.expect("loader state");
        assert_eq!(loaded.value, 42);

        start.clear_loader_info();
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_borrows_typed_loader_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(start.install_state_with_handler(super::stop_package, &mut state));
        unsafe { start.loader_state_mut::<State>() }
            .expect("state")
            .value = 7;

        assert_eq!(state, State { value: 7 });
    }

    #[test]
    fn package_start_loader_state_rejects_null_arg() {
        struct State;

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(unsafe { start.loader_state_mut::<State>() }.is_none());
    }

    #[test]
    fn package_start_loader_state_rejects_null_metadata() {
        let mut start = super::PackageStart::from_raw(core::ptr::null_mut());

        assert!(unsafe { start.loader_state_mut::<u32>() }.is_none());
    }

    #[test]
    fn package_start_loader_state_guard_clears_uncommitted_metadata() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        {
            let _guard = start
                .install_state_guard_with_handler(super::stop_package, &mut state)
                .expect("loader state guard");
            assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
            assert!(info.stop_fun.is_some());
        }

        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_loader_state_guard_keeps_committed_metadata() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        let guard = start
            .install_state_guard_with_handler(super::stop_package, &mut state)
            .expect("loader state guard");
        guard.commit();

        assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
        assert!(info.stop_fun.is_some());
    }

    #[test]
    fn package_start_loader_runtime_state_guard_clears_uncommitted_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let state_ptr = core::ptr::from_mut(&mut state).cast::<c_void>();
        let mut start = super::PackageStart::from_raw(&mut info);

        {
            let _guard = start
                .install_runtime_state_guard_with_handler(super::stop_package, &mut state, &runtime)
                .expect("loader runtime state guard");
            assert_eq!(runtime.with(|state| state.value), Some(42));
            assert_eq!(info.arg, state_ptr);
            assert!(info.stop_fun.is_some());
        }

        assert!(!runtime.is_installed());
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_loader_runtime_state_installs_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let state_ptr = core::ptr::from_mut(&mut state).cast::<c_void>();
        let mut start = super::PackageStart::from_raw(&mut info);

        assert!(start.install_runtime_state_with_handler(
            super::stop_package,
            &mut state,
            &runtime
        ));

        assert_eq!(runtime.with(|state| state.value), Some(42));
        assert_eq!(info.arg, state_ptr);
        assert!(info.stop_fun.is_some());
        runtime.clear();
    }

    #[test]
    fn package_start_loader_runtime_state_guard_keeps_committed_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut state = State { value: 42 };
        let mut start = super::PackageStart::from_raw(&mut info);

        let guard = start
            .install_runtime_state_guard_with_handler(super::stop_package, &mut state, &runtime)
            .expect("loader runtime state guard");
        guard.commit();

        assert_eq!(runtime.with(|state| state.value), Some(42));
        assert_eq!(info.arg, core::ptr::from_mut(&mut state).cast::<c_void>());
        assert!(info.stop_fun.is_some());
        runtime.clear();
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
        let runtime = crate::PackageStateStore::<u8>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut state = 42_u8;
        let mut start = super::PackageStart::from_raw(&mut info);

        {
            let _guard = start
                .install_runtime_state_guard_with_handler(super::stop_package, &mut state, &runtime)
                .expect("loader runtime state guard");
            assert_eq!(
                start.register_callbacks_with_bindings::<Config, Callback, 1, _>(&bindings),
                Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
            );
        }

        // C map: Refloat installs loader state before callback registration at
        // `third_party/refloat/src/main.c:2431-2456`. Rust registration can fail,
        // so the uncommitted guard must remove both state sources.
        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.handler_calls.get(), 0);
        assert!(!runtime.is_installed());
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());

        let bindings = FakeAppDataBindings::with_set_handler_result(false);
        let runtime = crate::PackageStateStore::<u8>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut state = 42_u8;
        let mut start = super::PackageStart::from_raw(&mut info);
        let _guard = start
            .install_runtime_state_guard_with_handler(super::stop_package, &mut state, &runtime)
            .expect("loader runtime state guard");

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
    fn package_start_allocates_loader_state_in_firmware_memory() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.allocate_state_with_handler(&allocator, super::stop_package, State { value: 99 }),
            Ok(())
        );

        assert_eq!(bindings.malloc_calls.get(), 1);
        assert_eq!(
            bindings.last_requested_len.get(),
            core::mem::size_of::<State>()
        );
        assert_eq!(bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert!(info.stop_fun.is_some());
        let loaded = unsafe { crate::loader_state_mut::<State>(&mut info) }.expect("loader state");
        assert_eq!(loaded.value, 99);
    }

    #[test]
    fn package_start_firmware_loader_state_guard_rolls_back_uncommitted_allocation() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        {
            let _guard = start
                .allocate_state_guard_with_handler(
                    &allocator,
                    super::stop_package,
                    State { value: 99 },
                )
                .expect("loader state guard");
            assert_eq!(bindings.free_calls.get(), 0);
            assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
            assert!(info.stop_fun.is_some());
        }

        assert_eq!(bindings.free_calls.get(), 1);
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_firmware_loader_state_guard_keeps_committed_allocation() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        let guard = start
            .allocate_state_guard_with_handler(&allocator, super::stop_package, State { value: 99 })
            .expect("loader state guard");
        guard.commit();

        assert_eq!(bindings.free_calls.get(), 0);
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert!(info.stop_fun.is_some());
        let loaded = unsafe { crate::loader_state_mut::<State>(&mut info) }.expect("loader state");
        assert_eq!(loaded.value, 99);
    }

    #[test]
    fn package_start_firmware_runtime_state_guard_clears_uncommitted_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        {
            let mut guard = start
                .allocate_runtime_state_guard_with_handler(
                    &allocator,
                    super::stop_package,
                    State { value: 99 },
                    &runtime,
                )
                .expect("loader runtime state guard");
            guard.state_mut().expect("state").value = 100;
            assert_eq!(runtime.with(|state| state.value), Some(100));
            assert_eq!(bindings.free_calls.get(), 0);
            assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
            assert!(info.stop_fun.is_some());
        }

        assert_eq!(bindings.free_calls.get(), 1);
        assert!(!runtime.is_installed());
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn package_start_firmware_runtime_state_installs_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.allocate_runtime_state_with_handler(
                &allocator,
                super::stop_package,
                State { value: 99 },
                &runtime,
            ),
            Ok(())
        );

        assert_eq!(bindings.free_calls.get(), 0);
        assert_eq!(runtime.with(|state| state.value), Some(99));
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert!(info.stop_fun.is_some());
        runtime.clear();
    }

    #[test]
    fn package_start_firmware_runtime_state_guard_keeps_committed_state() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let runtime = crate::PackageStateStore::<State>::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut backing = MaybeUninit::<State>::uninit();
        let bindings = TestAllocBindings::new(backing.as_mut_ptr().cast());
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        let guard = start
            .allocate_runtime_state_guard_with_handler(
                &allocator,
                super::stop_package,
                State { value: 99 },
                &runtime,
            )
            .expect("loader runtime state guard");
        guard.commit();

        assert_eq!(bindings.free_calls.get(), 0);
        assert_eq!(runtime.with(|state| state.value), Some(99));
        assert_eq!(info.arg, backing.as_mut_ptr().cast::<c_void>());
        assert!(info.stop_fun.is_some());
        let loaded = unsafe { crate::loader_state_mut::<State>(&mut info) }.expect("loader state");
        assert_eq!(loaded.value, 99);
        runtime.clear();
    }

    #[test]
    fn package_start_allocation_failure_clears_loader_metadata() {
        #[derive(Debug, PartialEq)]
        struct State {
            value: u32,
        }

        let mut info = ffi::LibInfo {
            stop_fun: Some(super::stop_package),
            arg: 0x1234_usize as *mut c_void,
            base_addr: 0,
        };
        let bindings = TestAllocBindings::failing();
        let allocator = crate::alloc::FirmwareAllocator::new(&bindings);
        let mut start = super::PackageStart::from_raw(&mut info);

        assert_eq!(
            start.allocate_state_with_handler(&allocator, super::stop_package, State { value: 7 }),
            Err(super::PackageStartError::AllocationFailed)
        );

        assert_eq!(bindings.malloc_calls.get(), 1);
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
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
