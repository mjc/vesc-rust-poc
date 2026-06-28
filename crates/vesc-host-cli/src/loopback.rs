use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;

use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::{LoopbackError, LoopbackPacket};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackTarget {
    device_name_hint: String,
    service_name_hint: String,
    address: Option<String>,
    require_explicit_match: bool,
}

impl LoopbackTarget {
    pub fn new(device_name_hint: impl Into<String>, service_name_hint: impl Into<String>) -> Self {
        Self {
            device_name_hint: device_name_hint.into(),
            service_name_hint: service_name_hint.into(),
            address: None,
            require_explicit_match: false,
        }
    }

    pub fn named(device_name: impl Into<String>) -> Self {
        Self {
            device_name_hint: device_name.into(),
            service_name_hint: "vesc-loopback-service".to_owned(),
            address: None,
            require_explicit_match: true,
        }
    }

    pub fn addressed(address: impl Into<String>) -> Self {
        Self {
            device_name_hint: "vesc-loopback-test".to_owned(),
            service_name_hint: "vesc-loopback-service".to_owned(),
            address: Some(address.into()),
            require_explicit_match: true,
        }
    }

    pub fn device_name_hint(&self) -> &str {
        &self.device_name_hint
    }

    pub fn service_name_hint(&self) -> &str {
        &self.service_name_hint
    }

    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    pub fn requires_explicit_match(&self) -> bool {
        self.require_explicit_match
    }
}

impl Default for LoopbackTarget {
    fn default() -> Self {
        Self::new("vesc-loopback-test", "vesc-loopback-service")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopbackTransportError {
    ScanTimeout,
    ConnectFailed,
    MissingService,
    Protocol(LoopbackError),
    Device(&'static str),
}

impl fmt::Display for LoopbackTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScanTimeout => f.write_str("scan timed out while opening the BLE transport"),
            Self::ConnectFailed => f.write_str("failed to connect to the BLE device"),
            Self::MissingService => f.write_str("missing BLE loopback service"),
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::Device(reason) => write!(f, "device error: {reason}"),
        }
    }
}

impl std::error::Error for LoopbackTransportError {}

pub trait LoopbackTransport {
    fn open(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError>;
    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackReport {
    target: LoopbackTarget,
    commands: Vec<WireCommand>,
}

impl LoopbackReport {
    pub fn target(&self) -> &LoopbackTarget {
        &self.target
    }

    pub fn commands(&self) -> &[WireCommand] {
        &self.commands
    }
}

#[derive(Debug, Default)]
pub struct FakeLoopbackTransport {
    open_result: RefCell<Option<Result<(), LoopbackTransportError>>>,
    open_target: RefCell<Option<LoopbackTarget>>,
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

    pub fn open_target(&self) -> Option<LoopbackTarget> {
        self.open_target.borrow().clone()
    }

    pub fn queue_response(&self, response: Result<Vec<u8>, LoopbackTransportError>) {
        self.responses.borrow_mut().push_back(response);
    }

    pub fn requests(&self) -> Vec<Vec<u8>> {
        self.requests.borrow().clone()
    }
}

impl LoopbackTransport for FakeLoopbackTransport {
    fn open(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        *self.open_target.borrow_mut() = Some(target);
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
    let target = LoopbackTarget::default();
    transport.open(target.clone())?;
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

    Ok(LoopbackReport { target, commands })
}

#[cfg(test)]
mod tests {
    use super::{FakeLoopbackTransport, LoopbackTarget, LoopbackTransportError, run_loopback};
    use vesc_protocol::WireCommand;

    #[test]
    fn loopback_transport_behavior() {
        {
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
            assert_eq!(report.target(), &LoopbackTarget::default());
            assert_eq!(transport.open_target(), Some(LoopbackTarget::default()));
            assert_eq!(report.target().device_name_hint(), "vesc-loopback-test");
            assert_eq!(report.target().service_name_hint(), "vesc-loopback-service");
        }

        {
            let transport =
                FakeLoopbackTransport::with_open_result(Err(LoopbackTransportError::ScanTimeout));
            assert_eq!(
                run_loopback(&transport),
                Err(LoopbackTransportError::ScanTimeout)
            );
        }

        {
            let transport = FakeLoopbackTransport::new();
            transport.queue_response(Ok(vec![1, 99, 0]));
            assert_eq!(
                run_loopback(&transport),
                Err(LoopbackTransportError::Protocol(
                    vesc_protocol::ble_loopback::LoopbackError::InvalidCommand { code: 99 }
                ))
            );
        }

        assert_eq!(
            LoopbackTransportError::ScanTimeout.to_string(),
            "scan timed out while opening the BLE transport"
        );
        assert_eq!(
            LoopbackTransportError::ConnectFailed.to_string(),
            "failed to connect to the BLE device"
        );
        assert_eq!(
            LoopbackTransportError::MissingService.to_string(),
            "missing BLE loopback service"
        );
        assert_eq!(
            LoopbackTransportError::Protocol(
                vesc_protocol::ble_loopback::LoopbackError::InvalidCommand { code: 99 }
            )
            .to_string(),
            "protocol error: invalid command code: 99"
        );
        assert_eq!(
            LoopbackTransportError::Device("boom").to_string(),
            "device error: boom"
        );
    }
}
