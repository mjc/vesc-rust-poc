#![cfg(feature = "test-support")]
//! Integration coverage for terminal callback ownership.

use vescpkg_rs::{TerminalHandler, test_support::FirmwareTest};

struct Handler;

impl TerminalHandler for Handler {
    fn run(mut args: vescpkg_rs::TerminalArgs<'_>) {
        assert_eq!(args.next().unwrap().to_bytes(), b"one");
    }
}

#[test]
fn terminal_registration_owns_callback_until_drop() {
    let firmware = FirmwareTest::new();
    let terminal = firmware.terminal();
    let registration = terminal
        .register::<Handler>(c"sdk", c"SDK command", c"arg")
        .unwrap();
    drop(registration);
}
