use crate::loopback::{LoopbackTarget, LoopbackTransport, LoopbackTransportError};
use crate::package_install::PackageInstallError;
use crate::package_transport::BtlePackageInstallTransport;
use std::time::Duration;

const COMM_CUSTOM_APP_DATA: u8 = 36;
const ALLOC_SMOKE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(8);
const ALLOC_SMOKE_PING_REQUEST: &[u8] = b"alloc-ping?";
const ALLOC_SMOKE_PONG: &[u8] = b"alloc-pong";
const ALLOC_SMOKE_PROBE_REQUEST: &[u8] = b"alloc-smoke?";
const ALLOC_SMOKE_PROBE_OK: &[u8] = b"alloc-ok";
const ALLOC_SMOKE_PROBE_FAIL: &[u8] = b"alloc-fail";

/// Successful alloc-smoke app-data probe summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocSmokeProbeReport {
    target: LoopbackTarget,
    ping_response: Vec<u8>,
    response: Vec<u8>,
}

impl AllocSmokeProbeReport {
    /// Returns the BLE target used by the probe.
    pub fn target(&self) -> &LoopbackTarget {
        &self.target
    }

    /// Returns the raw app-data ping response bytes returned by the package callback.
    pub fn ping_response(&self) -> &[u8] {
        &self.ping_response
    }

    /// Returns the raw app-data response bytes returned by the package.
    pub fn response(&self) -> &[u8] {
        &self.response
    }
}

/// Runs the alloc-smoke app-data probe over BLE.
pub fn run_alloc_smoke_probe(
    target: LoopbackTarget,
) -> Result<AllocSmokeProbeReport, LoopbackTransportError> {
    let transport = BtlePackageInstallTransport::new().map_err(map_package_error)?;
    transport.open(target.clone()).map_err(map_package_error)?;
    let responses = transport.with_app_data_session(|runtime, session| {
        session.confirm_fw_version(runtime)?;
        let ping_response = exchange_custom_app_data(runtime, session, ALLOC_SMOKE_PING_REQUEST)?;
        let alloc_response = exchange_custom_app_data(runtime, session, ALLOC_SMOKE_PROBE_REQUEST)?;
        Ok((ping_response, alloc_response))
    });
    transport.close();
    let (ping_response, response) = responses.map_err(map_package_error)?;

    validate_alloc_smoke_response(target, ping_response, response)
}

/// Runs the alloc-smoke app-data probe against an arbitrary transport.
pub fn run_alloc_smoke_probe_with_transport<T: LoopbackTransport>(
    transport: &T,
    target: LoopbackTarget,
) -> Result<AllocSmokeProbeReport, LoopbackTransportError> {
    transport.open(target.clone())?;
    let ping_response = transport.exchange(ALLOC_SMOKE_PING_REQUEST)?;
    validate_alloc_smoke_ping(&ping_response)?;
    let response = transport.exchange(ALLOC_SMOKE_PROBE_REQUEST)?;

    validate_alloc_smoke_response(target, ping_response, response)
}

fn exchange_custom_app_data(
    runtime: &tokio::runtime::Runtime,
    session: &mut crate::package_transport::VescSession,
    request: &[u8],
) -> Result<Vec<u8>, PackageInstallError> {
    session.clear_packet_state();
    let packet = crate::package_transport::build_command_packet(COMM_CUSTOM_APP_DATA, request);
    runtime.block_on(crate::package_transport::write_ble_uart_packet(
        &session.peripheral,
        &session.rx_char,
        &packet,
    ))?;
    session.receive_custom_app_data(ALLOC_SMOKE_RESPONSE_TIMEOUT)
}

fn validate_alloc_smoke_response(
    target: LoopbackTarget,
    ping_response: Vec<u8>,
    response: Vec<u8>,
) -> Result<AllocSmokeProbeReport, LoopbackTransportError> {
    validate_alloc_smoke_ping(&ping_response)?;
    if response == ALLOC_SMOKE_PROBE_FAIL {
        return Err(LoopbackTransportError::Device(
            "alloc-smoke Rust allocation failed".to_owned(),
        ));
    }
    if response != ALLOC_SMOKE_PROBE_OK {
        return Err(LoopbackTransportError::Device(format!(
            "alloc-smoke allocation probe failed: expected {ALLOC_SMOKE_PROBE_OK:?}, got {response:?}"
        )));
    }

    Ok(AllocSmokeProbeReport {
        target,
        ping_response,
        response,
    })
}

fn validate_alloc_smoke_ping(response: &[u8]) -> Result<(), LoopbackTransportError> {
    if response != ALLOC_SMOKE_PONG {
        return Err(LoopbackTransportError::Device(format!(
            "alloc-smoke handler ping failed: expected {ALLOC_SMOKE_PONG:?}, got {response:?}"
        )));
    }

    Ok(())
}

fn map_package_error(error: PackageInstallError) -> LoopbackTransportError {
    LoopbackTransportError::Device(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        ALLOC_SMOKE_PING_REQUEST, ALLOC_SMOKE_PONG, ALLOC_SMOKE_PROBE_OK, ALLOC_SMOKE_PROBE_REQUEST,
    };
    use crate::alloc_smoke_probe::run_alloc_smoke_probe_with_transport;
    use crate::loopback::{FakeLoopbackTransport, LoopbackTarget, LoopbackTransportError};

    #[test]
    fn alloc_smoke_probe_sends_request_and_accepts_ok_response() {
        let transport = FakeLoopbackTransport::new();
        transport.queue_response(Ok(ALLOC_SMOKE_PONG.to_vec()));
        transport.queue_response(Ok(ALLOC_SMOKE_PROBE_OK.to_vec()));
        let target = LoopbackTarget::named("VESC BLE UART");

        let report = run_alloc_smoke_probe_with_transport(&transport, target.clone())
            .expect("alloc smoke probe");

        assert_eq!(transport.open_target(), Some(target.clone()));
        assert_eq!(
            transport.requests(),
            vec![
                ALLOC_SMOKE_PING_REQUEST.to_vec(),
                ALLOC_SMOKE_PROBE_REQUEST.to_vec()
            ]
        );
        assert_eq!(report.target(), &target);
        assert_eq!(report.ping_response(), ALLOC_SMOKE_PONG);
        assert_eq!(report.response(), ALLOC_SMOKE_PROBE_OK);
    }

    #[test]
    fn alloc_smoke_probe_rejects_unexpected_ping_response() {
        let transport = FakeLoopbackTransport::new();
        transport.queue_response(Ok(b"not-pong".to_vec()));

        let error = run_alloc_smoke_probe_with_transport(&transport, LoopbackTarget::default())
            .expect_err("unexpected ping response");

        assert_eq!(
            error,
            LoopbackTransportError::Device(
                "alloc-smoke handler ping failed: expected [97, 108, 108, 111, 99, 45, 112, 111, 110, 103], got [110, 111, 116, 45, 112, 111, 110, 103]"
                    .to_owned()
            )
        );
    }

    #[test]
    fn alloc_smoke_probe_rejects_unexpected_alloc_response() {
        let transport = FakeLoopbackTransport::new();
        transport.queue_response(Ok(ALLOC_SMOKE_PONG.to_vec()));
        transport.queue_response(Ok(b"alloc-fail".to_vec()));

        let error = run_alloc_smoke_probe_with_transport(&transport, LoopbackTarget::default())
            .expect_err("unexpected response");

        assert_eq!(
            error,
            LoopbackTransportError::Device("alloc-smoke Rust allocation failed".to_owned())
        );
    }

    #[test]
    fn alloc_smoke_probe_rejects_unknown_alloc_response() {
        let transport = FakeLoopbackTransport::new();
        transport.queue_response(Ok(ALLOC_SMOKE_PONG.to_vec()));
        transport.queue_response(Ok(b"wat".to_vec()));

        let error = run_alloc_smoke_probe_with_transport(&transport, LoopbackTarget::default())
            .expect_err("unknown response");

        assert_eq!(
            error,
            LoopbackTransportError::Device(
                "alloc-smoke allocation probe failed: expected [97, 108, 108, 111, 99, 45, 111, 107], got [119, 97, 116]"
                    .to_owned()
            )
        );
    }
}
