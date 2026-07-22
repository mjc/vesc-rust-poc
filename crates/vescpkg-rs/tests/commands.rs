#![cfg(feature = "test-support")]
//! Integration coverage for command reply ownership.

use vescpkg_rs::{CommandError, CommandReplyHandler, test_support::FirmwareTest};

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

#[test]
fn command_reply_registration_is_exclusive() {
    let firmware = FirmwareTest::new();
    let commands = firmware.commands();
    let mut first_packet = [1_u8];
    let first = commands.process::<Handler>(&mut first_packet).unwrap();
    let mut second_packet = [2_u8];
    assert!(matches!(
        commands.process::<Handler>(&mut second_packet),
        Err(CommandError::Busy)
    ));
    drop(first);
    assert!(commands.process::<Handler>(&mut second_packet).is_ok());
}
