struct State;
struct Callback;

impl vescpkg_rs::AppDataHandler for Callback {
    type State = State;

    fn handle(_state: &mut Self::State, _packet: vescpkg_rs::AppDataPacket<'_>) {}
}

fn main() {}
