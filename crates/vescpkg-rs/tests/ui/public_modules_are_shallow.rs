fn takes_config(_: vescpkg_rs::types::CustomConfigImage<2>) {}

fn main() {
    takes_config(vescpkg_rs::types::CustomConfigImage::new([0, 0]));
    let _: Option<vescpkg_rs::units::Rpm> = None;
}
