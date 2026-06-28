//! Hardware-in-the-loop sketch. Requires `VESC_DEVICE` and `VESC_BLE_ADDR`.
//!
//! Run explicitly with:
//! `cargo nextest run -p vesc-host-cli --profile hil -- --ignored`

#[test]
#[ignore = "requires VESC hardware; run via nextest hil profile"]
fn hil_loopback_sketch() {
    let _device = std::env::var("VESC_DEVICE").expect("VESC_DEVICE for HIL loopback");
    let _addr = std::env::var("VESC_BLE_ADDR").expect("VESC_BLE_ADDR for HIL loopback");
}
