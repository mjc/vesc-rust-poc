use std::cell::RefCell;
use std::collections::VecDeque;

use vesc_protocol::ble_loopback::{LoopbackError, LoopbackPacket};
use vesc_protocol::WireCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopbackTransportError {
    ScanTimeout,
    ConnectFailed,
    MissingService,
    Protocol(LoopbackError),
    Device(&'static str),
}

pub trait LoopbackTransport {
    fn open(&self) -> Result<(), LoopbackTransportError>;
    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackReport {
    commands: Vec<WireCommand>,
}

impl LoopbackReport {
    pub fn commands(&self) -> &[WireCommand] {
        &self.commands
    }
}

#[derive(Debug, Default)]
pub struct FakeLoopbackTransport {
    open_result: RefCell<Option<Result<(), LoopbackTransportError>>>,
    responses: RefCell<VecDeque<Result<Vec<u8>, LoopbackTransportError>>>,
    requests: RefCell<Vec<Vec<u8>>>,
}

impl FakeLoopbackTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scripted_success() -> Self {
        let transport = Self::new();
        [
            LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping response"),
            LoopbackPacket::new(WireCommand::Echo, &[9, 8]).expect("echo response"),
            LoopbackPacket::new(WireCommand::Status, &42_u64.to_le_bytes())
                .expect("status response"),
            LoopbackPacket::new(WireCommand::Teardown, &[]).expect("teardown response"),
        ]
        .iter()
        .for_each(|packet| {
            let (bytes, len) = packet.encode();
            transport.queue_response(Ok(bytes[..len].to_vec()));
        });
        transport
    }

    pub fn with_open_result(result: Result<(), LoopbackTransportError>) -> Self {
        let transport = Self::new();
        *transport.open_result.borrow_mut() = Some(result);
        transport
    }

    pub fn queue_response(&self, response: Result<Vec<u8>, LoopbackTransportError>) {
        self.responses.borrow_mut().push_back(response);
    }

    pub fn requests(&self) -> Vec<Vec<u8>> {
        self.requests.borrow().clone()
    }
}

impl LoopbackTransport for FakeLoopbackTransport {
    fn open(&self) -> Result<(), LoopbackTransportError> {
        self.open_result.borrow_mut().take().unwrap_or(Ok(()))
    }

    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError> {
        self.requests.borrow_mut().push(request.to_vec());
        self.responses
            .borrow_mut()
            .pop_front()
            .unwrap_or(Err(LoopbackTransportError::Device(
                "missing scripted response",
            )))
    }
}

pub fn run_loopback<T: LoopbackTransport>(
    transport: &T,
) -> Result<LoopbackReport, LoopbackTransportError> {
    transport.open()?;
    let steps = [
        LoopbackPacket::new(WireCommand::Ping, &[]).expect("ping packet"),
        LoopbackPacket::new(WireCommand::Echo, &[9, 8]).expect("echo packet"),
        LoopbackPacket::new(WireCommand::Status, &[]).expect("status packet"),
        LoopbackPacket::new(WireCommand::Teardown, &[]).expect("teardown packet"),
    ];

    let commands = steps
        .iter()
        .map(|packet| {
            let (bytes, len) = packet.encode();
            let response = transport.exchange(&bytes[..len])?;
            let decoded =
                LoopbackPacket::decode(&response).map_err(LoopbackTransportError::Protocol)?;
            Ok(decoded.frame().command())
        })
        .collect::<Result<Vec<_>, LoopbackTransportError>>()?;

    Ok(LoopbackReport { commands })
}

#[cfg(test)]
mod tests {
    use super::{run_loopback, FakeLoopbackTransport, LoopbackTransportError};
    use vesc_protocol::WireCommand;

    #[test]
    fn loopback_runs_a_deterministic_protocol_exchange() {
        let transport = FakeLoopbackTransport::scripted_success();

        let report = run_loopback(&transport).expect("loopback report");

        assert_eq!(
            report.commands(),
            &[
                WireCommand::Ping,
                WireCommand::Echo,
                WireCommand::Status,
                WireCommand::Teardown
            ]
        );
        assert_eq!(transport.requests().len(), 4);
    }

    #[test]
    fn loopback_propagates_open_failures() {
        let transport =
            FakeLoopbackTransport::with_open_result(Err(LoopbackTransportError::ScanTimeout));

        assert_eq!(
            run_loopback(&transport),
            Err(LoopbackTransportError::ScanTimeout)
        );
    }

    #[test]
    fn loopback_reports_invalid_device_responses() {
        let transport = FakeLoopbackTransport::new();
        let bad_response = vec![1, 99, 0];
        transport.queue_response(Ok(bad_response));

        assert_eq!(
            run_loopback(&transport),
            Err(LoopbackTransportError::Protocol(
                vesc_protocol::ble_loopback::LoopbackError::InvalidCommand { code: 99 }
            ))
        );
    }
}
