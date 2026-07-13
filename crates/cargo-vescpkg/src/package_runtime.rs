use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::{LoopbackError, LoopbackPacket, handle_loopback_frame};

/// Firmware-facing services required by the loopback package runtime.
pub trait FirmwareServices {
    /// Return the current firmware time in milliseconds.
    fn now_ms(&self) -> u64;
    /// Record a runtime log message.
    fn log(&self, message: &str);
    /// Initialize the BLE loopback transport.
    fn init_ble_loopback(&self) -> Result<(), &'static str>;
    /// Report whether BLE is currently connected.
    fn ble_connected(&self) -> bool;
    /// Pull the next inbound BLE frame, if any.
    fn receive_ble_frame(&self) -> Option<Vec<u8>>;
    /// Send one outbound BLE frame.
    fn send_ble_frame(&self, bytes: &[u8]);
}

/// Runtime state for the loopback package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackPackageState {
    /// Startup has begun but BLE is not ready yet.
    Booting,
    /// BLE initialized but no connection is present yet.
    WaitingForConnection,
    /// BLE is connected and frames may be processed.
    Ready,
    /// The runtime hit an unrecoverable error.
    Failed(&'static str),
}

/// Tick outcomes produced by the loopback runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackTick {
    /// Nothing happened on this tick.
    Idle,
    /// The runtime is still waiting for a BLE connection.
    WaitingForConnection,
    /// A frame was handled and replied to.
    Replied(WireCommand),
}

/// Errors that can occur while starting the loopback runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackStartError {
    /// BLE initialization failed.
    InitFailed(&'static str),
}

/// Errors that can occur while ticking the loopback runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackRuntimeError {
    /// Startup failed before the runtime could process frames.
    Start(LoopbackStartError),
    /// A loopback frame was malformed.
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

/// Test-only fake firmware services used by the runtime tests.
#[derive(Debug, Default)]
pub struct FakeFirmwareServices {
    now_ms: Cell<u64>,
    ble_connected: Cell<bool>,
    ble_init_error: RefCell<Option<&'static str>>,
    logs: RefCell<Vec<String>>,
    inbox: RefCell<VecDeque<Vec<u8>>>,
    outbox: RefCell<Vec<Vec<u8>>>,
}

impl FakeFirmwareServices {
    /// Construct a new fake service set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current time returned by the fake services.
    pub fn set_now_ms(&self, now_ms: u64) {
        self.now_ms.set(now_ms);
    }

    /// Set whether the fake services report a BLE connection.
    pub fn set_ble_connected(&self, connected: bool) {
        self.ble_connected.set(connected);
    }

    /// Configure BLE initialization to fail with an optional reason.
    pub fn set_ble_init_error(&self, error: Option<&'static str>) {
        *self.ble_init_error.borrow_mut() = error;
    }

    /// Return the recorded firmware time.
    pub fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }

    /// Queue one inbound BLE frame.
    pub fn queue_ble_frame(&self, bytes: impl Into<Vec<u8>>) {
        self.inbox.borrow_mut().push_back(bytes.into());
    }

    /// Record one log message.
    pub fn log(&self, message: &str) {
        self.logs.borrow_mut().push(message.to_owned());
    }

    /// Initialize the fake BLE loopback transport.
    pub fn init_ble_loopback(&self) -> Result<(), &'static str> {
        self.ble_init_error.borrow().map_or(Ok(()), Err)
    }

    /// Return whether the fake reports a BLE connection.
    pub fn ble_connected(&self) -> bool {
        self.ble_connected.get()
    }

    /// Pop the next queued BLE frame.
    pub fn receive_ble_frame(&self) -> Option<Vec<u8>> {
        self.inbox.borrow_mut().pop_front()
    }

    /// Record one transmitted BLE frame.
    pub fn send_ble_frame(&self, bytes: &[u8]) {
        self.outbox.borrow_mut().push(bytes.to_vec());
    }

    /// Return the recorded log messages.
    pub fn logs(&self) -> Vec<String> {
        self.logs.borrow().clone()
    }

    /// Return the frames transmitted by the fake.
    pub fn transmitted_frames(&self) -> Vec<Vec<u8>> {
        self.outbox.borrow().clone()
    }
}

impl FirmwareServices for FakeFirmwareServices {
    fn now_ms(&self) -> u64 {
        Self::now_ms(self)
    }

    fn log(&self, message: &str) {
        Self::log(self, message)
    }

    fn init_ble_loopback(&self) -> Result<(), &'static str> {
        Self::init_ble_loopback(self)
    }

    fn ble_connected(&self) -> bool {
        Self::ble_connected(self)
    }

    fn receive_ble_frame(&self) -> Option<Vec<u8>> {
        Self::receive_ble_frame(self)
    }

    fn send_ble_frame(&self, bytes: &[u8]) {
        Self::send_ble_frame(self, bytes)
    }
}

impl FirmwareServices for &FakeFirmwareServices {
    fn now_ms(&self) -> u64 {
        FakeFirmwareServices::now_ms(self)
    }

    fn log(&self, message: &str) {
        FakeFirmwareServices::log(self, message)
    }

    fn init_ble_loopback(&self) -> Result<(), &'static str> {
        FakeFirmwareServices::init_ble_loopback(self)
    }

    fn ble_connected(&self) -> bool {
        FakeFirmwareServices::ble_connected(self)
    }

    fn receive_ble_frame(&self) -> Option<Vec<u8>> {
        FakeFirmwareServices::receive_ble_frame(self)
    }

    fn send_ble_frame(&self, bytes: &[u8]) {
        FakeFirmwareServices::send_ble_frame(self, bytes)
    }
}

/// Stateful loopback package runtime.
pub struct LoopbackPackageRuntime<S> {
    services: S,
    state: LoopbackPackageState,
}

impl<S: FirmwareServices> LoopbackPackageRuntime<S> {
    /// Construct a runtime around the provided services.
    pub fn new(services: S) -> Self {
        Self {
            services,
            state: LoopbackPackageState::Booting,
        }
    }

    /// Return the current runtime state.
    pub fn state(&self) -> LoopbackPackageState {
        self.state
    }

    /// Start the runtime and transition it to the appropriate steady state.
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
                self.services.log(&format!("BLE init failed: {reason}"));
                return Err(LoopbackStartError::InitFailed(reason));
            }
        }

        Ok(self.state)
    }

    /// Advance the runtime once and process at most one frame.
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

        let Some(bytes) = self.services.receive_ble_frame() else {
            self.services.log("waiting for BLE frame");
            return Ok(LoopbackTick::Idle);
        };

        self.services.log("received BLE frame");
        let packet = match LoopbackPacket::decode(&bytes) {
            Ok(packet) => packet,
            Err(error) => {
                self.state = LoopbackPackageState::Failed("malformed BLE frame");
                self.services.log("malformed BLE frame");
                return Err(LoopbackRuntimeError::from(error));
            }
        };
        let command = packet.frame().command();

        match handle_loopback_frame(&bytes, self.services.now_ms()) {
            Ok((response, len)) => {
                self.services.send_ble_frame(&response[..len]);
            }
            Err(error) => {
                self.state = LoopbackPackageState::Failed("malformed BLE frame");
                self.services.log("malformed BLE frame");
                return Err(LoopbackRuntimeError::from(error));
            }
        }

        self.services.log("replied to BLE frame");
        Ok(LoopbackTick::Replied(command))
    }
}
