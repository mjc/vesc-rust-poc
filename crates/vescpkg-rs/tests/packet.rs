#![cfg(feature = "test-support")]
//! Integration coverage for owned packet framing state.

use vescpkg_rs::{PacketCodec, PacketHandler, test_support::FirmwareTest};

struct Handler;

impl PacketHandler for Handler {
    fn send(_data: &[u8]) {}

    fn process(_data: &[u8]) {}
}

#[test]
fn packet_codec_registers_processes_and_resets_owned_state() {
    let _firmware = FirmwareTest::new();
    let mut codec = PacketCodec::<Handler>::new();
    let mut registration = codec.register().unwrap();
    registration.process_byte(0x42).unwrap();
    registration.send_packet(&mut [1, 2, 3]).unwrap();
    drop(registration);
}
