use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use vesc_protocol::ble_loopback::{LoopbackError, LoopbackPacket};
use vesc_protocol::WireCommand;

pub trait FirmwareServices {
    fn now_ms(&self) -> u64;
    fn log(&self, message: &str);
    fn receive_ble_frame(&self) -> Option<Vec<u8>>;
    fn send_ble_frame(&self, bytes: &[u8]);
}

#[derive(Debug, Default)]
pub struct FakeFirmwareServices {
    now_ms: Cell<u64>,
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

    pub fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }

    pub fn queue_ble_frame(&self, bytes: impl Into<Vec<u8>>) {
        self.inbox.borrow_mut().push_back(bytes.into());
    }

    pub fn log(&self, message: &str) {
        self.logs.borrow_mut().push(message.to_owned());
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

    fn receive_ble_frame(&self) -> Option<Vec<u8>> {
        FakeFirmwareServices::receive_ble_frame(self)
    }

    fn send_ble_frame(&self, bytes: &[u8]) {
        FakeFirmwareServices::send_ble_frame(self, bytes)
    }
}

pub struct LoopbackPackageRuntime<S> {
    services: S,
}

impl<S: FirmwareServices> LoopbackPackageRuntime<S> {
    pub fn new(services: S) -> Self {
        Self { services }
    }

    pub fn tick(&self) -> Result<bool, LoopbackError> {
        let Some(bytes) = self.services.receive_ble_frame() else {
            return Ok(false);
        };

        self.services.log("received BLE frame");
        let packet = LoopbackPacket::decode(&bytes)?;

        match packet.frame().command() {
            WireCommand::Ping => self.reply(WireCommand::Ping, &[]),
            WireCommand::Echo => self.reply(WireCommand::Echo, packet.frame().payload()),
            WireCommand::Status => self.reply_status(),
            WireCommand::Teardown => self.reply(WireCommand::Teardown, &[]),
        }

        Ok(true)
    }

    fn reply(&self, command: WireCommand, payload: &[u8]) {
        let packet = LoopbackPacket::new(command, payload).expect("response payload fits");
        let (bytes, len) = packet.encode();
        self.services.send_ble_frame(&bytes[..len]);
    }

    fn reply_status(&self) {
        let payload = self.services.now_ms().to_le_bytes();
        self.reply(WireCommand::Status, &payload);
    }
}

#[cfg(test)]
mod tests {
    use super::{FakeFirmwareServices, LoopbackPackageRuntime};
    use vesc_protocol::ble_loopback::LoopbackPacket;
    use vesc_protocol::WireCommand;

    #[test]
    fn fake_services_record_time_logs_and_transmissions() {
        let services = FakeFirmwareServices::new();
        services.set_now_ms(42);
        services.log("boot");
        services.queue_ble_frame([1, 1, 0]);

        assert_eq!(services.now_ms(), 42);
        assert_eq!(services.logs(), ["boot".to_owned()]);
        assert_eq!(services.receive_ble_frame(), Some(vec![1, 1, 0]));
        assert_eq!(services.receive_ble_frame(), None);
    }

    #[test]
    fn runtime_echoes_ping_echo_and_status_frames() {
        let services = FakeFirmwareServices::new();
        services.set_now_ms(0x0102_0304_0506_0708);

        let ping = LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping frame");
        let echo = LoopbackPacket::new(WireCommand::Echo, &[9, 8]).expect("echo frame");
        let status = LoopbackPacket::new(WireCommand::Status, &[]).expect("status frame");

        let (ping_bytes, ping_len) = ping.encode();
        let (echo_bytes, echo_len) = echo.encode();
        let (status_bytes, status_len) = status.encode();

        services.queue_ble_frame(ping_bytes[..ping_len].to_vec());
        services.queue_ble_frame(echo_bytes[..echo_len].to_vec());
        services.queue_ble_frame(status_bytes[..status_len].to_vec());

        let runtime = LoopbackPackageRuntime::new(&services);

        assert_eq!(runtime.tick(), Ok(true));
        assert_eq!(runtime.tick(), Ok(true));
        assert_eq!(runtime.tick(), Ok(true));
        assert_eq!(runtime.tick(), Ok(false));

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

        assert!(services
            .logs()
            .iter()
            .any(|line| line == "received BLE frame"));
    }
}
