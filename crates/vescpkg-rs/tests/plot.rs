#![cfg(feature = "test-support")]
//! Integration coverage for plotting capability.

use vescpkg_rs::test_support::FirmwareTest;

#[test]
fn plot_forwards_named_graphs_and_checked_points() {
    let firmware = FirmwareTest::new();
    let plot = firmware.plot();
    plot.init(c"SDK", c"demo").unwrap();
    plot.add_graph(c"speed").unwrap();
    plot.set_graph(0).unwrap();
    plot.send_points(1.0, 2.0).unwrap();
    assert!(plot.send_points(f32::NAN, 2.0).is_err());
    assert!(plot.set_graph(-1).is_err());
}
