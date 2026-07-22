//! Hardware-in-the-loop sketch. Requires `VESC_DEVICE` and `VESC_BLE_ADDR`.
//!
//! Run explicitly with:
//! `cargo nextest run -p cargo-vescpkg --features hil --profile hil -- --ignored`

#![cfg(feature = "hil")]

#[test]
#[ignore = "requires VESC hardware; run via nextest hil profile"]
fn hil_loopback_sketch() {
    let device = std::env::var("VESC_DEVICE").expect("VESC_DEVICE for HIL control-loop probe");
    let address = std::env::var("VESC_BLE_ADDR").expect("VESC_BLE_ADDR for HIL control-loop probe");
    let report = cargo_vescpkg::deploy::run_control_loop_probe(
        cargo_vescpkg::loopback::LoopbackTarget::addressed(address),
        |event| eprintln!("{device}: {event}"),
    )
    .expect("control-loop probe");

    assert!(report.statuses().len() >= 2);
    assert!(
        report
            .statuses()
            .windows(2)
            .any(|samples| samples[0].tick_count() < samples[1].tick_count())
    );
    assert!(
        report
            .statuses()
            .windows(2)
            .any(|samples| samples[0].output() != samples[1].output())
    );
    eprintln!(
        "{device}: control-loop timing {:?}..{:?}, jitter {:?}",
        report.timing().min_tick_period(),
        report.timing().max_tick_period(),
        report.timing().jitter()
    );
}
