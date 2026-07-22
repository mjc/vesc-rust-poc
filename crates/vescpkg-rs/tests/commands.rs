#![cfg(feature = "test-support")]
//! Integration coverage for command reply ownership.

use vescpkg_rs::{
    CommandError, CommandReplyHandler, CommandReplyLease, PackageRuntimeState, PackageStateStore,
    test_support::FirmwareTest,
};

struct Handler;

impl CommandReplyHandler for Handler {
    fn reply(_data: &[u8]) {}
}

struct PackageState {
    _lease: Option<CommandReplyLease<Handler>>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
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

#[test]
fn package_stop_releases_command_reply_state_before_next_registration() {
    let firmware = FirmwareTest::new();
    let mut packet = [1_u8];
    let lease = firmware
        .commands()
        .process::<Handler>(&mut packet)
        .expect("command reply callback");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _lease: Some(lease),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    let mut packet = [2_u8];
    firmware
        .commands()
        .process::<Handler>(&mut packet)
        .expect("stop released command reply callback");
}
