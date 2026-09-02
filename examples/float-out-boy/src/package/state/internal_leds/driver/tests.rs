use core::cell::Cell;

use crate::{
    FloatOutBoyFootpadState, FloatOutBoyMode, FloatOutBoyRunState,
    lcm::{FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode},
    leds::{
        FloatOutBoyLedAnimationMode, FloatOutBoyLedAnimationSpeed, FloatOutBoyLedBarConfig,
        FloatOutBoyLedColor, FloatOutBoyLedColorOrder, FloatOutBoyLedFrameUpdate,
        FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedRenderer,
        FloatOutBoyLedStatusUpdate, FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
        FloatOutBoyLedUpdate, FloatOutBoyLedsConfig, FloatOutBoyStatusBarConfig,
        FloatOutBoyStatusBarIdleTimeout,
    },
};
use vescpkg_rs::Ratio;

use super::{FloatOutBoyInternalLedDriver, WS2812_ONE, WS2812_RESET, WS2812_ZERO, encode_byte};

fn solid_bar(color: FloatOutBoyLedColor) -> FloatOutBoyLedBarConfig {
    FloatOutBoyLedBarConfig::new(
        Ratio::from_ratio_const(1.0),
        color,
        FloatOutBoyLedColor::Black,
        FloatOutBoyLedAnimationMode::Solid,
        FloatOutBoyLedAnimationSpeed::from_units(1.0),
    )
}

fn enabled_config() -> FloatOutBoyLedsConfig {
    let black = solid_bar(FloatOutBoyLedColor::Black);
    FloatOutBoyLedsConfig::new(
        black,
        black,
        solid_bar(FloatOutBoyLedColor::Red),
        black,
        FloatOutBoyStatusBarConfig::new(
            FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.5),
            Ratio::from_ratio_const(0.2),
            Ratio::from_ratio_const(0.2),
            Ratio::from_ratio_const(0.5),
        ),
        black,
    )
    .enabled()
}

#[test]
fn internal_layout_matches_production_order_offsets_and_empty_strips() {
    let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            2,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_front_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Third,
            3,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Second,
            5,
            FloatOutBoyLedColorOrder::Grb,
        ));
    let layout = hardware.internal_layout().unwrap();

    assert_eq!(
        layout.roles(),
        &[
            crate::lcm::FloatOutBoyLedStripRole::Status,
            crate::lcm::FloatOutBoyLedStripRole::Rear,
            crate::lcm::FloatOutBoyLedStripRole::Front,
        ]
    );
    assert_eq!(
        layout.offset(crate::lcm::FloatOutBoyLedStripRole::Status),
        Some(0)
    );
    assert_eq!(
        layout.offset(crate::lcm::FloatOutBoyLedStripRole::Rear),
        Some(2)
    );
    assert_eq!(
        layout.offset(crate::lcm::FloatOutBoyLedStripRole::Front),
        Some(7)
    );
    assert_eq!(layout.pixel_count(), 10);

    let empty = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            u8::MAX,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_front_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            u8::MAX,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            u8::MAX,
            FloatOutBoyLedColorOrder::Grb,
        ));
    let layout = empty.internal_layout().unwrap();
    assert!(layout.roles().is_empty());
    assert_eq!(layout.pixel_count(), 0);
}

#[test]
fn internal_layout_rejects_front_and_rear_over_60_pixels() {
    let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_front_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            30,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::Second,
            31,
            FloatOutBoyLedColorOrder::Grb,
        ));

    assert!(hardware.internal_layout().is_err());
}

#[test]
fn setup_builds_source_sized_zero_pulse_buffer() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        2,
        FloatOutBoyLedColorOrder::Grbw,
    );
    let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_pin(FloatOutBoyLedPin::C9)
        .with_pin_config(FloatOutBoyLedPinConfig::NoPullup)
        .with_front_strip(strip)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ));
    let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
    let mut setup = None;

    assert!(driver.setup(
        |pin, pin_config, pulses| {
            setup = Some((pin, pin_config, pulses.len()));
            true
        },
        |_| panic!("successful setup rolled back"),
    ));

    assert_eq!(
        setup,
        Some((FloatOutBoyLedPin::C9, FloatOutBoyLedPinConfig::NoPullup, 65,))
    );
    assert_eq!(driver.pulses()[..64], [WS2812_ZERO; 64]);
    assert_eq!(driver.pulses()[64], WS2812_RESET);
    assert!(driver.is_operational());
}

#[test]
fn every_byte_maps_to_exact_ws2812_pulses() {
    let masks = [0x80_u8, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

    for byte in 0_u8..=u8::MAX {
        let mut pulses = [WS2812_RESET; 8];
        let mut index = 0;
        assert!(encode_byte(&mut pulses, &mut index, byte));
        assert_eq!(index, 8);
        assert_eq!(
            pulses,
            masks.map(|mask| {
                if byte & mask == 0 {
                    WS2812_ZERO
                } else {
                    WS2812_ONE
                }
            }),
            "wrong pulses for byte {byte:#04x}"
        );
    }

    let mut short = [WS2812_RESET; 7];
    let mut index = 0;
    assert!(!encode_byte(&mut short, &mut index, u8::MAX));
    assert_eq!(index, short.len());
    assert_eq!(short, [WS2812_ONE; 7]);
}

#[test]
fn setup_rejects_zero_and_out_of_range_strip_counts_without_touching_hardware() {
    let zero = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_front_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ));
    let too_many = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::First,
            31,
            FloatOutBoyLedColorOrder::Grb,
        ));

    for hardware in [zero, too_many] {
        let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
        assert!(!driver.setup(
            |_, _, _| panic!("invalid layout reached hardware"),
            |_| panic!("hardware teardown ran without setup"),
        ));
        assert!(!driver.is_operational());
    }
}

#[test]
fn paint_encodes_gamma_ordered_renderer_pixels_and_restarts_once() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_front_strip(strip)
        .with_status_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ))
        .with_rear_strip(FloatOutBoyLedStripConfig::new(
            FloatOutBoyLedStripOrder::None,
            0,
            FloatOutBoyLedColorOrder::Grb,
        ));
    let config = enabled_config();
    let mut renderer = FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    let frame = FloatOutBoyLedFrameUpdate::new(
        FloatOutBoyLedUpdate {
            run_state: FloatOutBoyRunState::Ready,
            mode: FloatOutBoyMode::Normal,
            darkride: false,
            footpad: FloatOutBoyFootpadState::None,
            pitch_degrees: 0.0,
            distance: 0.0,
        },
        FloatOutBoyLedStatusUpdate {
            battery_level: 1.0,
            duty_cycle: 0.0,
            motor_current_saturation: 0.0,
            battery_current_saturation: 0.0,
            moving: false,
        },
    );
    for _ in 0..10 {
        renderer.update(config, frame, 0.0);
    }
    let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
    assert!(driver.setup(|_, _, _| true, |_| panic!("successful setup rolled back")));
    let mut restarted = 0;

    assert!(driver.paint(
        &renderer,
        |_| true,
        |_, pulses| {
            restarted += 1;
            assert_eq!(pulses[..8], [WS2812_ZERO; 8]);
            assert_eq!(pulses[8..16], [WS2812_ONE; 8]);
            assert_eq!(pulses[16..24], [WS2812_ZERO; 8]);
            assert_eq!(pulses[24], WS2812_RESET);
            true
        }
    ));

    assert_eq!(restarted, 1);
}

#[test]
fn failed_setup_and_successful_destroy_teardown_exactly_once() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal).with_front_strip(strip);
    let mut failed = FloatOutBoyInternalLedDriver::new(hardware);
    let mut failed_teardowns = 0;

    assert!(!failed.setup(|_, _, _| false, |_| failed_teardowns += 1,));
    assert!(failed.destroy(|_| {
        failed_teardowns += 1;
        true
    }));
    assert_eq!(failed_teardowns, 1);

    let mut active = FloatOutBoyInternalLedDriver::new(hardware);
    assert!(active.setup(|_, _, _| true, |_| {}));
    let mut active_teardowns = 0;
    assert!(active.destroy(|_| {
        active_teardowns += 1;
        true
    }));
    assert!(active.destroy(|_| {
        active_teardowns += 1;
        true
    }));
    assert_eq!(active_teardowns, 1);
    assert!(!active.is_operational());
}

#[test]
fn failed_destroy_retains_dma_storage_and_can_retry() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal).with_front_strip(strip);
    let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
    assert!(driver.setup(|_, _, _| true, |_| {}));
    let pulse_count = driver.pulses().len();
    let mut teardowns = 0;

    assert!(!driver.destroy(|_| {
        teardowns += 1;
        false
    }));
    assert!(!driver.is_operational());
    assert!(pulse_count > 0);
    assert_eq!(driver.pulses().len(), pulse_count);
    assert!(driver.destroy(|_| {
        teardowns += 1;
        true
    }));
    assert_eq!(teardowns, 2);
    assert!(driver.pulses().is_empty());
}

#[test]
fn paint_quiesces_before_mutating_pulses_and_faults_still_teardown() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal).with_front_strip(strip);
    let config = enabled_config();
    let renderer = FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
    assert!(driver.setup(|_, _, _| true, |_| {}));
    let quiesced = Cell::new(false);

    assert!(!driver.paint(
        &renderer,
        |_| {
            quiesced.set(true);
            false
        },
        |_, _| panic!("failed quiesce restarted DMA")
    ));
    assert!(quiesced.get());
    assert!(!driver.is_operational());

    let mut teardowns = 0;
    assert!(driver.destroy(|_| {
        teardowns += 1;
        true
    }));
    assert!(driver.destroy(|_| {
        teardowns += 1;
        true
    }));
    assert_eq!(teardowns, 1);
}

#[test]
fn failed_restart_faults_driver_and_blocks_further_paints_until_destroy() {
    let strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal).with_front_strip(strip);
    let renderer = FloatOutBoyLedRenderer::new(hardware, enabled_config(), 0.0);
    let mut driver = FloatOutBoyInternalLedDriver::new(hardware);
    assert!(driver.setup(|_, _, _| true, |_| {}));

    assert!(!driver.paint(&renderer, |_| true, |_, _| false));
    assert!(!driver.paint(
        &renderer,
        |_| panic!("faulted driver quiesced hardware again"),
        |_, _| panic!("faulted driver restarted DMA again"),
    ));
    assert!(driver.destroy(|_| true));
}
