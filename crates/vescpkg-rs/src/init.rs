//! Native VESC package loader helpers shared across package payloads.

use crate::ffi;
use crate::runtime::CallbackRecorder;
use core::any::TypeId;

const MAX_CUSTOM_CONFIG_LEN: usize = 510;

#[cfg(test)]
fn state_allocation_size<T>() -> Option<usize> {
    core::mem::size_of::<T>()
        .max(1)
        .checked_add(core::mem::align_of::<T>() - 1)
}

#[cfg(all(not(test), target_arch = "arm"))]
fn state_allocation_size<T>() -> Option<usize> {
    crate::runtime::firmware_runtime_allocation_size::<T>()
}

#[cfg(test)]
fn align_state_pointer<T>(
    allocation: core::ptr::NonNull<core::ffi::c_void>,
) -> core::ptr::NonNull<T> {
    let align = core::mem::align_of::<T>();
    let aligned = allocation
        .as_ptr()
        .map_addr(|address| (address + align - 1) & !(align - 1))
        .cast::<T>();
    unsafe { core::ptr::NonNull::new_unchecked(aligned) }
}

#[cfg(all(not(test), target_arch = "arm"))]
fn align_state_pointer<T>(
    allocation: core::ptr::NonNull<core::ffi::c_void>,
) -> core::ptr::NonNull<T> {
    unsafe { crate::runtime::firmware_runtime_state_pointer(allocation) }
}

#[cfg(any(test, feature = "test-support", target_arch = "arm"))]
unsafe extern "C" fn stop_owned_package_state<T: crate::PackageRuntimeState>(
    arg: *mut core::ffi::c_void,
) {
    let Some(mut state) = core::ptr::NonNull::new(arg.cast::<T>()) else {
        return;
    };
    let runtime = T::runtime_store();
    if !runtime.begin_stop(state) {
        return;
    }
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let firmware = crate::Firmware::new();
        runtime
            .take_callbacks(state)
            .clear_registered(&crate::bindings::RealBindings);
        if let Some(threads) = runtime.take_threads(state) {
            threads.terminate_reverse(firmware.threads());
        }
    }
    runtime.finish_stop(state);
    unsafe { state.as_mut() }.stop();
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        unsafe { state.as_ptr().drop_in_place() };
        // VESC cannot unregister LispBM extensions or quiesce callbacks that
        // already loaded this ARG. Keep the allocation as a STOPPED admission
        // tombstone; late callbacks inspect it without touching dropped `T`.
    }
    #[cfg(not(target_arch = "arm"))]
    {
        drop(unsafe { crate::rust_alloc::boxed::Box::from_raw(state.as_ptr()) });
    }
}

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
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
    if info.stop_fun.is_some() {
        return Err(PackageStartError::StateAlreadyInstalled);
    }
    info.stop_fun = Some(crate::firmware::stop_handler_for_loader(info, stop_package));
    Ok(())
}

/// Failure while preparing package startup against firmware loader metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStartError {
    /// The firmware did not provide loader metadata for this package.
    LoaderUnavailable,
    /// Firmware could not allocate the requested package state.
    AllocationFailed,
    /// Package startup already installed runtime state.
    StateAlreadyInstalled,
    /// A state-backed operation named a different runtime state type.
    StateTypeMismatch,
    /// Firmware could not start the complete package thread pair.
    ThreadSpawnFailed,
    /// Package startup already installed its firmware thread pair.
    ThreadsAlreadyInstalled,
}

/// Safe startup context for package authors.
///
/// VESC can load up to ten native libraries in one Lisp package, but its IMU,
/// app-data, and custom-config callbacks are package-global singleton slots.
/// One coordinator library must own those registrations for the package;
/// secondary libraries must communicate through that owner instead of replacing
/// the global handlers.
pub struct PackageStart<'info> {
    info: *mut crate::LoaderInfo,
    state_type: Option<TypeId>,
    callback_recorder: Option<CallbackRecorder>,
    extension_image_pinned: bool,
    _info: core::marker::PhantomData<&'info mut crate::LoaderInfo>,
}

/// Package-local app-data callback resolved into its loaded firmware image.
///
/// Construct this through [`PackageStart::app_data_callback`], then register it
/// after any callbacks that firmware requires to be installed first.
pub struct LoadedAppDataCallback {
    handler: ffi::AppDataHandler,
    recorder: CallbackRecorder,
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

    #[inline(always)]
    fn register_with_bindings<B: crate::bindings::AppDataBindings>(
        self,
        bindings: &B,
    ) -> Result<(), crate::AppDataHandlerRegistrationError> {
        let registered = unsafe { bindings.set_app_data_handler(self.handler) };
        if registered && self.recorder.record_app_data() {
            return Ok(());
        }
        if registered {
            let _ = unsafe { bindings.clear_app_data_handler() };
        }
        Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
    }
}

impl<'info> PackageStart<'info> {
    /// Build a startup context tied to borrowed loader metadata.
    #[doc(hidden)]
    pub fn from_info(info: &'info mut crate::LoaderInfo) -> Self {
        Self {
            info,
            state_type: None,
            callback_recorder: None,
            extension_image_pinned: false,
            _info: core::marker::PhantomData,
        }
    }

    /// Build a startup context from the firmware ABI pointer.
    pub(crate) unsafe fn from_raw(info: *mut crate::LoaderInfo) -> Self {
        Self {
            info,
            state_type: None,
            callback_recorder: None,
            extension_image_pinned: false,
            _info: core::marker::PhantomData,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_lib_info(info: &'info mut ffi::LibInfo) -> Self {
        Self {
            info: core::ptr::from_mut(info).cast(),
            state_type: None,
            callback_recorder: None,
            extension_image_pinned: false,
            _info: core::marker::PhantomData,
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
        install_stop_hook(self.info.cast())?;
        Ok(())
    }

    /// Allocate loader-owned package state and publish it for callbacks.
    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    pub fn install_runtime_state<T: crate::PackageRuntimeState>(
        &mut self,
        state_value: T,
    ) -> Result<(), PackageStartError> {
        self.install_runtime_state_with(state_value, |state, allocation| unsafe {
            T::runtime_store().install_owned(state, allocation)
        })
    }

    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    fn install_runtime_state_with<T: crate::PackageRuntimeState>(
        &mut self,
        state_value: T,
        install: impl FnOnce(
            &mut T,
            core::ptr::NonNull<core::ffi::c_void>,
        ) -> Result<(), crate::runtime::PackageStateInstallError>,
    ) -> Result<(), PackageStartError> {
        let info = self
            .raw_info_mut()
            .ok_or(PackageStartError::LoaderUnavailable)?;
        // VESC marks loader slots free by clearing `stop_fun`; it intentionally
        // leaves the previous package's `arg` behind when reusing the slot.
        if info.stop_fun.is_some() {
            return Err(PackageStartError::StateAlreadyInstalled);
        }

        #[cfg(target_arch = "arm")]
        let (mut state_ptr, allocation) = {
            let bytes = state_allocation_size::<T>().ok_or(PackageStartError::AllocationFailed)?;
            let allocation = core::ptr::NonNull::new(unsafe { ffi::vesc_malloc(bytes) })
                .ok_or(PackageStartError::AllocationFailed)?;
            let state = align_state_pointer::<T>(allocation);
            unsafe { state.as_ptr().write(state_value) };
            (state, allocation)
        };
        #[cfg(target_arch = "arm")]
        let state = unsafe { state_ptr.as_mut() };

        #[cfg(not(target_arch = "arm"))]
        let mut owned_state = crate::rust_alloc::boxed::Box::new(state_value);
        #[cfg(not(target_arch = "arm"))]
        let state = owned_state.as_mut();
        #[cfg(not(target_arch = "arm"))]
        let allocation = core::ptr::NonNull::from(&mut *state).cast();

        let state_ptr = core::ptr::from_mut(state);
        if let Err(error) = install(state, allocation) {
            #[cfg(target_arch = "arm")]
            {
                unsafe { state_ptr.drop_in_place() };
                unsafe { ffi::vesc_free(allocation.as_ptr()) };
            }
            return Err(match error {
                crate::runtime::PackageStateInstallError::AlreadyInstalled => {
                    PackageStartError::StateAlreadyInstalled
                }
            });
        }
        info.arg = state_ptr.cast();
        info.stop_fun = Some(crate::firmware::stop_handler_for_loader(
            info,
            stop_owned_package_state::<T>,
        ));
        let callback_recorder = CallbackRecorder::new(core::ptr::NonNull::from(&mut *state));
        #[cfg(not(target_arch = "arm"))]
        let _ = crate::rust_alloc::boxed::Box::into_raw(owned_state);
        self.state_type = Some(TypeId::of::<T>());
        self.callback_recorder = Some(callback_recorder);
        Ok(())
    }

    /// Commit a successful package start, or stop and roll back a failed start.
    ///
    /// An image that has already published a LispBM extension cannot be rolled
    /// back because VESC does not expose extension removal through `VESC_IF`.
    /// Such an image remains loaded until the package-wide Lisp restart.
    ///
    /// C map: VESC uses the init result as the Lisp load result, but selects a
    /// reusable loader slot by `stop_fun == NULL` and only clears that field on
    /// unload/stop (`lispBM/lispif_c_lib.c:1087-1155`). Refloat returns false
    /// for allocation and thread-spawn failures (`src/main.c:2664-2702`), so a
    /// failed Rust start must both run its stop hook and release the loader slot.
    #[doc(hidden)]
    pub fn finish_start(mut self, started: bool) -> bool {
        // VESC_IF can add LispBM extensions but cannot remove them. Once a
        // handler is published, keep its native image and runtime alive until
        // the package-wide Lisp restart resets the extension table.
        let started = started || self.extension_image_pinned;
        let running = self
            .callback_recorder
            .map_or(started, |recorder| recorder.finish_start(started));
        if !running && let Some(info) = self.raw_info_mut() {
            let arg = info.arg;
            if let Some(stop) = info.stop_fun.take() {
                unsafe { stop(arg) };
            }
        }
        running
    }

    fn state_type_matches<T: 'static>(&self) -> bool {
        self.state_type == Some(TypeId::of::<T>())
    }

    /// Run startup work with loader-owned package state.
    pub fn with_runtime_state<T: crate::PackageRuntimeState, R>(
        &mut self,
        operation: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        if !self.state_type_matches::<T>() {
            return None;
        }
        let state = self
            .raw_info_mut()
            .and_then(|info| core::ptr::NonNull::new(info.arg.cast::<T>()))?;
        T::runtime_store().with_expected_mut(crate::runtime::ExpectedState::Exact(state), operation)
    }

    /// Start and retain a complete package-owned firmware thread pair.
    #[cfg(not(test))]
    pub fn spawn_thread_pair<T: crate::PackageRuntimeState>(
        &mut self,
        pair: crate::ThreadPairSpec<T>,
    ) -> Result<(), PackageStartError> {
        self.spawn_thread_pair_with_bindings(pair, crate::thread::RealThreadBindings)
    }

    pub(crate) fn spawn_thread_pair_with_bindings<T, B>(
        &mut self,
        pair: crate::ThreadPairSpec<T>,
        bindings: B,
    ) -> Result<(), PackageStartError>
    where
        T: crate::PackageRuntimeState,
        B: crate::thread::ThreadBindings,
    {
        if !self.state_type_matches::<T>() {
            return Err(PackageStartError::StateTypeMismatch);
        }
        let state = self
            .raw_info_mut()
            .and_then(|info| core::ptr::NonNull::new(info.arg.cast::<T>()))
            .ok_or(PackageStartError::LoaderUnavailable)?;
        T::runtime_store()
            .with_expected(crate::runtime::ExpectedState::Exact(state), |_| ())
            .ok_or(PackageStartError::LoaderUnavailable)?;
        let threads = crate::thread::ThreadApi::new(&bindings)
            .spawn_thread_pair(pair, state)
            .ok_or(PackageStartError::ThreadSpawnFailed)?;
        T::runtime_store()
            .install_threads(state, threads)
            .map_err(|threads| {
                threads.terminate_reverse(&crate::thread::ThreadApi::new(bindings));
                PackageStartError::ThreadsAlreadyInstalled
            })
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
    /// stores those resolved addresses at `third_party/vesc/conf_custom.c:34-42`.
    pub(crate) fn register_stateful_custom_config_with_bindings<B, T, const LEN: usize>(
        &mut self,
        bindings: &B,
    ) -> bool
    where
        B: crate::bindings::CustomConfigBindings,
        T: crate::__macro_support::PackageCustomConfigCallback<LEN>,
    {
        let Some(image) = self.native_image().filter(|_| LEN <= MAX_CUSTOM_CONFIG_LEN) else {
            return false;
        };
        if !self.state_type_matches::<T::State>() {
            return false;
        }
        let Some(recorder) = self.callback_recorder else {
            return false;
        };
        let (get, set, xml) = T::image_addresses();
        let callbacks = unsafe {
            (
                core::mem::transmute::<usize, ffi::CustomConfigGet>(image.resolve_addr(get)),
                core::mem::transmute::<usize, ffi::CustomConfigSet>(image.resolve_addr(set)),
                core::mem::transmute::<usize, ffi::CustomConfigXml>(image.resolve_addr(xml)),
            )
        };
        let registered =
            bindings.register_custom_config_callbacks(callbacks.0, callbacks.1, callbacks.2);
        if registered && recorder.record_custom_config() {
            true
        } else {
            if registered {
                let _ = unsafe { bindings.clear_custom_configs() };
            }
            false
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
    pub fn app_data_callback<T: crate::__macro_support::PackageAppDataCallback>(
        &mut self,
    ) -> Option<LoadedAppDataCallback> {
        let state_type = T::state_type()?;
        if self.state_type != Some(state_type) {
            return None;
        }
        let recorder = self.callback_recorder?;
        let image = self.native_image()?;
        let address = image.resolve_addr(T::image_address());
        let handler = unsafe { core::mem::transmute::<usize, ffi::AppDataHandler>(address) };
        Some(LoadedAppDataCallback { handler, recorder })
    }

    /// Register a concrete package-local typed IMU callback.
    ///
    /// This claims the package-global IMU callback slot. Call it only from the
    /// package's designated coordinator native library.
    ///
    /// C map: Refloat registers `imu_ref_callback` at
    /// `third_party/refloat/src/main.c:2454`; VESC stores and invokes it through
    /// `third_party/vesc/imu/imu.c:581-582` and `:704-727`.
    #[cfg(not(test))]
    #[inline(always)]
    pub fn register_imu_read_callback<T: crate::__macro_support::PackageImuReadCallback>(
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
        T: crate::__macro_support::PackageImuReadCallback,
        B: crate::bindings::ImuReadCallbackBindings,
    {
        let Some(image) = self.native_image() else {
            return Err(PackageStartError::LoaderUnavailable);
        };
        if !self.state_type_matches::<T::State>() {
            return Err(PackageStartError::StateTypeMismatch);
        }
        let recorder = self
            .callback_recorder
            .ok_or(PackageStartError::StateTypeMismatch)?;
        let address = image.resolve_addr(T::image_address());
        let callback = unsafe { core::mem::transmute::<usize, ffi::ImuReadCallback>(address) };
        bindings.set_imu_read_callback_handler(callback);
        if recorder.record_imu() {
            Ok(())
        } else {
            unsafe { bindings.clear_imu_read_callback() };
            Err(PackageStartError::StateTypeMismatch)
        }
    }

    /// Register this package's typed custom config and app-data callback.
    ///
    /// This claims package-global callback slots. Call it only from the
    /// package's designated coordinator native library.
    ///
    /// The app-data callback is resolved before either firmware call, while
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
        C: crate::__macro_support::PackageCustomConfigCallback<LEN>,
        A: crate::__macro_support::PackageAppDataCallback,
    {
        self.register_callbacks_with_bindings::<C, A, LEN, _>(&crate::bindings::RealBindings)
    }

    #[inline(always)]
    fn register_callbacks_with_bindings<C, A, const LEN: usize, B>(
        &mut self,
        bindings: &B,
    ) -> Result<(), crate::AppDataHandlerRegistrationError>
    where
        C: crate::__macro_support::PackageCustomConfigCallback<LEN>,
        A: crate::__macro_support::PackageAppDataCallback,
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
            if let Some(recorder) = self.callback_recorder {
                let _ = recorder.clear_custom_config();
            }
            return Err(error);
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-support", target_arch = "arm"))]
    /// Register extension descriptors using loader metadata for this package image.
    fn register_extensions_with_lifecycle<B, const N: usize>(
        &mut self,
        lifecycle: &crate::lifecycle_core::PackageLifecycle<B>,
        descriptors: [crate::ExtensionDescriptor; N],
    ) -> Result<crate::ExtensionRegistration, crate::RegisterError>
    where
        B: crate::bindings::LbmBindings,
    {
        descriptors.iter().copied().try_for_each(|descriptor| {
            descriptor
                .validate()
                .map(|_| ())
                .map_err(|_| crate::RegisterError::InvalidExtensionName)
        })?;
        if descriptors.iter().copied().any(|descriptor| {
            descriptor
                .state_type()
                .is_some_and(|state_type| self.state_type != Some(state_type))
        }) {
            return Err(crate::RegisterError::StateTypeMismatch);
        }
        let info = self
            .raw_info_mut()
            .ok_or(crate::RegisterError::LoaderUnavailable)?;
        if info.stop_fun.is_none() {
            return Err(crate::RegisterError::PackageNotRetained);
        }
        // SAFETY: `info` is the loader metadata just installed for this package
        // image, so extension descriptor pointers are resolved against the image
        // that owns them.
        let registration = unsafe {
            lifecycle.register_extensions_from_image(ffi::NativeImage::from_info(info), descriptors)
        };
        self.extension_image_pinned |= registration.registered() != 0;
        Ok(registration)
    }

    /// Register extension descriptors through the test-support lifecycle seam.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn register_extensions_with<B, const N: usize>(
        &mut self,
        lifecycle: &crate::lifecycle_core::PackageLifecycle<B>,
        descriptors: [crate::ExtensionDescriptor; N],
    ) -> Result<crate::ExtensionRegistration, crate::RegisterError>
    where
        B: crate::bindings::LbmBindings,
    {
        self.register_extensions_with_lifecycle(lifecycle, descriptors)
    }

    /// Register extension descriptors with the live firmware bindings.
    ///
    /// Firmware may accept only part of the table. The returned report exposes
    /// that outcome, and any accepted handler pins this native image until the
    /// package-wide Lisp restart.
    #[cfg(all(not(test), target_arch = "arm"))]
    pub fn register_extensions<const N: usize>(
        &mut self,
        descriptors: [crate::ExtensionDescriptor; N],
    ) -> Result<crate::ExtensionRegistration, crate::RegisterError> {
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(crate::bindings::RealBindings);
        self.register_extensions_with_lifecycle(&lifecycle, descriptors)
    }
}

/// Construct the startup context for the exported package entry trampoline.
#[doc(hidden)]
///
/// # Safety
///
/// `info` must be null or point to loader metadata that remains valid for package startup.
pub unsafe fn __package_start_from_raw<'info>(info: *mut crate::LoaderInfo) -> PackageStart<'info> {
    unsafe { PackageStart::from_raw(info) }
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

        /// Return this package's loaded image base for stateful callbacks.
        #[cfg(all(not(test), target_arch = "arm"))]
        #[doc(hidden)]
        #[inline(never)]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".program_address")]
        pub extern "C" fn __vescpkg_program_address() -> u32 {
            let address: u32;
            // SAFETY: `prog_ptr` is the first word in the package image and
            // this helper is linked into the adjacent program-address section, within the
            // range of Thumb's PC-relative `adr` instruction.
            unsafe {
                core::arch::asm!(
                    "adr {address}, prog_ptr",
                    address = out(reg) address,
                    options(readonly, nostack, preserves_flags),
                );
            }
            address
        }

        /// Firmware loader entrypoint that runs the package start function.
        #[cfg(any(test, all(not(test), target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        extern "C" fn package_lib_init(info: *mut $crate::LoaderInfo) -> bool {
            let mut start = unsafe { $crate::__macro_support::__package_start_from_raw(info) };
            let started = $start(&mut start);
            start.finish_start(started)
        }

        /// Host-linking loader shim for package crates.
        #[cfg(all(not(test), not(target_arch = "arm")))]
        #[inline(never)]
        #[unsafe(no_mangle)]
        extern "C" fn package_lib_init(info: *mut $crate::LoaderInfo) -> bool {
            let mut start = unsafe { $crate::__macro_support::__package_start_from_raw(info) };
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
        PackageStartError, align_state_pointer, init_for_tests, install_stop_hook,
        reset_init_call_count_for_tests, state_allocation_size,
    };
    use crate::ffi;
    use crate::test_support::FakeAppDataBindings;
    use core::sync::atomic::{AtomicUsize, Ordering};

    struct TestPackageImuRead;
    struct WrongPackageImuRead;

    static TEST_IMU_STATE: crate::PackageStateStore<TestImuState> = crate::PackageStateStore::new();
    static OWNED_STATE: crate::PackageStateStore<OwnedState> = crate::PackageStateStore::new();
    static OWNED_STATE_STOPS: AtomicUsize = AtomicUsize::new(0);
    static OWNED_STATE_DROPS: AtomicUsize = AtomicUsize::new(0);
    static FAILED_STATE: crate::PackageStateStore<FailedState> = crate::PackageStateStore::new();
    static FAILED_STATE_DROPS: AtomicUsize = AtomicUsize::new(0);
    static EXTENSION_REGISTRATION_STATE: crate::PackageStateStore<ExtensionRegistrationState> =
        crate::PackageStateStore::new();
    static REGISTRATION_STATE: crate::PackageStateStore<RegistrationState> =
        crate::PackageStateStore::new();
    static RELOAD_STATE: crate::PackageStateStore<ReloadState> = crate::PackageStateStore::new();
    static SPAWN_STATE: crate::PackageStateStore<SpawnState> = crate::PackageStateStore::new();

    struct OwnedState(u32);
    struct FailedState;
    struct ExtensionRegistrationState;
    struct RegistrationState;
    struct ReloadState;
    struct SpawnState;
    struct TestImuState;
    struct WrongImuState;

    mod failing_entrypoint {
        fn start(_: &mut crate::PackageStart<'_>) -> bool {
            false
        }

        crate::package_start!(start);

        pub(super) fn run(info: &mut crate::LoaderInfo) -> bool {
            package_lib_init(info)
        }
    }

    impl crate::PackageRuntimeState for ExtensionRegistrationState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &EXTENSION_REGISTRATION_STATE
        }
    }

    impl crate::PackageRuntimeState for TestImuState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &TEST_IMU_STATE
        }
    }

    impl crate::PackageRuntimeState for WrongImuState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            unreachable!("registration rejects the mismatched type before callback dispatch")
        }
    }

    impl crate::PackageRuntimeState for ReloadState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &RELOAD_STATE
        }
    }

    impl crate::PackageRuntimeState for RegistrationState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &REGISTRATION_STATE
        }
    }

    impl crate::PackageRuntimeState for SpawnState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &SPAWN_STATE
        }
    }

    #[repr(align(64))]
    struct AlignedState([u8; 64]);

    #[test]
    fn state_allocation_preserves_alignment_and_supports_zero_sized_state() {
        assert_eq!(AlignedState([0; 64]).0.len(), 64);
        assert_eq!(state_allocation_size::<AlignedState>(), Some(127));
        assert_eq!(state_allocation_size::<()>(), Some(1));

        let allocation = core::ptr::NonNull::new(65_usize as *mut core::ffi::c_void).unwrap();
        assert_eq!(
            align_state_pointer::<AlignedState>(allocation).as_ptr() as usize,
            128
        );
    }

    impl crate::PackageRuntimeState for OwnedState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &OWNED_STATE
        }

        fn stop(&mut self) {
            OWNED_STATE_STOPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl crate::PackageRuntimeState for FailedState {
        fn runtime_store() -> &'static crate::PackageStateStore<Self> {
            &FAILED_STATE
        }
    }

    impl Drop for FailedState {
        fn drop(&mut self) {
            FAILED_STATE_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Drop for OwnedState {
        fn drop(&mut self) {
            OWNED_STATE_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl crate::ImuReadHandler for TestPackageImuRead {
        type State = TestImuState;

        fn read(_state: &mut Self::State, _sample: crate::ImuReadSample) {}
    }

    unsafe impl crate::__macro_support::PackageImuReadCallback for TestPackageImuRead {
        fn image_address() -> usize {
            crate::test_support::stubs::imu_read_callback as *const () as usize
        }
    }

    impl crate::ImuReadHandler for WrongPackageImuRead {
        type State = WrongImuState;

        fn read(_state: &mut Self::State, _sample: crate::ImuReadSample) {}
    }

    unsafe impl crate::__macro_support::PackageImuReadCallback for WrongPackageImuRead {
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
    fn package_entrypoint_propagates_start_failure() {
        let mut info = crate::LoaderInfo::new();

        assert!(!failing_entrypoint::run(&mut info));
    }

    #[test]
    fn install_stop_hook_rejects_null_loader_metadata() {
        assert_eq!(
            install_stop_hook(core::ptr::null_mut()),
            Err(PackageStartError::LoaderUnavailable)
        );
    }

    #[test]
    fn package_start_owns_runtime_state_until_stop() {
        OWNED_STATE_STOPS.store(0, Ordering::Relaxed);
        OWNED_STATE_DROPS.store(0, Ordering::Relaxed);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

        assert_eq!(start.install_runtime_state(OwnedState(37)), Ok(()));
        assert_eq!(
            start.install_runtime_state(OwnedState(99)),
            Err(super::PackageStartError::StateAlreadyInstalled)
        );
        assert_eq!(
            start.with_runtime_state::<OwnedState, _>(|state| state.0),
            Some(37)
        );
        assert_eq!(OWNED_STATE.with(|state| state.0), Some(37));

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("owned state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };
        unsafe { stop(arg) };

        assert_eq!(OWNED_STATE_STOPS.load(Ordering::Relaxed), 1);
        assert_eq!(OWNED_STATE_DROPS.load(Ordering::Relaxed), 2);
        assert_eq!(OWNED_STATE.with(|state| state.0), None);
    }

    #[test]
    fn failed_package_start_rolls_back_owned_runtime_state() {
        OWNED_STATE_STOPS.store(0, Ordering::Relaxed);
        OWNED_STATE_DROPS.store(0, Ordering::Relaxed);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

        start.install_runtime_state(OwnedState(37)).unwrap();
        assert!(!start.finish_start(false));

        assert_eq!(OWNED_STATE_STOPS.load(Ordering::Relaxed), 1);
        assert_eq!(OWNED_STATE_DROPS.load(Ordering::Relaxed), 1);
        assert_eq!(OWNED_STATE.with(|state| state.0), None);
        assert!(info.stop_fun.is_none());
    }

    #[test]
    fn runtime_install_failure_drops_state_before_publishing_loader_metadata() {
        FAILED_STATE_DROPS.store(0, Ordering::Relaxed);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

        let result = start.install_runtime_state_with(FailedState, |_, _| {
            Err(crate::runtime::PackageStateInstallError::AlreadyInstalled)
        });

        assert_eq!(result, Err(PackageStartError::StateAlreadyInstalled));
        assert_eq!(FAILED_STATE_DROPS.load(Ordering::Relaxed), 1);
        assert!(info.arg.is_null());
        assert!(info.stop_fun.is_none());
        assert!(!FAILED_STATE.is_installed());
    }

    #[test]
    fn package_start_reuses_a_loader_slot_with_a_stopped_tombstone() {
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_runtime_state(ReloadState).unwrap();
        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("first stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };

        info.stop_fun = None;
        assert_eq!(info.arg, arg);
        let mut reloaded = super::PackageStart::from_lib_info(&mut info);
        assert_eq!(reloaded.install_runtime_state(ReloadState), Ok(()));
        assert!(!reloaded.raw_info_mut().unwrap().arg.is_null());
        assert!(reloaded.finish_start(true));
    }

    #[test]
    fn package_start_spawns_threads_with_the_loader_state_identity() {
        unsafe extern "C" fn thread_entry(_arg: *mut core::ffi::c_void) {}

        let bindings = crate::thread::test_support::FakeThreadBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };
        let (stop, arg) = {
            let mut start = super::PackageStart::from_lib_info(&mut info);
            start.install_runtime_state(SpawnState).unwrap();
            let state_arg = start.raw_info_mut().unwrap().arg as usize;
            let pair = crate::ThreadPairSpec::new(
                crate::ThreadSpec::<SpawnState>::from_entry(
                    thread_entry,
                    crate::ThreadStackSize::from_bytes(1_536),
                    crate::thread_name!("main"),
                ),
                crate::ThreadSpec::<()>::from_entry(
                    thread_entry,
                    crate::ThreadStackSize::from_bytes(1_024),
                    crate::thread_name!("aux"),
                ),
            );

            assert_eq!(
                start.spawn_thread_pair_with_bindings(pair, &bindings),
                Ok(())
            );
            assert_eq!(bindings.spawn_args.get(), [state_arg, 0]);
            let stop = (
                start.raw_info_mut().unwrap().stop_fun.unwrap(),
                start.raw_info_mut().unwrap().arg,
            );
            assert!(start.finish_start(true));
            stop
        };
        unsafe { stop(arg) };
    }

    #[test]
    fn fresh_package_start_rejects_a_poisoned_loader_arg_before_state_access() {
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::without_provenance_mut(1),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

        assert_eq!(
            start.with_runtime_state::<OwnedState, _>(|state| state.0),
            None
        );
    }

    #[test]
    fn fresh_package_start_rejects_thread_spawn_before_reading_a_poisoned_arg() {
        unsafe extern "C" fn thread_entry(_arg: *mut core::ffi::c_void) {}

        let bindings = crate::thread::test_support::FakeThreadBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::without_provenance_mut(1),
            base_addr: 0,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);
        let pair = crate::ThreadPairSpec::new(
            crate::ThreadSpec::<SpawnState>::from_entry(
                thread_entry,
                crate::ThreadStackSize::from_bytes(1_536),
                crate::thread_name!("main"),
            ),
            crate::ThreadSpec::<()>::from_entry(
                thread_entry,
                crate::ThreadStackSize::from_bytes(1_024),
                crate::thread_name!("aux"),
            ),
        );

        assert_eq!(
            start.spawn_thread_pair_with_bindings(pair, &bindings),
            Err(PackageStartError::StateTypeMismatch)
        );
        assert_eq!(bindings.spawn_calls.get(), 0);
    }

    #[test]
    fn package_start_exposes_loader_native_image_identity() {
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

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
    fn package_start_rejects_stateless_app_data_callback_registration() {
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        struct Callback;

        unsafe impl crate::__macro_support::PackageAppDataCallback for Callback {
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
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_stop_hook().unwrap();

        assert!(start.app_data_callback::<Callback>().is_none());
        assert_eq!(bindings.handler_calls.get(), 0);
        assert_eq!(
            start.install_stop_hook(),
            Err(PackageStartError::StateAlreadyInstalled)
        );
    }

    #[test]
    fn package_start_stops_callback_registration_when_custom_config_is_rejected() {
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        struct Callback;
        struct Config;
        struct ConfigState;

        static CONFIG_STATE: crate::PackageStateStore<ConfigState> =
            crate::PackageStateStore::new();

        impl crate::PackageRuntimeState for ConfigState {
            fn runtime_store() -> &'static crate::PackageStateStore<Self> {
                &CONFIG_STATE
            }
        }

        unsafe impl crate::__macro_support::PackageAppDataCallback for Callback {
            fn image_address() -> usize {
                handler as *const () as usize
            }

            fn state_type() -> Option<core::any::TypeId> {
                Some(core::any::TypeId::of::<ConfigState>())
            }
        }

        impl crate::StatefulCustomConfigCallback<1> for Config {
            type State = ConfigState;

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
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_runtime_state(ConfigState).unwrap();

        assert_eq!(
            start.register_callbacks_with_bindings::<Config, Callback, 1, _>(&bindings),
            Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
        );
        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.handler_calls.get(), 0);

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("first config state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };

        let bindings = FakeAppDataBindings::with_set_handler_result(false);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_runtime_state(ConfigState).unwrap();

        assert_eq!(
            start.register_callbacks_with_bindings::<Config, Callback, 1, _>(&bindings),
            Err(crate::AppDataHandlerRegistrationError::FirmwareRejected)
        );
        assert_eq!(bindings.custom_config_register_calls.get(), 1);
        assert_eq!(bindings.handler_calls.get(), 1);
        assert_eq!(bindings.custom_config_clear_calls.get(), 1);

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("second config state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };
    }

    #[test]
    fn package_start_rejects_custom_config_larger_than_the_firmware_buffer() {
        struct Config;
        struct ConfigState;

        static CONFIG_STATE: crate::PackageStateStore<ConfigState> =
            crate::PackageStateStore::new();
        static CONFIG: [u8; 511] = [0; 511];

        impl crate::PackageRuntimeState for ConfigState {
            fn runtime_store() -> &'static crate::PackageStateStore<Self> {
                &CONFIG_STATE
            }
        }

        impl crate::StatefulCustomConfigCallback<511> for Config {
            type State = ConfigState;

            fn default_config() -> crate::ConfigBytes<'static> {
                crate::ConfigBytes::new(&CONFIG)
            }

            fn current_config(_state: &Self::State) -> Option<crate::ConfigBytes<'_>> {
                Some(crate::ConfigBytes::new(&CONFIG))
            }

            fn set_config(_state: &mut Self::State, _config: crate::ConfigBytes<'_>) -> bool {
                true
            }

            fn config_xml() -> crate::ConfigXml<'static> {
                crate::ConfigXml::new(b"<Config/>")
            }
        }
        crate::firmware_stateful_custom_config_callbacks!(
            oversized_config_get,
            oversized_config_set,
            oversized_config_xml,
            Config,
            511
        );

        let bindings = FakeAppDataBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);

        assert!(!start.register_stateful_custom_config_with_bindings::<_, Config, 511>(&bindings));
        assert_eq!(bindings.custom_config_register_calls.get(), 0);
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
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_runtime_state(TestImuState).unwrap();

        assert_eq!(
            start.register_imu_read_callback_with_bindings::<TestPackageImuRead, _>(&bindings),
            Ok(())
        );
        // C map: Refloat registers `imu_ref_callback` at
        // `third_party/refloat/src/main.c:2454`; VESC stores the resolved pointer
        // at `third_party/vesc/imu/imu.c:581-582`.
        assert_eq!(bindings.imu_read_callback_calls.get(), 1);
        assert_eq!(
            bindings.last_imu_read_callback.get(),
            image.resolve_addr(
                <TestPackageImuRead as crate::__macro_support::PackageImuReadCallback>::image_address()
            )
        );

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("IMU state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };
    }

    #[test]
    fn package_start_rejects_a_callback_for_a_different_runtime_state() {
        unsafe extern "C" fn handler(_data: *mut u8, _len: u32) {}

        struct WrongAppDataCallback;

        unsafe impl crate::__macro_support::PackageAppDataCallback for WrongAppDataCallback {
            fn image_address() -> usize {
                handler as *const () as usize
            }

            fn state_type() -> Option<core::any::TypeId> {
                Some(core::any::TypeId::of::<()>())
            }
        }

        let bindings = FakeAppDataBindings::new();
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x3000,
        };
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_runtime_state(RegistrationState).unwrap();

        assert_eq!(
            start.register_imu_read_callback_with_bindings::<WrongPackageImuRead, _>(&bindings),
            Err(super::PackageStartError::StateTypeMismatch)
        );
        assert!(start.app_data_callback::<WrongAppDataCallback>().is_none());

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("registration state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };
    }

    #[test]
    fn package_start_rejects_imu_callback_without_loader_metadata() {
        let bindings = FakeAppDataBindings::new();
        let mut start = unsafe { super::PackageStart::from_raw(core::ptr::null_mut()) };

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
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_stop_hook().unwrap();
        let descriptor = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-start-probe"),
            stubs::extension_handler,
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [descriptor]),
            Ok(crate::ExtensionRegistration::new(1, 1))
        );
        assert_eq!(bindings.add_calls.get(), 1);
        assert_eq!(
            bindings.last_name.get(),
            descriptor.name().as_cstr().as_ptr() as usize
        );
        assert_eq!(bindings.last_handler.get(), descriptor.handler() as usize);

        let mut rejected_info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let rejecting_bindings = FakeBindings::rejecting();
        let rejecting_lifecycle = crate::lifecycle_core::PackageLifecycle::new(&rejecting_bindings);
        let mut rejecting_start = super::PackageStart::from_lib_info(&mut rejected_info);
        rejecting_start.install_stop_hook().unwrap();

        assert_eq!(
            rejecting_start.register_extensions_with(&rejecting_lifecycle, [descriptor]),
            Ok(crate::ExtensionRegistration::new(1, 0))
        );
    }

    #[test]
    fn package_start_keeps_an_extension_image_loaded_after_later_failure() {
        use crate::test_support::{FakeBindings, stubs};

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let bindings = FakeBindings::with_add_results([true, false]);
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(&bindings);
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start.install_stop_hook().unwrap();
        let first = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-start-a"),
            stubs::extension_handler,
        );
        let second = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-start-b"),
            stubs::extension_handler,
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [first, second]),
            Ok(crate::ExtensionRegistration::new(2, 1))
        );
        assert!(start.finish_start(false));
    }

    #[test]
    fn package_start_rejects_extensions_before_the_image_is_retained() {
        use crate::test_support::{FakeBindings, stubs};

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let bindings = FakeBindings::new();
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(&bindings);
        let mut start = super::PackageStart::from_lib_info(&mut info);
        let descriptor = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-unretained"),
            stubs::extension_handler,
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [descriptor]),
            Err(crate::RegisterError::PackageNotRetained)
        );
        assert_eq!(bindings.add_calls.get(), 0);
    }

    #[test]
    fn package_start_preflights_all_extension_names_before_registration() {
        use crate::test_support::{FakeBindings, stubs};

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let bindings = FakeBindings::new();
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(&bindings);
        let mut start = super::PackageStart::from_lib_info(&mut info);
        let valid = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("ext-valid"),
            stubs::extension_handler,
        );
        let invalid = crate::ExtensionDescriptor::from_handler(
            crate::extension_name!("invalid"),
            stubs::extension_handler,
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [valid, invalid]),
            Err(crate::RegisterError::InvalidExtensionName)
        );
        assert_eq!(bindings.add_calls.get(), 0);
    }

    #[test]
    fn package_start_rejects_an_extension_for_a_different_runtime_state() {
        struct WrongState;
        struct WrongExtension;

        static WRONG_STATE: crate::PackageStateStore<WrongState> = crate::PackageStateStore::new();

        impl crate::PackageRuntimeState for WrongState {
            fn runtime_store() -> &'static crate::PackageStateStore<Self> {
                &WRONG_STATE
            }
        }

        impl crate::StatefulLbmExtension for WrongExtension {
            type State = WrongState;

            fn call(_state: &mut Self::State, _args: crate::LispArgs<'_>) -> crate::LispValue {
                unreachable!("registration rejects the mismatched extension")
            }
        }

        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };
        let bindings = crate::test_support::FakeBindings::new();
        let lifecycle = crate::lifecycle_core::PackageLifecycle::new(&bindings);
        let mut start = super::PackageStart::from_lib_info(&mut info);
        start
            .install_runtime_state(ExtensionRegistrationState)
            .unwrap();
        let descriptor = crate::ExtensionDescriptor::stateful::<WrongExtension>(
            crate::extension_name!("ext-wrong-state"),
        );

        assert_eq!(
            start.register_extensions_with(&lifecycle, [descriptor]),
            Err(crate::RegisterError::StateTypeMismatch)
        );
        assert_eq!(bindings.add_calls.get(), 0);

        let stop = start
            .raw_info_mut()
            .unwrap()
            .stop_fun
            .expect("extension state stop hook");
        let arg = start.raw_info_mut().unwrap().arg;
        assert!(start.finish_start(true));
        unsafe { stop(arg) };
    }
}
