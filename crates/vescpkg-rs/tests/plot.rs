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

#[test]
fn plot_reports_absent_optional_slots() {
    let firmware = FirmwareTest::new();
    firmware.set_plot_available(false);
    let plot = firmware.plot();

    assert_eq!(
        plot.init(c"SDK", c"demo"),
        Err(vescpkg_rs::PlotError::Unavailable)
    );
    assert_eq!(
        plot.add_graph(c"speed"),
        Err(vescpkg_rs::PlotError::Unavailable)
    );
    assert_eq!(plot.set_graph(0), Err(vescpkg_rs::PlotError::Unavailable));
    assert_eq!(
        plot.send_points(1.0, 2.0),
        Err(vescpkg_rs::PlotError::Unavailable)
    );
}
