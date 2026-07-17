use std::fmt;

use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackError;

/// Target selection for BLE loopback and related diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackTarget {
    device_name_hint: String,
    service_name_hint: String,
    address: Option<String>,
    require_explicit_match: bool,
}

impl LoopbackTarget {
    /// Creates a target from broad device and service name hints.
    pub fn new(device_name_hint: impl Into<String>, service_name_hint: impl Into<String>) -> Self {
        Self {
            device_name_hint: device_name_hint.into(),
            service_name_hint: service_name_hint.into(),
            address: None,
            require_explicit_match: false,
        }
    }

    /// Creates a target that must match the given BLE device name.
    pub fn named(device_name: impl Into<String>) -> Self {
        Self {
            device_name_hint: device_name.into(),
            service_name_hint: "vesc-loopback-service".to_owned(),
            address: None,
            require_explicit_match: true,
        }
    }

    /// Creates a target that must match the given BLE address.
    pub fn addressed(address: impl Into<String>) -> Self {
        Self {
            device_name_hint: "vesc-loopback-test".to_owned(),
            service_name_hint: "vesc-loopback-service".to_owned(),
            address: Some(address.into()),
            require_explicit_match: true,
        }
    }

    /// Returns the preferred BLE device-name match hint.
    pub fn device_name_hint(&self) -> &str {
        &self.device_name_hint
    }

    /// Returns the preferred BLE service-name match hint.
    pub fn service_name_hint(&self) -> &str {
        &self.service_name_hint
    }

    /// Returns the explicit BLE address filter, if one was provided.
    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    /// Returns whether discovery must match the provided name or address exactly.
    pub fn requires_explicit_match(&self) -> bool {
        self.require_explicit_match
    }
}

impl Default for LoopbackTarget {
    fn default() -> Self {
        Self::new("vesc-loopback-test", "vesc-loopback-service")
    }
}

/// Errors returned by loopback transports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopbackTransportError {
    /// A loopback protocol frame was invalid or unexpected.
    Protocol(LoopbackError),
    /// The device or transport reported a human-readable failure.
    Device(String),
}

impl fmt::Display for LoopbackTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::Device(reason) => write!(f, "device error: {reason}"),
        }
    }
}

impl std::error::Error for LoopbackTransportError {}

/// Successful loopback run summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackReport {
    target: LoopbackTarget,
    commands: Vec<WireCommand>,
}

impl LoopbackReport {
    /// Creates a loopback report from the selected target and observed commands.
    pub fn new(target: LoopbackTarget, commands: Vec<WireCommand>) -> Self {
        Self { target, commands }
    }

    /// Returns the target used by the loopback run.
    pub fn target(&self) -> &LoopbackTarget {
        &self.target
    }

    /// Returns the response commands decoded during the loopback run.
    pub fn commands(&self) -> &[WireCommand] {
        &self.commands
    }
}
