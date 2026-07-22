#![cfg(feature = "test-support")]
//! Integration coverage for terminal callback ownership.

use vescpkg_rs::{
    PackageRuntimeState, PackageStateStore, TerminalError, TerminalHandler, TerminalRegistration,
    test_support::FirmwareTest,
};

struct Handler;

impl TerminalHandler for Handler {
    fn run(mut args: vescpkg_rs::TerminalArgs<'_>) {
        assert_eq!(args.next().unwrap().to_bytes(), b"one");
    }
}

struct PackageState {
    _registration: Option<TerminalRegistration<'static, Handler>>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

#[test]
fn terminal_registration_owns_callback_until_drop() {
    let firmware = FirmwareTest::new();
    let terminal = firmware.terminal();
    let registration = terminal
        .register::<Handler>(c"sdk", c"SDK command", c"arg")
        .unwrap();
    assert!(matches!(
        terminal.register::<Handler>(c"other", c"Other command", c"arg"),
        Err(TerminalError::Busy)
    ));
    drop(registration);
}

#[test]
fn terminal_registration_reports_absent_optional_slots() {
    let firmware = FirmwareTest::new();
    firmware.set_terminal_available(false);

    assert!(matches!(
        firmware
            .terminal()
            .register::<Handler>(c"sdk", c"SDK command", c"arg"),
        Err(TerminalError::Unavailable)
    ));
}

#[test]
fn package_stop_releases_terminal_state_before_next_registration() {
    let firmware = FirmwareTest::new();
    let terminal: &'static _ = Box::leak(Box::new(firmware.terminal()));
    let registration = terminal
        .register::<Handler>(c"sdk", c"SDK command", c"arg")
        .expect("terminal callback");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _registration: Some(registration),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    let terminal: &'static _ = Box::leak(Box::new(firmware.terminal()));
    terminal
        .register::<Handler>(c"sdk", c"SDK command", c"arg")
        .expect("stop released terminal callback");
}
