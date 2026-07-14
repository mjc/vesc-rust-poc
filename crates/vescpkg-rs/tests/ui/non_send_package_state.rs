use std::rc::Rc;

static STATE: vescpkg_rs::PackageStateStore<Rc<()>> = vescpkg_rs::PackageStateStore::new();

fn main() {
    let _ = &STATE;
}
