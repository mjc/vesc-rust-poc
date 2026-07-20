use vescpkg_rs::{BaudRate, PacketLength};

fn requires_default<T: Default>() {}

fn main() {
    requires_default::<BaudRate>();
    requires_default::<PacketLength>();
}
