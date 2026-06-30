//! Integration tests for the fake BLE loopback bridge.

use std::cell::RefCell;

use vesc_cli::loopback::{LoopbackTarget, LoopbackTransport, LoopbackTransportError, run_loopback};
use vesc_protocol::WireCommand;
use vesc_protocol::ble_loopback::LoopbackPacket;
use vescpkg_build::{
    FakeFirmwareServices, LoopbackPackageRuntime, LoopbackPackageState, LoopbackRuntimeError,
    LoopbackStartError, LoopbackTick,
};

struct BridgeTransport<'a> {
    services: &'a FakeFirmwareServices,
    runtime: RefCell<LoopbackPackageRuntime<&'a FakeFirmwareServices>>,
    requests: RefCell<Vec<Vec<u8>>>,
}

impl<'a> BridgeTransport<'a> {
    fn new(services: &'a FakeFirmwareServices) -> Self {
        services.set_ble_connected(true);

        Self {
            services,
            runtime: RefCell::new(LoopbackPackageRuntime::new(services)),
            requests: RefCell::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<Vec<u8>> {
        self.requests.borrow().clone()
    }

    fn start_if_needed(&self) -> Result<(), LoopbackTransportError> {
        let mut runtime = self.runtime.borrow_mut();
        if matches!(runtime.state(), LoopbackPackageState::Booting) {
            runtime.start().map_err(map_start_error)?;
        }

        Ok(())
    }
}

impl<'a> LoopbackTransport for BridgeTransport<'a> {
    fn open(&self, target: LoopbackTarget) -> Result<(), LoopbackTransportError> {
        assert_eq!(target, LoopbackTarget::default());
        self.start_if_needed()
    }

    fn exchange(&self, request: &[u8]) -> Result<Vec<u8>, LoopbackTransportError> {
        self.requests.borrow_mut().push(request.to_vec());
        self.services.queue_ble_frame(request.to_vec());

        let tick = { self.runtime.borrow_mut().tick() };
        match tick {
            Ok(LoopbackTick::Replied(_)) => {
                let transmitted = self.services.transmitted_frames();
                transmitted
                    .last()
                    .cloned()
                    .ok_or(LoopbackTransportError::Device(
                        "device produced no reply".to_owned(),
                    ))
            }
            Ok(LoopbackTick::Idle) | Ok(LoopbackTick::WaitingForConnection) => Err(
                LoopbackTransportError::Device("device produced no reply".to_owned()),
            ),
            Err(error) => Err(map_runtime_error(error)),
        }
    }
}

fn map_start_error(error: LoopbackStartError) -> LoopbackTransportError {
    match error {
        LoopbackStartError::InitFailed(reason) => LoopbackTransportError::Device(reason.to_owned()),
    }
}

fn map_runtime_error(error: LoopbackRuntimeError) -> LoopbackTransportError {
    match error {
        LoopbackRuntimeError::Start(start) => map_start_error(start),
        LoopbackRuntimeError::Frame(frame) => LoopbackTransportError::Protocol(frame),
    }
}

fn decode_commands(frames: &[Vec<u8>]) -> Vec<WireCommand> {
    frames
        .iter()
        .map(|frame| {
            LoopbackPacket::decode(frame)
                .expect("loopback frame")
                .frame()
                .command()
        })
        .collect()
}

#[test]
fn fake_ble_loopback_bridge_behavior() {
    {
        let services = FakeFirmwareServices::new();
        services.set_now_ms(0x0102_0304_0506_0708);
        let bridge = BridgeTransport::new(&services);

        let report = run_loopback(&bridge).expect("loopback report");

        assert_eq!(
            report.commands(),
            &[
                WireCommand::Ping,
                WireCommand::Echo,
                WireCommand::Status,
                WireCommand::Teardown,
            ]
        );

        let requests = bridge.requests();
        assert_eq!(
            decode_commands(&requests),
            [
                WireCommand::Ping,
                WireCommand::Echo,
                WireCommand::Status,
                WireCommand::Teardown,
            ]
        );
        assert_eq!(
            decode_commands(&services.transmitted_frames()),
            report.commands()
        );

        let transmitted = services.transmitted_frames();
        let status = LoopbackPacket::decode(&transmitted[2]).expect("decoded status reply");
        assert_eq!(status.frame().command(), WireCommand::Status);
        assert_eq!(
            status.frame().payload(),
            &0x0102_0304_0506_0708_u64.to_le_bytes()
        );

        let teardown = LoopbackPacket::decode(&transmitted[3]).expect("decoded teardown reply");
        assert_eq!(teardown.frame().command(), WireCommand::Teardown);
        assert!(teardown.frame().payload().is_empty());

        assert!(
            services
                .logs()
                .iter()
                .any(|line| line == "BLE loopback ready")
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

    {
        let services = FakeFirmwareServices::new();
        services.set_ble_init_error(Some("BLE init failed"));
        let bridge = BridgeTransport::new(&services);

        assert_eq!(
            run_loopback(&bridge),
            Err(LoopbackTransportError::Device("BLE init failed".to_owned()))
        );
    }
}
