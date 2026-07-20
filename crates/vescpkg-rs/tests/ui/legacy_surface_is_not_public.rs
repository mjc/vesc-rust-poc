use vescpkg_rs::{
    app_data_packet, bindings, encode_integer, ffi, PackageProgramAddress, ProtocolFrame,
    FirmwareThreadHandle, FirmwareThreadPair, FirmwareThreadPairSpec, FirmwareThreadSpec,
    ThreadGroup, ThreadHandle, WireCommand, WireVersion, PackageStart,
};

fn main() {
    let _ = app_data_packet(core::ptr::null_mut(), 0);
    let _ = encode_integer(1);
    let _ = ProtocolFrame::new(WireVersion::CURRENT, WireCommand::Ping, &[]);
    let _ = PackageProgramAddress::new(0).get();
    let _ = vescpkg_rs::LoaderInfo::as_mut_ptr;
    let _ = PackageStart::from_raw::<&mut vescpkg_rs::LoaderInfo>;
}
