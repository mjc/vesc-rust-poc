#![cfg(feature = "test-support")]
//! Integration coverage for command reply ownership.

use vescpkg_rs::{CommandReplyHandler, test_support::FirmwareTest};

struct Handler;

impl CommandReplyHandler for Handler {
    fn reply(_data: &[u8]) {}
}

#[test]
fn command_reply_lease_processes_packet_and_unregisters_on_drop() {
    let firmware = FirmwareTest::new();
    let commands = firmware.commands();
    let mut packet = [1_u8, 2, 3];
    let lease = commands.process::<Handler>(&mut packet).unwrap();
    drop(lease);
}
