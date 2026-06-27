use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicUsize, Ordering};

use vesc_protocol::ble_loopback::{
    LoopbackError, LoopbackPacket, BLE_LOOPBACK_PROTOCOL_VERSION, MAX_LOOPBACK_PAYLOAD_BYTES,
};
use vesc_protocol::WireCommand;
use vesc_protocol::WireVersion;

const MAX_FRAME_BYTES: usize = 19;
const MAX_LOGS: usize = 8;
const MAX_FRAMES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BleFrame {
    bytes: [u8; MAX_FRAME_BYTES],
    len: usize,
}

impl BleFrame {
    pub fn from_slice(bytes: &[u8]) -> Self {
        assert!(bytes.len() <= MAX_FRAME_BYTES, "BLE frame exceeds budget");

        let mut frame = [0_u8; MAX_FRAME_BYTES];
        let mut index = 0;
        while index < bytes.len() {
            frame[index] = bytes[index];
            index += 1;
        }

        Self {
            bytes: frame,
            len: bytes.len(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

pub trait DeviceServices {
    fn now_ms(&self) -> u64;
    fn log(&self, message: &'static str);
    fn init_ble_loopback(&self) -> Result<(), &'static str>;
    fn ble_connected(&self) -> bool;
    fn receive_ble_frame(&self) -> Option<BleFrame>;
    fn send_ble_frame(&self, frame: BleFrame);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackPackageState {
    Booting,
    WaitingForConnection,
    Ready,
    Failed(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackTick {
    Idle,
    WaitingForConnection,
    Replied(WireCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackStartError {
    InitFailed(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackRuntimeError {
    Start(LoopbackStartError),
    Frame(LoopbackError),
}

impl From<LoopbackStartError> for LoopbackRuntimeError {
    fn from(error: LoopbackStartError) -> Self {
        Self::Start(error)
    }
}

impl From<LoopbackError> for LoopbackRuntimeError {
    fn from(error: LoopbackError) -> Self {
        Self::Frame(error)
    }
}

#[derive(Debug)]
pub struct LoopbackPackageRuntime<S> {
    services: S,
    state: LoopbackPackageState,
}

impl<S: DeviceServices> LoopbackPackageRuntime<S> {
    pub fn new(services: S) -> Self {
        Self {
            services,
            state: LoopbackPackageState::Booting,
        }
    }

    pub fn state(&self) -> LoopbackPackageState {
        self.state
    }

    pub fn start(&mut self) -> Result<LoopbackPackageState, LoopbackStartError> {
        self.services.log("booting BLE loopback package");

        match self.services.init_ble_loopback() {
            Ok(()) if self.services.ble_connected() => {
                self.state = LoopbackPackageState::Ready;
                self.services.log("BLE loopback ready");
            }
            Ok(()) => {
                self.state = LoopbackPackageState::WaitingForConnection;
                self.services.log("waiting for BLE connection");
            }
            Err(reason) => {
                self.state = LoopbackPackageState::Failed(reason);
                self.services.log("BLE init failed");
                return Err(LoopbackStartError::InitFailed(reason));
            }
        }

        Ok(self.state)
    }

    pub fn tick(&mut self) -> Result<LoopbackTick, LoopbackRuntimeError> {
        match self.state {
            LoopbackPackageState::Booting => {
                let _ = self.start().map_err(LoopbackRuntimeError::from)?;
            }
            LoopbackPackageState::WaitingForConnection => {
                if !self.services.ble_connected() {
                    self.services.log("waiting for BLE connection");
                    return Ok(LoopbackTick::WaitingForConnection);
                }

                self.state = LoopbackPackageState::Ready;
                self.services.log("BLE connection established");
            }
            LoopbackPackageState::Failed(reason) => {
                self.services.log(reason);
                return Ok(LoopbackTick::Idle);
            }
            LoopbackPackageState::Ready => {}
        }

        let Some(frame) = self.services.receive_ble_frame() else {
            self.services.log("waiting for BLE frame");
            return Ok(LoopbackTick::Idle);
        };

        self.services.log("received BLE frame");
        let packet = match LoopbackPacket::decode(frame.as_slice()) {
            Ok(packet) => packet,
            Err(error) => {
                self.state = LoopbackPackageState::Failed("malformed BLE frame");
                self.services.log("malformed BLE frame");
                return Err(LoopbackRuntimeError::from(error));
            }
        };
        let command = packet.frame().command();

        match command {
            WireCommand::Ping => self.reply(WireCommand::Ping, &[]),
            WireCommand::Echo => self.reply(WireCommand::Echo, packet.frame().payload()),
            WireCommand::Status => self.reply_status(),
            WireCommand::Teardown => self.reply(WireCommand::Teardown, &[]),
        }

        self.services.log("replied to BLE frame");
        Ok(LoopbackTick::Replied(command))
    }

    fn reply(&self, command: WireCommand, payload: &[u8]) {
        let packet = LoopbackPacket::new(command, payload).expect("response payload fits");
        let (bytes, len) = packet.encode();
        self.services
            .send_ble_frame(BleFrame::from_slice(&bytes[..len]));
    }

    fn reply_status(&self) {
        let payload = self.services.now_ms().to_le_bytes();
        self.reply(WireCommand::Status, &payload);
    }
}

pub fn handle_loopback_frame(
    bytes: &[u8],
    now_ms: u64,
) -> Result<([u8; 19], usize), LoopbackError> {
    if bytes.len() < 3 {
        return Err(LoopbackError::FrameTooShort);
    }

    let actual_version = WireVersion::new(bytes[0]);
    if actual_version != BLE_LOOPBACK_PROTOCOL_VERSION {
        return Err(LoopbackError::InvalidVersion {
            expected: BLE_LOOPBACK_PROTOCOL_VERSION,
            actual: actual_version,
        });
    }

    let command =
        WireCommand::from_code(bytes[1]).ok_or(LoopbackError::InvalidCommand { code: bytes[1] })?;
    let payload_len = bytes[2] as usize;
    if payload_len > MAX_LOOPBACK_PAYLOAD_BYTES {
        return Err(LoopbackError::PayloadTooLong {
            len: payload_len,
            max: MAX_LOOPBACK_PAYLOAD_BYTES,
        });
    }

    let required = 3 + payload_len;
    if bytes.len() < required {
        return Err(LoopbackError::FrameTooShort);
    }

    let status_bytes = now_ms.to_le_bytes();
    let payload = match command {
        WireCommand::Ping | WireCommand::Teardown => &[][..],
        WireCommand::Echo => &bytes[3..required],
        WireCommand::Status => &status_bytes,
    };

    let mut response = [0_u8; 19];
    response[0] = BLE_LOOPBACK_PROTOCOL_VERSION.raw();
    response[1] = command.code();
    response[2] = payload.len() as u8;

    let mut index = 0;
    while index < payload.len() {
        response[3 + index] = payload[index];
        index += 1;
    }

    Ok((response, 3 + payload.len()))
}

#[cfg(not(test))]
unsafe extern "C" fn app_data_handler(data: *mut u8, len: u32) {
    if data.is_null() {
        return;
    }

    let bytes = core::slice::from_raw_parts(data as *const u8, len as usize);
    let now_ticks = crate::ffi::raw::vesc_system_time_ticks();
    let now_ms = u64::from(now_ticks) / 10;

    if let Ok((response, response_len)) = handle_loopback_frame(bytes, now_ms) {
        crate::ffi::raw::vesc_send_app_data(response.as_ptr(), response_len as u32);
    }
}

#[derive(Debug)]
pub struct NullDeviceServices;

impl DeviceServices for NullDeviceServices {
    fn now_ms(&self) -> u64 {
        0
    }

    fn log(&self, _message: &'static str) {}

    fn init_ble_loopback(&self) -> Result<(), &'static str> {
        Ok(())
    }

    fn ble_connected(&self) -> bool {
        false
    }

    fn receive_ble_frame(&self) -> Option<BleFrame> {
        None
    }

    fn send_ble_frame(&self, _frame: BleFrame) {}
}

#[derive(Debug)]
pub struct FakeDeviceServices {
    now_ms: Cell<u64>,
    ble_connected: Cell<bool>,
    ble_init_error: Cell<Option<&'static str>>,
    logs: RefCell<[Option<&'static str>; MAX_LOGS]>,
    log_count: Cell<usize>,
    inbox: RefCell<[Option<BleFrame>; MAX_FRAMES]>,
    inbox_count: Cell<usize>,
    inbox_cursor: Cell<usize>,
    outbox: RefCell<[Option<BleFrame>; MAX_FRAMES]>,
    outbox_count: Cell<usize>,
}

impl Default for FakeDeviceServices {
    fn default() -> Self {
        Self {
            now_ms: Cell::new(0),
            ble_connected: Cell::new(false),
            ble_init_error: Cell::new(None),
            logs: RefCell::new([None; MAX_LOGS]),
            log_count: Cell::new(0),
            inbox: RefCell::new([None; MAX_FRAMES]),
            inbox_count: Cell::new(0),
            inbox_cursor: Cell::new(0),
            outbox: RefCell::new([None; MAX_FRAMES]),
            outbox_count: Cell::new(0),
        }
    }
}

impl FakeDeviceServices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_now_ms(&self, now_ms: u64) {
        self.now_ms.set(now_ms);
    }

    pub fn set_ble_connected(&self, connected: bool) {
        self.ble_connected.set(connected);
    }

    pub fn set_ble_init_error(&self, error: Option<&'static str>) {
        self.ble_init_error.set(error);
    }

    pub fn queue_ble_frame(&self, frame: BleFrame) {
        let index = self.inbox_count.get();
        assert!(index < MAX_FRAMES, "too many queued BLE frames");

        self.inbox.borrow_mut()[index] = Some(frame);
        self.inbox_count.set(index + 1);
    }

    pub fn log_count(&self) -> usize {
        self.log_count.get()
    }

    pub fn log_at(&self, index: usize) -> Option<&'static str> {
        self.logs.borrow().get(index).copied().flatten()
    }

    pub fn transmitted_frame_count(&self) -> usize {
        self.outbox_count.get()
    }

    pub fn transmitted_frame_at(&self, index: usize) -> Option<BleFrame> {
        self.outbox.borrow().get(index).copied().flatten()
    }
}

impl DeviceServices for FakeDeviceServices {
    fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }

    fn log(&self, message: &'static str) {
        let index = self.log_count.get();
        assert!(index < MAX_LOGS, "too many logged BLE events");

        self.logs.borrow_mut()[index] = Some(message);
        self.log_count.set(index + 1);
    }

    fn init_ble_loopback(&self) -> Result<(), &'static str> {
        self.ble_init_error.get().map_or(Ok(()), Err)
    }

    fn ble_connected(&self) -> bool {
        self.ble_connected.get()
    }

    fn receive_ble_frame(&self) -> Option<BleFrame> {
        let cursor = self.inbox_cursor.get();
        if cursor >= self.inbox_count.get() {
            return None;
        }

        self.inbox_cursor.set(cursor + 1);
        self.inbox.borrow().get(cursor).copied().flatten()
    }

    fn send_ble_frame(&self, frame: BleFrame) {
        let index = self.outbox_count.get();
        assert!(index < MAX_FRAMES, "too many transmitted BLE frames");

        self.outbox.borrow_mut()[index] = Some(frame);
        self.outbox_count.set(index + 1);
    }
}

impl DeviceServices for &FakeDeviceServices {
    fn now_ms(&self) -> u64 {
        FakeDeviceServices::now_ms(self)
    }

    fn log(&self, message: &'static str) {
        FakeDeviceServices::log(self, message)
    }

    fn init_ble_loopback(&self) -> Result<(), &'static str> {
        FakeDeviceServices::init_ble_loopback(self)
    }

    fn ble_connected(&self) -> bool {
        FakeDeviceServices::ble_connected(self)
    }

    fn receive_ble_frame(&self) -> Option<BleFrame> {
        FakeDeviceServices::receive_ble_frame(self)
    }

    fn send_ble_frame(&self, frame: BleFrame) {
        FakeDeviceServices::send_ble_frame(self, frame)
    }
}

static INIT_CALLS: AtomicUsize = AtomicUsize::new(0);
static STOP_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn stop_package(_arg: *mut core::ffi::c_void) {
    #[cfg(not(test))]
    {
        let _ =
            crate::ffi::LoopbackLifecycle::new(crate::ffi::RealBindings).clear_app_data_handler();
    }

    #[cfg(test)]
    {
        STOP_CALLS.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(not(test))]
pub fn init_package(info: *mut crate::ffi::LibInfo) -> bool {
    let Some(info_ref) = (unsafe { info.as_ref() }) else {
        return false;
    };
    let image = crate::ffi::NativeImage::from_info(info_ref);
    let lifecycle = crate::ffi::LoopbackLifecycle::new(crate::ffi::RealBindings);
    if unsafe { lifecycle.install(info, image, stop_package, app_data_handler) } {
        return true;
    }

    if let Some(info) = unsafe { info.as_mut() } {
        info.stop_fun = Some(stop_package);
    }
    true
}

pub fn init_package_for_tests(info: *mut crate::ffi::LibInfo) -> bool {
    if let Some(info) = unsafe { info.as_mut() } {
        info.stop_fun = Some(stop_package);
    }
    INIT_CALLS.fetch_add(1, Ordering::SeqCst);
    true
}

pub fn reset_init_call_count_for_tests() {
    INIT_CALLS.store(0, Ordering::SeqCst);
    STOP_CALLS.store(0, Ordering::SeqCst);
}

pub fn init_call_count_for_tests() -> usize {
    INIT_CALLS.load(Ordering::SeqCst)
}

pub fn stop_call_count_for_tests() -> usize {
    STOP_CALLS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::{
        handle_loopback_frame, init_call_count_for_tests, reset_init_call_count_for_tests,
        stop_call_count_for_tests, BleFrame, FakeDeviceServices, LoopbackPackageRuntime,
        LoopbackPackageState, LoopbackTick,
    };
    use crate::ffi;
    use core::cell::Cell;
    use vesc_protocol::ble_loopback::LoopbackPacket;
    use vesc_protocol::WireCommand;

    fn frame(command: WireCommand, payload: &[u8]) -> BleFrame {
        let packet = LoopbackPacket::new(command, payload).expect("frame");
        let (bytes, len) = packet.encode();

        BleFrame::from_slice(&bytes[..len])
    }

    #[test]
    fn runtime_records_a_failed_start_when_ble_init_fails() {
        let services = FakeDeviceServices::new();
        services.set_ble_init_error(Some("BLE init failed"));

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(
            runtime.start(),
            Err(super::LoopbackStartError::InitFailed("BLE init failed"))
        );
        assert_eq!(
            runtime.state(),
            LoopbackPackageState::Failed("BLE init failed")
        );
        assert_eq!(services.log_at(0), Some("booting BLE loopback package"));
        assert_eq!(services.log_at(1), Some("BLE init failed"));
    }

    #[test]
    fn runtime_waits_for_connection_before_consuming_frames() {
        let services = FakeDeviceServices::new();
        services.queue_ble_frame(frame(WireCommand::Ping, &[]));

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(
            runtime.start(),
            Ok(LoopbackPackageState::WaitingForConnection)
        );
        assert_eq!(runtime.tick(), Ok(LoopbackTick::WaitingForConnection));
        assert_eq!(services.transmitted_frame_count(), 0);
    }

    #[test]
    fn runtime_echoes_ping_echo_and_status_frames() {
        let services = FakeDeviceServices::new();
        services.set_now_ms(0x0102_0304_0506_0708);
        services.set_ble_connected(true);
        services.queue_ble_frame(frame(WireCommand::Ping, &[]));
        services.queue_ble_frame(frame(WireCommand::Echo, &[9, 8]));
        services.queue_ble_frame(frame(WireCommand::Status, &[]));

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(runtime.start(), Ok(LoopbackPackageState::Ready));
        assert_eq!(runtime.tick(), Ok(LoopbackTick::Replied(WireCommand::Ping)));
        assert_eq!(runtime.tick(), Ok(LoopbackTick::Replied(WireCommand::Echo)));
        assert_eq!(
            runtime.tick(),
            Ok(LoopbackTick::Replied(WireCommand::Status))
        );

        assert_eq!(
            services
                .transmitted_frame_at(0)
                .expect("ping reply")
                .as_slice(),
            frame(WireCommand::Ping, &[]).as_slice()
        );
        assert_eq!(
            services
                .transmitted_frame_at(1)
                .expect("echo reply")
                .as_slice(),
            frame(WireCommand::Echo, &[9, 8]).as_slice()
        );
        assert_eq!(
            services
                .transmitted_frame_at(2)
                .expect("status reply")
                .as_slice(),
            frame(
                WireCommand::Status,
                &0x0102_0304_0506_0708_u64.to_le_bytes()
            )
            .as_slice()
        );
    }

    #[test]
    fn package_entrypoint_records_device_initialization() {
        reset_init_call_count_for_tests();

        assert!(super::init_package_for_tests(core::ptr::null_mut()));

        assert_eq!(init_call_count_for_tests(), 1);
    }

    #[test]
    fn package_entrypoint_installs_a_stop_hook() {
        reset_init_call_count_for_tests();
        let mut info = crate::ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(super::init_package_for_tests(&mut info));

        let stop_fun = info.stop_fun.expect("stop hook");
        unsafe {
            stop_fun(info.arg);
        }
        assert_eq!(stop_call_count_for_tests(), 1);
    }

    struct FakeAppDataBindings {
        calls: Cell<usize>,
        last_handler: Cell<usize>,
    }

    impl FakeAppDataBindings {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
                last_handler: Cell::new(usize::MAX),
            }
        }
    }

    impl ffi::AppDataBindings for FakeAppDataBindings {
        unsafe fn set_app_data_handler(&self, handler: Option<ffi::AppDataHandler>) -> bool {
            self.calls.set(self.calls.get() + 1);
            self.last_handler
                .set(handler.map_or(0, |handler| handler as *const () as usize));
            true
        }
    }

    unsafe extern "C" fn stub_stop_handler(_arg: *mut core::ffi::c_void) {}

    unsafe extern "C" fn stub_app_data_handler(_data: *mut u8, _len: u32) {}

    #[test]
    fn lifecycle_descriptor_installs_the_stop_hook() {
        let bindings = FakeAppDataBindings::new();
        let lifecycle = crate::ffi::LoopbackLifecycle::new(bindings);
        let image = ffi::NativeImage::new(0x2000);
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert!(unsafe {
            lifecycle.install(&mut info, image, stub_stop_handler, stub_app_data_handler)
        });

        assert_eq!(
            info.stop_fun.expect("stop hook") as *const () as usize,
            stub_stop_handler as *const () as usize
        );
        assert_eq!(lifecycle.bindings().calls.get(), 0);
    }

    #[test]
    fn lifecycle_registers_the_app_data_handler_separately() {
        let bindings = FakeAppDataBindings::new();
        let lifecycle = crate::ffi::LoopbackLifecycle::new(bindings);

        assert!(lifecycle.register_app_data_handler(stub_app_data_handler));

        assert_eq!(lifecycle.bindings().calls.get(), 1);
        assert_eq!(
            lifecycle.bindings().last_handler.get(),
            stub_app_data_handler as *const () as usize
        );
    }

    #[test]
    fn lifecycle_cleanup_clears_the_package_app_data_handler() {
        let bindings = FakeAppDataBindings::new();
        let lifecycle = crate::ffi::LoopbackLifecycle::new(bindings);

        assert!(lifecycle.clear_app_data_handler());

        assert_eq!(lifecycle.bindings().calls.get(), 1);
        assert_eq!(lifecycle.bindings().last_handler.get(), 0);
    }

    #[test]
    fn app_data_ping_echoes_the_frame_back() {
        let request = frame(WireCommand::Ping, &[]);
        let (response, len) =
            handle_loopback_frame(request.as_slice(), 1234).expect("loopback response");

        assert_eq!(&response[..len], frame(WireCommand::Ping, &[]).as_slice());
    }

    #[test]
    fn app_data_status_uses_device_time() {
        let request = frame(WireCommand::Status, &[]);
        let (response, len) =
            handle_loopback_frame(request.as_slice(), 0x0102_0304_0506_0708).expect("status");

        assert_eq!(
            &response[..len],
            frame(
                WireCommand::Status,
                &0x0102_0304_0506_0708_u64.to_le_bytes()
            )
            .as_slice()
        );
    }

    #[test]
    fn app_data_rejects_invalid_frames() {
        assert!(handle_loopback_frame(&[9, 1, 0], 0).is_err());
    }
}
