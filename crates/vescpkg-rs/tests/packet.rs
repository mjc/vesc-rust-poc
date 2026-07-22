#![cfg(feature = "test-support")]
//! Integration coverage for owned packet framing state.

#[cfg(feature = "alloc")]
use vescpkg_rs::{OwnedPacketRegistration, PackageRuntimeState, PackageStateStore};
use vescpkg_rs::{PacketCodec, PacketHandler, test_support::FirmwareTest};

struct Handler;

impl PacketHandler for Handler {
    fn send(_data: &[u8]) {}

    fn process(_data: &[u8]) {}
}

#[cfg(feature = "alloc")]
struct PackageState {
    _registration: Option<OwnedPacketRegistration<Handler>>,
}

#[cfg(feature = "alloc")]
static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

#[cfg(feature = "alloc")]
impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

#[test]
fn packet_codec_registers_processes_and_resets_owned_state() {
    let _firmware = FirmwareTest::new();
    let mut codec = PacketCodec::<Handler>::new();
    let mut registration = codec.register().unwrap();
    registration.process_byte(0x42).unwrap();
    registration.send_packet(&mut [1, 2, 3]).unwrap();
    assert_eq!(
        registration.send_packet(&mut [0; 513]),
        Err(vescpkg_rs::PacketError::PacketTooLong)
    );
    drop(registration);
}

#[test]
fn packet_codec_registration_is_exclusive_and_released_on_drop() {
    let _firmware = FirmwareTest::new();
    let mut first = PacketCodec::<Handler>::new();
    let mut second = PacketCodec::<Handler>::new();
    let registration = first.register().unwrap();
    assert!(matches!(
        second.register(),
        Err(vescpkg_rs::PacketError::Busy)
    ));
    drop(registration);
    assert!(second.register().is_ok());
}

#[test]
#[cfg(feature = "alloc")]
fn owned_packet_registration_survives_into_package_state_and_stops_cleanly() {
    let _firmware = FirmwareTest::new();
    let registration = PacketCodec::<Handler>::new()
        .register_owned()
        .expect("owned packet registration");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _registration: Some(registration),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    PacketCodec::<Handler>::new()
        .register_owned()
        .expect("stop released owned packet registration");
}
