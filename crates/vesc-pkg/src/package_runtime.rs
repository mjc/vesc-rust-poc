use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::{LoopbackError, LoopbackPacket, handle_loopback_frame};

pub trait FirmwareServices {
    fn now_ms(&self) -> u64;
    fn log(&self, message: &str);
    fn init_ble_loopback(&self) -> Result<(), &'static str>;
    fn ble_connected(&self) -> bool;
    fn receive_ble_frame(&self) -> Option<Vec<u8>>;
    fn send_ble_frame(&self, bytes: &[u8]);
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
        *self.ble_init_error.borrow_mut() = error;
    }

    pub fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }

    pub fn queue_ble_frame(&self, bytes: impl Into<Vec<u8>>) {
        self.inbox.borrow_mut().push_back(bytes.into());
    }

    pub fn log(&self, message: &str) {
        self.logs.borrow_mut().push(message.to_owned());
    }

    pub fn init_ble_loopback(&self) -> Result<(), &'static str> {
        self.ble_init_error.borrow().map_or(Ok(()), Err)
    }

    pub fn ble_connected(&self) -> bool {
        self.ble_connected.get()
    }

    pub fn receive_ble_frame(&self) -> Option<Vec<u8>> {
        self.inbox.borrow_mut().pop_front()
    }

    pub fn send_ble_frame(&self, bytes: &[u8]) {
        self.outbox.borrow_mut().push(bytes.to_vec());
    }

    pub fn logs(&self) -> Vec<String> {
        self.logs.borrow().clone()
    }

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

pub struct LoopbackPackageRuntime<S> {
    services: S,
    state: LoopbackPackageState,
}

impl<S: FirmwareServices> LoopbackPackageRuntime<S> {
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
                self.services.log(&format!("BLE init failed: {reason}"));
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

#[cfg(test)]
mod tests {
    use super::{
        FakeFirmwareServices, LoopbackPackageRuntime, LoopbackPackageState, LoopbackRuntimeError,
        LoopbackTick,
    };
    use vesc_protocol::WireCommand;
    use vesc_protocol::ble_loopback::LoopbackPacket;

    #[test]
    fn fake_services_record_time_logs_connectivity_and_transmissions() {
        let services = FakeFirmwareServices::new();
        services.set_now_ms(42);
        services.set_ble_connected(true);
        services.log("boot");
        services.queue_ble_frame([1, 1, 0]);

        assert_eq!(services.now_ms(), 42);
        assert!(services.ble_connected());
        assert_eq!(services.logs(), ["boot".to_owned()]);
        assert_eq!(services.receive_ble_frame(), Some(vec![1, 1, 0]));
        assert_eq!(services.receive_ble_frame(), None);
    }

    #[test]
    fn runtime_records_a_failed_start_when_ble_init_fails() {
        let services = FakeFirmwareServices::new();
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
        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "BLE init failed: BLE init failed")
        );
    }

    #[test]
    fn runtime_waits_for_connection_before_consuming_frames() {
        let services = FakeFirmwareServices::new();
        services.set_ble_connected(false);
        services.queue_ble_frame([1, 1, 0]);

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(
            runtime.start(),
            Ok(LoopbackPackageState::WaitingForConnection)
        );
        assert_eq!(runtime.tick(), Ok(LoopbackTick::WaitingForConnection));
        assert_eq!(services.transmitted_frames(), Vec::<Vec<u8>>::new());
        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "waiting for BLE connection")
        );
    }

    #[test]
    fn runtime_echoes_ping_echo_and_status_frames() {
        let services = FakeFirmwareServices::new();
        services.set_now_ms(0x0102_0304_0506_0708);
        services.set_ble_connected(true);

        let ping = LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping frame");
        let echo = LoopbackPacket::new(WireCommand::Echo, &[9, 8]).expect("echo frame");
        let status = LoopbackPacket::new(WireCommand::Status, &[]).expect("status frame");

        let (ping_bytes, ping_len) = ping.encode();
        let (echo_bytes, echo_len) = echo.encode();
        let (status_bytes, status_len) = status.encode();

        services.queue_ble_frame(ping_bytes[..ping_len].to_vec());
        services.queue_ble_frame(echo_bytes[..echo_len].to_vec());
        services.queue_ble_frame(status_bytes[..status_len].to_vec());

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(runtime.start(), Ok(LoopbackPackageState::Ready));
        assert_eq!(runtime.tick(), Ok(LoopbackTick::Replied(WireCommand::Ping)));
        assert_eq!(runtime.tick(), Ok(LoopbackTick::Replied(WireCommand::Echo)));
        assert_eq!(
            runtime.tick(),
            Ok(LoopbackTick::Replied(WireCommand::Status))
        );
        assert_eq!(runtime.tick(), Ok(LoopbackTick::Idle));

        let transmitted = services.transmitted_frames();
        assert_eq!(transmitted.len(), 3);

        let decoded_ping = LoopbackPacket::decode(&transmitted[0]).expect("decoded ping response");
        assert_eq!(decoded_ping.frame().command(), WireCommand::Ping);
        assert_eq!(decoded_ping.frame().payload(), &[]);

        let decoded_echo = LoopbackPacket::decode(&transmitted[1]).expect("decoded echo response");
        assert_eq!(decoded_echo.frame().command(), WireCommand::Echo);
        assert_eq!(decoded_echo.frame().payload(), &[9, 8]);

        let decoded_status =
            LoopbackPacket::decode(&transmitted[2]).expect("decoded status response");
        assert_eq!(decoded_status.frame().command(), WireCommand::Status);
        assert_eq!(
            decoded_status.frame().payload(),
            &0x0102_0304_0506_0708_u64.to_le_bytes()
        );

        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "received BLE frame")
        );
        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "replied to BLE frame")
        );
    }

    #[test]
    fn runtime_marks_bad_frames_as_failed() {
        let services = FakeFirmwareServices::new();
        services.set_ble_connected(true);
        services.queue_ble_frame([1, 99, 0]);

        let mut runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(runtime.start(), Ok(LoopbackPackageState::Ready));
        assert_eq!(
            runtime.tick(),
            Err(LoopbackRuntimeError::Frame(
                vesc_protocol::ble_loopback::LoopbackError::InvalidCommand { code: 99 }
            ))
        );
        assert_eq!(
            runtime.state(),
            LoopbackPackageState::Failed("malformed BLE frame")
        );
        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "malformed BLE frame")
        );
    }
}
