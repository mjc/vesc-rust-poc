use super::{FloatOutBoyLedColor, FloatOutBoyLedColorOrder, FloatOutBoyLedPixel};
use vescpkg_rs::prelude::Ratio;

#[must_use]
fn solid_bar(primary: FloatOutBoyLedColor) -> super::FloatOutBoyLedBarConfig {
    full_brightness_bar(
        primary,
        FloatOutBoyLedColor::Black,
        super::FloatOutBoyLedAnimationMode::Solid,
    )
}

#[must_use]
fn solid_bar_with_brightness(
    brightness: Ratio,
    primary: FloatOutBoyLedColor,
) -> super::FloatOutBoyLedBarConfig {
    led_bar(
        brightness,
        primary,
        FloatOutBoyLedColor::Black,
        super::FloatOutBoyLedAnimationMode::Solid,
    )
}

#[must_use]
fn led_bar(
    brightness: Ratio,
    primary: FloatOutBoyLedColor,
    secondary: FloatOutBoyLedColor,
    mode: super::FloatOutBoyLedAnimationMode,
) -> super::FloatOutBoyLedBarConfig {
    super::FloatOutBoyLedBarConfig::new(
        brightness,
        primary,
        secondary,
        mode,
        super::FloatOutBoyLedAnimationSpeed::from_units(1.0),
    )
}

#[must_use]
fn full_brightness_bar(
    primary: FloatOutBoyLedColor,
    secondary: FloatOutBoyLedColor,
    mode: super::FloatOutBoyLedAnimationMode,
) -> super::FloatOutBoyLedBarConfig {
    led_bar(Ratio::from_ratio_const(1.0), primary, secondary, mode)
}

#[must_use]
fn strip_channels<const N: usize>(frame: &super::FloatOutBoyLedStripFrame) -> [[u8; 4]; N] {
    core::array::from_fn(|index| {
        frame
            .physical_pixel(index)
            .map(super::FloatOutBoyLedPixel::channels)
            .unwrap_or_default()
    })
}

#[must_use]
fn white_led_config(
    idle_timeout: super::FloatOutBoyStatusBarIdleTimeout,
) -> super::FloatOutBoyLedsConfig {
    let bar = solid_bar(FloatOutBoyLedColor::WhiteRgb);
    let status = super::FloatOutBoyStatusBarConfig::new(
        idle_timeout,
        Ratio::from_ratio_const(0.9),
        Ratio::from_ratio_const(0.1),
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(0.5),
    );
    super::FloatOutBoyLedsConfig::new(bar, bar, bar, bar, status, bar).enabled()
}

#[must_use]
fn ride_only(ride: super::FloatOutBoyLedUpdate) -> super::FloatOutBoyLedFrameUpdate {
    super::FloatOutBoyLedFrameUpdate::new(
        ride,
        super::FloatOutBoyLedStatusUpdate {
            battery_level: 0.0,
            duty_cycle: 0.0,
            moving: true,
        },
    )
}

#[test]
fn named_led_colors_match_refloat_1_2_1_rgba_channels() {
    let cases = [
        (FloatOutBoyLedColor::Black, [0x00, 0x00, 0x00, 0x00]),
        (FloatOutBoyLedColor::WhiteFull, [0xff, 0xff, 0xff, 0xff]),
        (FloatOutBoyLedColor::WhiteRgb, [0xff, 0xff, 0xff, 0x00]),
        (FloatOutBoyLedColor::WhiteSingle, [0x00, 0x00, 0x00, 0xff]),
        (FloatOutBoyLedColor::Red, [0xff, 0x00, 0x00, 0x00]),
        (FloatOutBoyLedColor::Ferrari, [0xff, 0x38, 0x00, 0x00]),
        (FloatOutBoyLedColor::Flame, [0xff, 0x50, 0x00, 0x00]),
        (FloatOutBoyLedColor::Coral, [0xff, 0x60, 0x40, 0x00]),
        (FloatOutBoyLedColor::Sunset, [0xff, 0x78, 0x30, 0x00]),
        (FloatOutBoyLedColor::Sunrise, [0xff, 0x90, 0x40, 0x00]),
        (FloatOutBoyLedColor::Gold, [0xff, 0x80, 0x20, 0x00]),
        (FloatOutBoyLedColor::Orange, [0xff, 0x78, 0x00, 0x00]),
        (FloatOutBoyLedColor::Yellow, [0xff, 0xa0, 0x00, 0x00]),
        (FloatOutBoyLedColor::Banana, [0xff, 0xb0, 0x40, 0x00]),
        (FloatOutBoyLedColor::Lime, [0xff, 0xff, 0x00, 0x00]),
        (FloatOutBoyLedColor::Acid, [0xa0, 0xff, 0x00, 0x00]),
        (FloatOutBoyLedColor::Sage, [0xa0, 0xff, 0x50, 0x00]),
        (FloatOutBoyLedColor::Green, [0x00, 0xff, 0x00, 0x00]),
        (FloatOutBoyLedColor::Mint, [0x00, 0xff, 0x50, 0x00]),
        (FloatOutBoyLedColor::Tiffany, [0x00, 0xff, 0xc0, 0x00]),
        (FloatOutBoyLedColor::Cyan, [0x00, 0xff, 0xff, 0x00]),
        (FloatOutBoyLedColor::Steel, [0x90, 0xc0, 0xff, 0x00]),
        (FloatOutBoyLedColor::Sky, [0x70, 0xd0, 0xff, 0x00]),
        (FloatOutBoyLedColor::Azure, [0x00, 0xa0, 0xff, 0x00]),
        (FloatOutBoyLedColor::Sapphire, [0x00, 0x70, 0xff, 0x00]),
        (FloatOutBoyLedColor::Blue, [0x00, 0x00, 0xff, 0x00]),
        (FloatOutBoyLedColor::Violet, [0x80, 0x00, 0xff, 0x00]),
        (FloatOutBoyLedColor::Amethyst, [0xa0, 0x60, 0xff, 0x00]),
        (FloatOutBoyLedColor::Magenta, [0xff, 0x00, 0xff, 0x00]),
        (FloatOutBoyLedColor::Pink, [0xff, 0x00, 0xc0, 0x00]),
        (FloatOutBoyLedColor::Fuchsia, [0xff, 0x00, 0x70, 0x00]),
        (FloatOutBoyLedColor::Lavender, [0xff, 0x70, 0xa0, 0x00]),
    ];

    for (color, channels) in cases {
        assert_eq!(FloatOutBoyLedPixel::from_named(color).channels(), channels);
    }
}

#[test]
fn led_config_enum_ids_match_refloat_1_2_1_settings() {
    assert_eq!(
        [
            super::FloatOutBoyLedPin::B6.id(),
            super::FloatOutBoyLedPin::B7.id(),
            super::FloatOutBoyLedPin::C9.id(),
        ],
        [0, 1, 2]
    );
    assert_eq!(
        [
            super::FloatOutBoyLedPinConfig::PullupTo5v.id(),
            super::FloatOutBoyLedPinConfig::NoPullup.id(),
        ],
        [0, 1]
    );
    assert_eq!(
        [
            FloatOutBoyLedColorOrder::Grb.id(),
            FloatOutBoyLedColorOrder::Grbw.id(),
            FloatOutBoyLedColorOrder::Rgb.id(),
            FloatOutBoyLedColorOrder::Wrgb.id(),
        ],
        [0, 1, 2, 3]
    );
    assert_eq!(
        [
            super::FloatOutBoyLedAnimationMode::Solid.id(),
            super::FloatOutBoyLedAnimationMode::Fade.id(),
            super::FloatOutBoyLedAnimationMode::Pulse.id(),
            super::FloatOutBoyLedAnimationMode::Strobe.id(),
            super::FloatOutBoyLedAnimationMode::KnightRider.id(),
            super::FloatOutBoyLedAnimationMode::Felony.id(),
            super::FloatOutBoyLedAnimationMode::RainbowCycle.id(),
            super::FloatOutBoyLedAnimationMode::RainbowFade.id(),
            super::FloatOutBoyLedAnimationMode::RainbowRoll.id(),
        ],
        [0, 1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_eq!(
        [
            super::FloatOutBoyLedTransition::Fade.id(),
            super::FloatOutBoyLedTransition::FadeOutIn.id(),
            super::FloatOutBoyLedTransition::Cipher.id(),
            super::FloatOutBoyLedTransition::MonoCipher.id(),
        ],
        [0, 1, 2, 3]
    );
    assert_eq!(
        [
            super::FloatOutBoyLedStripOrder::None.id(),
            super::FloatOutBoyLedStripOrder::First.id(),
            super::FloatOutBoyLedStripOrder::Second.id(),
            super::FloatOutBoyLedStripOrder::Third.id(),
        ],
        [0, 1, 2, 3]
    );

    let colors = [
        FloatOutBoyLedColor::Black,
        FloatOutBoyLedColor::WhiteFull,
        FloatOutBoyLedColor::WhiteRgb,
        FloatOutBoyLedColor::WhiteSingle,
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Ferrari,
        FloatOutBoyLedColor::Flame,
        FloatOutBoyLedColor::Coral,
        FloatOutBoyLedColor::Sunset,
        FloatOutBoyLedColor::Sunrise,
        FloatOutBoyLedColor::Gold,
        FloatOutBoyLedColor::Orange,
        FloatOutBoyLedColor::Yellow,
        FloatOutBoyLedColor::Banana,
        FloatOutBoyLedColor::Lime,
        FloatOutBoyLedColor::Acid,
        FloatOutBoyLedColor::Sage,
        FloatOutBoyLedColor::Green,
        FloatOutBoyLedColor::Mint,
        FloatOutBoyLedColor::Tiffany,
        FloatOutBoyLedColor::Cyan,
        FloatOutBoyLedColor::Steel,
        FloatOutBoyLedColor::Sky,
        FloatOutBoyLedColor::Azure,
        FloatOutBoyLedColor::Sapphire,
        FloatOutBoyLedColor::Blue,
        FloatOutBoyLedColor::Violet,
        FloatOutBoyLedColor::Amethyst,
        FloatOutBoyLedColor::Magenta,
        FloatOutBoyLedColor::Pink,
        FloatOutBoyLedColor::Fuchsia,
        FloatOutBoyLedColor::Lavender,
    ];
    for (expected, color) in (0_u8..).zip(colors) {
        assert_eq!(color.id(), expected, "wrong ID for {color:?}");
    }
}

fn physical_channels(
    pixel: FloatOutBoyLedPixel,
    order: FloatOutBoyLedColorOrder,
) -> std::vec::Vec<u8> {
    let (channels, len) = pixel.physical_channels(order);
    channels[..len].to_vec()
}

#[test]
fn physical_channels_apply_refloat_gamma_and_color_order() {
    let pixel = FloatOutBoyLedPixel {
        channels: [16, 64, 128, 255],
    };

    assert_eq!(
        physical_channels(pixel, FloatOutBoyLedColorOrder::Grb),
        [16, 1, 64]
    );
    assert_eq!(
        physical_channels(pixel, FloatOutBoyLedColorOrder::Grbw),
        [16, 1, 64, 255]
    );
    assert_eq!(
        physical_channels(pixel, FloatOutBoyLedColorOrder::Rgb),
        [1, 16, 64]
    );
    assert_eq!(
        physical_channels(pixel, FloatOutBoyLedColorOrder::Wrgb),
        [255, 1, 16, 64]
    );
}

#[test]
fn physical_channels_match_refloat_gamma_for_every_input() {
    for channel in 0_u8..=u8::MAX {
        let pixel = FloatOutBoyLedPixel {
            channels: [channel; 4],
        };
        let widened = u16::from(channel);
        let expected = widened
            .checked_mul(widened)
            .and_then(|square| square.checked_add(widened))
            .and_then(|value| u8::try_from(value / 256).ok())
            .expect("gamma stays in u8");

        assert_eq!(
            physical_channels(pixel, FloatOutBoyLedColorOrder::Wrgb),
            [expected; 4]
        );
    }
}

#[test]
fn pixel_blend_matches_refloat_channel_math_for_every_byte_pair() {
    for (first, second, blend, expected) in [
        (
            [0, 64, 255, 200],
            [255, 192, 0, 100],
            0.25,
            [63, 96, 191, 175],
        ),
        (
            [0, 64, 255, 200],
            [255, 192, 0, 100],
            0.5,
            [127, 128, 127, 150],
        ),
        ([5, 10, 15, 20], [25, 30, 35, 40], f32::NAN, [0; 4]),
        ([5, 10, 15, 20], [25, 30, 35, 40], -1.0, [5, 10, 15, 20]),
        ([5, 10, 15, 20], [25, 30, 35, 40], 2.0, [25, 30, 35, 40]),
    ] {
        assert_eq!(
            FloatOutBoyLedPixel::blend(
                FloatOutBoyLedPixel { channels: first },
                FloatOutBoyLedPixel { channels: second },
                blend,
            )
            .channels(),
            expected,
        );
    }

    for first in 0_u8..=u8::MAX {
        for second in 0_u8..=u8::MAX {
            for blend in [f32::NAN, -1.0, 0.0, 0.1, 0.5, 0.999, 1.0, 2.0] {
                let expected = if blend <= 0.0 {
                    first
                } else if blend >= 1.0 {
                    second
                } else {
                    crate::wire::saturating_trunc_f32_to_u8(
                        f32::from(first) * (1.0 - blend) + f32::from(second) * blend,
                    )
                };

                assert_eq!(
                    FloatOutBoyLedPixel::blend(
                        FloatOutBoyLedPixel {
                            channels: [first; 4],
                        },
                        FloatOutBoyLedPixel {
                            channels: [second; 4],
                        },
                        blend,
                    )
                    .channels(),
                    [expected; 4],
                    "first={first}, second={second}, blend={blend}",
                );
            }
        }
    }
}

#[test]
fn strip_frame_maps_logical_pixels_to_checked_physical_order() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        3,
        FloatOutBoyLedColorOrder::Grb,
    )
    .reversed();
    let mut frame = super::FloatOutBoyLedStripFrame::new(config);
    let red = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red);
    let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);

    assert!(frame.set_logical_pixel(0, red));
    assert!(frame.set_logical_pixel(2, blue));
    assert!(!frame.set_logical_pixel(3, red));
    assert_eq!(frame.physical_pixel(0), Some(blue));
    assert_eq!(
        frame.physical_pixel(1),
        Some(FloatOutBoyLedPixel::default())
    );
    assert_eq!(frame.physical_pixel(2), Some(red));
    assert_eq!(frame.physical_pixel(3), None);
}

#[test]
fn oversized_strip_frames_render_up_to_physical_capacity() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        u8::MAX,
        FloatOutBoyLedColorOrder::Grb,
    );
    let mut frame = super::FloatOutBoyLedStripFrame::new(config);
    let red = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red);

    frame.render_target(
        red,
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(1.0),
    );

    assert_eq!(
        frame.physical_pixel(super::MAX_LED_STRIP_PIXELS - 1),
        Some(red)
    );
    assert_eq!(frame.physical_pixel(super::MAX_LED_STRIP_PIXELS), None);
}

#[test]
fn solid_bar_applies_brightness_on_off_fade_and_blend_like_refloat() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        2,
        FloatOutBoyLedColorOrder::Grb,
    );
    let mut frame = super::FloatOutBoyLedStripFrame::new(config);
    let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);
    assert!(frame.set_logical_pixel(0, blue));
    assert!(frame.set_logical_pixel(1, blue));
    let bar = solid_bar_with_brightness(Ratio::from_ratio_const(0.5), FloatOutBoyLedColor::Red);

    frame.render_target(
        FloatOutBoyLedPixel::from_named(bar.primary_color()),
        Ratio::clamped(bar.brightness().as_ratio() * 0.5),
        Ratio::from_ratio_const(0.5),
    );

    let expected = FloatOutBoyLedPixel {
        channels: [32, 0, 127, 0],
    };
    assert_eq!(frame.physical_pixel(0), Some(expected));
    assert_eq!(frame.physical_pixel(1), Some(expected));
}

#[test]
fn fade_and_strobe_match_refloat_time_boundaries() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let fade = full_brightness_bar(
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Blue,
        super::FloatOutBoyLedAnimationMode::Fade,
    );
    let strobe = full_brightness_bar(
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Blue,
        super::FloatOutBoyLedAnimationMode::Strobe,
    );

    let mut frame = super::FloatOutBoyLedStripFrame::new(config);
    frame.render_bar(fade, Ratio::from_ratio_const(1.0), 0.0);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue))
    );
    frame.render_bar(fade, Ratio::from_ratio_const(1.0), 0.5);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel {
            channels: [127, 0, 127, 0]
        })
    );
    frame.render_bar(fade, Ratio::from_ratio_const(1.0), 1.0);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
    );

    frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 0.999);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
    );
    frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 1.0);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue))
    );
    frame.render_bar(strobe, Ratio::from_ratio_const(1.0), 2.0);
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red))
    );
}

#[test]
fn felony_preserves_odd_center_blackout_across_all_three_phases() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        5,
        FloatOutBoyLedColorOrder::Grb,
    );
    let bar = full_brightness_bar(
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Blue,
        super::FloatOutBoyLedAnimationMode::Felony,
    );
    let red = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Red);
    let blue = FloatOutBoyLedPixel::from_named(FloatOutBoyLedColor::Blue);
    let black = FloatOutBoyLedPixel::default();
    let mut frame = super::FloatOutBoyLedStripFrame::new(config);

    frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.0);
    assert_eq!(
        core::array::from_fn(|index| frame.physical_pixel(index)),
        [Some(red), Some(red), Some(black), Some(black), Some(black)]
    );
    frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.051);
    assert_eq!(
        core::array::from_fn(|index| frame.physical_pixel(index)),
        [
            Some(black),
            Some(black),
            Some(black),
            Some(blue),
            Some(blue)
        ]
    );
    frame.render_bar(bar, Ratio::from_ratio_const(1.0), 0.101);
    assert_eq!(
        core::array::from_fn(|index| frame.physical_pixel(index)),
        [Some(blue), Some(blue), Some(black), Some(red), Some(red)]
    );
}

#[test]
fn rainbow_modes_match_refloat_hue_steps_and_strip_offsets() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        4,
        FloatOutBoyLedColorOrder::Grb,
    );
    let mut frame = super::FloatOutBoyLedStripFrame::new(config);

    frame.render_bar(
        full_brightness_bar(
            FloatOutBoyLedColor::Black,
            FloatOutBoyLedColor::Black,
            super::FloatOutBoyLedAnimationMode::RainbowCycle,
        ),
        Ratio::from_ratio_const(1.0),
        0.9,
    );
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel {
            channels: [0x7e, 0x00, 0xfe, 0]
        })
    );

    frame.render_bar(
        full_brightness_bar(
            FloatOutBoyLedColor::Black,
            FloatOutBoyLedColor::Black,
            super::FloatOutBoyLedAnimationMode::RainbowFade,
        ),
        Ratio::from_ratio_const(1.0),
        0.25,
    );
    assert_eq!(
        frame.physical_pixel(0),
        Some(FloatOutBoyLedPixel {
            channels: [0xfe, 0x79, 0x00, 0]
        })
    );

    frame.render_bar(
        full_brightness_bar(
            FloatOutBoyLedColor::Black,
            FloatOutBoyLedColor::Black,
            super::FloatOutBoyLedAnimationMode::RainbowRoll,
        ),
        Ratio::from_ratio_const(1.0),
        0.0,
    );
    assert_eq!(
        core::array::from_fn(|index| frame.physical_pixel(index)),
        [
            Some(FloatOutBoyLedPixel {
                channels: [0xf7, 0x00, 0xe2, 0]
            }),
            Some(FloatOutBoyLedPixel {
                channels: [0xfe, 0x79, 0x00, 0]
            }),
            Some(FloatOutBoyLedPixel {
                channels: [0x00, 0xff, 0x00, 0]
            }),
            Some(FloatOutBoyLedPixel {
                channels: [0x00, 0x80, 0xfd, 0]
            }),
        ]
    );
}

#[test]
fn pulse_and_knight_rider_match_refloat_spatial_frames() {
    let config = |count| {
        super::FloatOutBoyLedStripConfig::new(
            super::FloatOutBoyLedStripOrder::First,
            count,
            FloatOutBoyLedColorOrder::Grb,
        )
    };

    let mut pulse = super::FloatOutBoyLedStripFrame::new(config(5));
    pulse.render_bar(
        full_brightness_bar(
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::Pulse,
        ),
        Ratio::from_ratio_const(1.0),
        0.5,
    );
    assert_eq!(
        core::array::from_fn(|index| {
            pulse
                .physical_pixel(index)
                .map(super::FloatOutBoyLedPixel::channels)
        }),
        [
            Some([0x26, 0, 0xd8, 0]),
            Some([0xbf, 0, 0x3f, 0]),
            Some([0xbf, 0, 0x3f, 0]),
            Some([0xbf, 0, 0x3f, 0]),
            Some([0x26, 0, 0xd8, 0]),
        ]
    );

    let mut knight_rider = super::FloatOutBoyLedStripFrame::new(config(6));
    knight_rider.render_bar(
        full_brightness_bar(
            FloatOutBoyLedColor::Red,
            FloatOutBoyLedColor::Blue,
            super::FloatOutBoyLedAnimationMode::KnightRider,
        ),
        Ratio::from_ratio_const(1.0),
        1.5,
    );
    assert_eq!(
        core::array::from_fn(|index| {
            knight_rider
                .physical_pixel(index)
                .map(super::FloatOutBoyLedPixel::channels)
        }),
        [
            Some([0x3b, 0, 0xc3, 0]),
            Some([0x90, 0, 0x6e, 0]),
            Some([0xe5, 0, 0x19, 0]),
            Some([0x4c, 0, 0xb2, 0]),
            Some([0x14, 0, 0xea, 0]),
            Some([0x14, 0, 0xea, 0]),
        ]
    );
}

#[test]
fn all_transition_modes_match_refloat_frames() {
    let from = led_bar(
        Ratio::from_ratio_const(0.5),
        FloatOutBoyLedColor::Red,
        FloatOutBoyLedColor::Blue,
        super::FloatOutBoyLedAnimationMode::Solid,
    );
    let to = led_bar(
        Ratio::from_ratio_const(1.0),
        FloatOutBoyLedColor::Green,
        FloatOutBoyLedColor::Yellow,
        super::FloatOutBoyLedAnimationMode::Fade,
    );
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        4,
        FloatOutBoyLedColorOrder::Grb,
    );

    let cases = [
        (
            super::FloatOutBoyLedTransition::Fade,
            -0.5,
            7,
            [[0x87, 0x19, 0, 0]; 4],
        ),
        (
            super::FloatOutBoyLedTransition::FadeOutIn,
            0.5,
            7,
            [[0x5f, 0x3c, 0, 0]; 4],
        ),
        (
            super::FloatOutBoyLedTransition::Cipher,
            0.0,
            7,
            [
                [0x61, 0xbf, 0x6d, 0],
                [0, 0, 0, 0],
                [0x61, 0xbf, 0x6d, 0],
                [0x74, 0xbc, 0x8c, 0],
            ],
        ),
        (
            super::FloatOutBoyLedTransition::MonoCipher,
            0.0,
            7,
            [
                [0xbf, 0x3e, 0, 0],
                [0, 0, 0, 0],
                [0xbf, 0x3e, 0, 0],
                [0xbf, 0x4b, 0, 0],
            ],
        ),
    ];

    for (transition, progress, seed, expected) in cases {
        let mut frame = super::FloatOutBoyLedStripFrame::new(config);
        frame.render_bar(from, Ratio::from_ratio_const(1.0), 0.0);
        frame.render_transition(
            transition,
            progress,
            seed,
            from,
            to,
            Ratio::from_ratio_const(1.0),
        );
        assert_eq!(
            core::array::from_fn(|index| {
                frame
                    .physical_pixel(index)
                    .map(super::FloatOutBoyLedPixel::channels)
                    .unwrap_or_default()
            }),
            expected,
            "{transition:?}"
        );
    }
}

#[test]
fn footpad_and_status_progress_pixels_match_refloat() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        5,
        FloatOutBoyLedColorOrder::Grb,
    );
    let overlay = super::FloatOutBoyLedOverlay {
        strip_brightness: Ratio::from_ratio_const(1.0),
        on_off_fade: Ratio::from_ratio_const(1.0),
        blend: Ratio::from_ratio_const(1.0),
    };

    let mut footpads = super::FloatOutBoyLedStripFrame::new(config);
    footpads.render_footpads(
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(0.5),
        false,
        overlay,
    );
    assert_eq!(
        strip_channels(&footpads),
        [
            [0, 0xc0, 0xff, 0],
            [0, 0xc0, 0xff, 0],
            [0, 0x73, 0x99, 0],
            [0, 0x60, 0x80, 0],
            [0, 0x60, 0x80, 0],
        ]
    );

    let mut battery = super::FloatOutBoyLedStripFrame::new(config);
    battery.render_status_progress(
        0.45,
        super::FloatOutBoyStatusProgress::Battery,
        Ratio::from_ratio_const(0.4),
        false,
        overlay,
    );
    assert_eq!(
        strip_channels(&battery),
        [
            [0x90, 0x90, 0x90, 0],
            [0x90, 0x90, 0x90, 0],
            [0x19, 0x19, 0x19, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]
    );

    let mut duty_reversed = super::FloatOutBoyLedStripFrame::new(config);
    duty_reversed.render_status_progress(
        0.8,
        super::FloatOutBoyStatusProgress::Duty,
        Ratio::from_ratio_const(0.4),
        true,
        overlay,
    );
    assert_eq!(
        strip_channels(&duty_reversed),
        [
            [0, 0, 0, 0],
            [0xff, 0x38, 0x28, 0],
            [0xff, 0xb0, 0x30, 0],
            [0xff, 0xb0, 0x30, 0],
            [0xff, 0xb0, 0x30, 0],
        ]
    );
}

#[test]
fn disabled_and_confirmation_pixels_match_refloat() {
    let config = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        5,
        FloatOutBoyLedColorOrder::Grb,
    );
    let mut disabled = super::FloatOutBoyLedStripFrame::new(config);
    disabled.render_disabled(
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(1.0),
        1.0,
    );
    assert_eq!(
        strip_channels(&disabled),
        [
            [0x1d, 0, 0, 0],
            [0x3f, 0, 0, 0],
            [0x3f, 0, 0, 0],
            [0x3f, 0, 0, 0],
            [0x1d, 0, 0, 0],
        ]
    );

    let mut confirmation = super::FloatOutBoyLedStripFrame::new(config);
    confirmation.render_confirmation(
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(1.0),
        0.5,
    );
    assert_eq!(
        strip_channels(&confirmation),
        [
            [0, 0, 0, 0],
            [0x4e, 0x1f, 0x7d, 0],
            [0xa0, 0x40, 0xff, 0],
            [0x4e, 0x1f, 0x7d, 0],
            [0, 0, 0, 0],
        ]
    );
}

#[test]
fn front_rear_bar_selection_matches_refloat_direction_roles() {
    let config = super::FloatOutBoyLedsConfig::new(
        solid_bar(FloatOutBoyLedColor::WhiteRgb),
        solid_bar(FloatOutBoyLedColor::Red),
        solid_bar(FloatOutBoyLedColor::Blue),
        solid_bar(FloatOutBoyLedColor::Green),
        super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.9),
            Ratio::from_ratio_const(0.1),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        ),
        solid_bar(FloatOutBoyLedColor::Black),
    );

    for (headlights_on, direction_forward, expected) in [
        (
            false,
            true,
            (FloatOutBoyLedColor::Blue, FloatOutBoyLedColor::Green),
        ),
        (
            false,
            false,
            (FloatOutBoyLedColor::Blue, FloatOutBoyLedColor::Green),
        ),
        (
            true,
            true,
            (FloatOutBoyLedColor::WhiteRgb, FloatOutBoyLedColor::Red),
        ),
        (
            true,
            false,
            (FloatOutBoyLedColor::Red, FloatOutBoyLedColor::WhiteRgb),
        ),
    ] {
        let (front, rear) = super::select_front_rear_bars(config, headlights_on, direction_forward);
        assert_eq!(
            (front.primary_color(), rear.primary_color()),
            expected,
            "headlights={headlights_on} forward={direction_forward}"
        );
    }
}

#[test]
fn front_rear_renderer_composes_headlight_and_direction_transitions() {
    let config = super::FloatOutBoyLedsConfig::new(
        solid_bar(FloatOutBoyLedColor::WhiteRgb),
        solid_bar(FloatOutBoyLedColor::Red),
        solid_bar(FloatOutBoyLedColor::Blue),
        solid_bar(FloatOutBoyLedColor::Green),
        super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.9),
            Ratio::from_ratio_const(0.1),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        ),
        solid_bar(FloatOutBoyLedColor::Black),
    )
    .enabled()
    .with_headlights_on();
    let strip = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        3,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_status_strip(strip)
            .with_front_strip(strip)
            .with_rear_strip(strip);
    let input = |run_state, distance| {
        ride_only(super::FloatOutBoyLedUpdate {
            run_state,
            mode: crate::FloatOutBoyMode::Normal,
            darkride: false,
            footpad: crate::FloatOutBoyFootpadState::None,
            pitch_degrees: 1.0,
            distance,
        })
    };
    let first = |frame: &super::FloatOutBoyLedStripFrame| {
        frame
            .physical_pixel(0)
            .map(super::FloatOutBoyLedPixel::channels)
            .unwrap_or_default()
    };

    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    for tick in 1..=10 {
        renderer.update(
            config,
            input(crate::FloatOutBoyRunState::Ready, 0.0),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(first(renderer.front()), [0, 0, 0xff, 0]);
    assert_eq!(first(renderer.rear()), [0, 0xff, 0, 0]);

    renderer.update(
        config,
        input(crate::FloatOutBoyRunState::Running, 0.0),
        11.0 / 30.0,
    );
    for tick in 12..=42 {
        renderer.update(
            config,
            input(crate::FloatOutBoyRunState::Running, 0.0),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(first(renderer.front()), [0xff, 0xff, 0xff, 0]);
    assert_eq!(first(renderer.rear()), [0xff, 0, 0, 0]);

    renderer.update(
        config,
        input(crate::FloatOutBoyRunState::Running, -1.0),
        43.0 / 30.0,
    );
    assert_eq!(first(renderer.front()), [0xff, 0, 0, 0]);
    assert_eq!(first(renderer.rear()), [0xff, 0xff, 0xff, 0]);

    let mut cancelled = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    for tick in 1..=10 {
        cancelled.update(
            config,
            input(crate::FloatOutBoyRunState::Ready, 0.0),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    cancelled.update(
        config,
        input(crate::FloatOutBoyRunState::Running, 0.0),
        11.0 / 30.0,
    );
    cancelled.update(
        config,
        input(crate::FloatOutBoyRunState::Running, 0.0),
        12.0 / 30.0,
    );
    cancelled.update(
        config,
        input(crate::FloatOutBoyRunState::Ready, 0.0),
        13.0 / 30.0,
    );
    assert_eq!(first(cancelled.front()), [0, 0, 0xff, 0]);
    assert_eq!(first(cancelled.rear()), [0, 0xff, 0, 0]);
}

#[test]
fn composed_status_frame_layers_battery_duty_footpads_and_confirmation() {
    let config = super::FloatOutBoyLedsConfig::new(
        solid_bar(FloatOutBoyLedColor::WhiteRgb),
        solid_bar(FloatOutBoyLedColor::Red),
        solid_bar(FloatOutBoyLedColor::Black),
        solid_bar(FloatOutBoyLedColor::Black),
        super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.2),
            Ratio::from_ratio_const(0.4),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        ),
        solid_bar(FloatOutBoyLedColor::Blue),
    )
    .enabled();
    let strip = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        5,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_status_strip(strip);
    let input = |footpad, duty| {
        super::FloatOutBoyLedFrameUpdate::new(
            super::FloatOutBoyLedUpdate {
                run_state: crate::FloatOutBoyRunState::Running,
                mode: crate::FloatOutBoyMode::Normal,
                darkride: false,
                footpad,
                pitch_degrees: 0.0,
                distance: 0.0,
            },
            super::FloatOutBoyLedStatusUpdate {
                battery_level: 0.45,
                duty_cycle: duty,
                moving: true,
            },
        )
    };
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    for tick in 1..=10 {
        renderer.update(
            config,
            input(crate::FloatOutBoyFootpadState::None, 0.0),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(
        strip_channels(renderer.status()),
        [
            [0x90, 0x90, 0x90, 0],
            [0x90, 0x90, 0x90, 0],
            [0x19, 0x19, 0x19, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ]
    );

    for tick in 11..=17 {
        renderer.update(
            config,
            input(crate::FloatOutBoyFootpadState::None, 0.9),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(
        strip_channels(renderer.status()),
        [
            [0xff, 0xb0, 0x30, 0],
            [0xff, 0xb0, 0x30, 0],
            [0xff, 0xb0, 0x30, 0],
            [0xff, 0x38, 0x28, 0],
            [0xff, 0x38, 0x28, 0],
        ]
    );

    renderer.start_confirmation(17.0 / 30.0);
    renderer.update(
        config,
        input(crate::FloatOutBoyFootpadState::Both, 0.0),
        17.0 / 30.0 + 0.4,
    );
    assert_eq!(
        strip_channels(renderer.status()),
        [
            [0, 0, 0, 0],
            [0x4e, 0x1f, 0x7d, 0],
            [0xa0, 0x40, 0xff, 0],
            [0x4e, 0x1f, 0x7d, 0],
            [0, 0, 0, 0],
        ]
    );
}

#[test]
fn composed_status_idle_and_sensor_fade_follow_source_order() {
    let config = super::FloatOutBoyLedsConfig::new(
        solid_bar(FloatOutBoyLedColor::WhiteRgb),
        solid_bar(FloatOutBoyLedColor::Red),
        solid_bar(FloatOutBoyLedColor::Black),
        solid_bar(FloatOutBoyLedColor::Black),
        super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(1),
            Ratio::from_ratio_const(0.9),
            Ratio::from_ratio_const(0.1),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        ),
        solid_bar_with_brightness(Ratio::from_ratio_const(0.5), FloatOutBoyLedColor::Blue),
    )
    .enabled();
    let strip = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_status_strip(strip);
    let frame = super::FloatOutBoyLedFrameUpdate::new(
        super::FloatOutBoyLedUpdate {
            run_state: crate::FloatOutBoyRunState::Ready,
            mode: crate::FloatOutBoyMode::Normal,
            darkride: false,
            footpad: crate::FloatOutBoyFootpadState::None,
            pitch_degrees: 0.0,
            distance: 0.0,
        },
        super::FloatOutBoyLedStatusUpdate {
            battery_level: 1.0,
            duty_cycle: 0.0,
            moving: false,
        },
    );
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);

    renderer.update(config, frame, 1.0);
    assert_eq!(strip_channels::<1>(renderer.status())[0], [1, 1, 1, 0]);

    for tick in 1..=40 {
        renderer.update(
            config,
            frame,
            1.0 + f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(strip_channels::<1>(renderer.status())[0], [0, 0, 0x80, 0]);

    let with_footpad = |footpad| super::FloatOutBoyLedFrameUpdate {
        ride: super::FloatOutBoyLedUpdate {
            footpad,
            ..frame.ride
        },
        ..frame
    };
    for tick in 41..=43 {
        renderer.update(
            config,
            with_footpad(crate::FloatOutBoyFootpadState::Left),
            1.0 + f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(
        strip_channels::<1>(renderer.status())[0],
        [0, 0x3a, 0x4d, 0]
    );

    renderer.update(config, frame, 1.0 + 44.0 / 30.0);
    assert_eq!(
        strip_channels::<1>(renderer.status())[0],
        [0x1c, 0x3b, 0x45, 0]
    );
}

#[test]
fn lifted_status_blends_onto_front_then_idles_and_restores_bars() {
    let config = super::FloatOutBoyLedsConfig::new(
        solid_bar(FloatOutBoyLedColor::WhiteRgb),
        solid_bar(FloatOutBoyLedColor::Red),
        solid_bar(FloatOutBoyLedColor::Blue),
        solid_bar(FloatOutBoyLedColor::Red),
        super::FloatOutBoyStatusBarConfig::new(
            super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
            Ratio::from_ratio_const(0.9),
            Ratio::from_ratio_const(0.1),
            Ratio::from_ratio_const(1.0),
            Ratio::from_ratio_const(1.0),
        ),
        solid_bar(FloatOutBoyLedColor::Green),
    )
    .enabled()
    .lights_off_when_lifted()
    .status_on_front_when_lifted();
    let strip = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_front_strip(strip)
            .with_rear_strip(strip);
    let frame = |pitch_degrees| {
        super::FloatOutBoyLedFrameUpdate::new(
            super::FloatOutBoyLedUpdate {
                run_state: crate::FloatOutBoyRunState::Ready,
                mode: crate::FloatOutBoyMode::Normal,
                darkride: false,
                footpad: crate::FloatOutBoyFootpadState::None,
                pitch_degrees,
                distance: 0.0,
            },
            super::FloatOutBoyLedStatusUpdate {
                battery_level: 1.0,
                duty_cycle: 0.0,
                moving: false,
            },
        )
    };
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);

    for tick in 1..=10 {
        renderer.update(
            config,
            frame(61.0),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(
        strip_channels::<1>(renderer.front())[0],
        [0x90, 0x90, 0x90, 0]
    );
    assert_eq!(strip_channels::<1>(renderer.rear())[0], [0, 0, 0, 0]);

    let idle_boundary = 1.0 / 30.0 + 3.0;
    renderer.update(config, frame(61.0), idle_boundary);
    assert_eq!(
        strip_channels::<1>(renderer.front())[0],
        [0x90, 0x90, 0x90, 0]
    );
    for tick in 1..=10 {
        renderer.update(
            config,
            frame(61.0),
            idle_boundary + f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(strip_channels::<1>(renderer.front())[0], [0, 0, 0, 0]);

    for tick in 11..=20 {
        renderer.update(
            config,
            frame(49.0),
            idle_boundary + f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    assert_eq!(strip_channels::<1>(renderer.front())[0], [0, 0, 0xff, 0]);
    assert_eq!(strip_channels::<1>(renderer.rear())[0], [0xff, 0, 0, 0]);
}

#[test]
fn headlight_transition_uses_elapsed_time_like_refloat() {
    let config = white_led_config(super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0))
        .with_headlights_on();
    let running = super::FloatOutBoyLedUpdate {
        run_state: crate::FloatOutBoyRunState::Running,
        mode: crate::FloatOutBoyMode::Normal,
        darkride: false,
        footpad: crate::FloatOutBoyFootpadState::None,
        pitch_degrees: 0.0,
        distance: 0.0,
    };
    let mut dynamics = super::FloatOutBoyLedDynamics::new(0.0);

    dynamics.update(config, running, 10.0);
    dynamics.update(config, running, 10.25);

    assert_eq!(dynamics.headlights(), (-0.5, false, true));
}

#[test]
fn fully_faded_leds_freeze_hidden_transitions_like_refloat() {
    let config = white_led_config(super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0))
        .with_headlights_on();
    let mut off = config;
    off.power = super::FloatOutBoyLedPower::Off;
    let running = super::FloatOutBoyLedUpdate {
        run_state: crate::FloatOutBoyRunState::Running,
        mode: crate::FloatOutBoyMode::Normal,
        darkride: false,
        footpad: crate::FloatOutBoyFootpadState::None,
        pitch_degrees: 0.0,
        distance: 0.0,
    };
    let mut dynamics = super::FloatOutBoyLedDynamics::new(0.0);

    dynamics.update(config, running, 1.0);
    dynamics.update(off, running, 1.01);
    let faded_out = dynamics.headlights();
    assert!(faded_out.0 > -1.0);
    dynamics.update(off, running, 2.0);

    assert_eq!(dynamics.headlights(), faded_out);

    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal);
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, off, 0.0);
    assert!(!renderer.update(off, ride_only(running), 2.0,));
}

#[test]
fn disabled_front_stays_dark_while_lifted_like_refloat() {
    let config = white_led_config(super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0))
        .lights_off_when_lifted();
    let strip = super::FloatOutBoyLedStripConfig::new(
        super::FloatOutBoyLedStripOrder::First,
        1,
        FloatOutBoyLedColorOrder::Grb,
    );
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal)
            .with_front_strip(strip);
    let frame = |run_state| {
        ride_only(super::FloatOutBoyLedUpdate {
            run_state,
            mode: crate::FloatOutBoyMode::Normal,
            darkride: false,
            footpad: crate::FloatOutBoyFootpadState::None,
            pitch_degrees: 61.0,
            distance: 0.0,
        })
    };
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);
    for tick in 1..=12 {
        renderer.update(
            config,
            frame(crate::FloatOutBoyRunState::Ready),
            f32::from(u16::try_from(tick).unwrap_or_default()) / 30.0,
        );
    }
    renderer.update(
        config,
        frame(crate::FloatOutBoyRunState::Disabled),
        13.0 / 30.0,
    );
    renderer.update(
        config,
        frame(crate::FloatOutBoyRunState::Disabled),
        14.0 / 30.0,
    );

    assert_eq!(
        renderer
            .front()
            .physical_pixel(0)
            .map(super::FloatOutBoyLedPixel::channels),
        Some([0; 4])
    );
}

#[test]
fn first_ready_resets_animation_and_idle_epochs_like_refloat() {
    let config = white_led_config(super::FloatOutBoyStatusBarIdleTimeout::from_seconds(1));
    let hardware =
        crate::lcm::FloatOutBoyHardwareLedsConfig::new(crate::lcm::FloatOutBoyLedMode::Internal);
    let frame = |run_state| {
        ride_only(super::FloatOutBoyLedUpdate {
            run_state,
            mode: crate::FloatOutBoyMode::Normal,
            darkride: false,
            footpad: crate::FloatOutBoyFootpadState::None,
            pitch_degrees: 0.0,
            distance: 0.0,
        })
    };
    let mut renderer = super::FloatOutBoyLedRenderer::new(hardware, config, 0.0);

    renderer.update(config, frame(crate::FloatOutBoyRunState::Startup), 100.0);
    renderer.update(config, frame(crate::FloatOutBoyRunState::Ready), 100.25);

    assert_eq!(
        (
            renderer.animation_start,
            renderer.status_dynamics.idle_time,
            renderer.status_dynamics.idle_blend,
        ),
        (100.25, 100.25, 0.0)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one sequential trace verifies the coupled Refloat state-machine boundaries"
)]
fn led_dynamics_match_refloat_rate_hysteresis_and_mode_gates() {
    let mut current_time = 0.0_f32;
    macro_rules! update {
        ($state:expr, $config:expr, $run:expr, $mode:expr, $darkride:expr, $footpad:expr, $pitch:expr, $distance:expr $(,)?) => {{
            current_time += 1.0 / 30.0;
            $state.update(
                $config,
                super::FloatOutBoyLedUpdate {
                    run_state: $run,
                    mode: $mode,
                    darkride: $darkride,
                    footpad: $footpad,
                    pitch_degrees: $pitch,
                    distance: $distance,
                },
                current_time,
            )
        }};
    }
    let bar = solid_bar(FloatOutBoyLedColor::WhiteRgb);
    let status = super::FloatOutBoyStatusBarConfig::new(
        super::FloatOutBoyStatusBarIdleTimeout::from_seconds(0),
        Ratio::from_ratio_const(0.9),
        Ratio::from_ratio_const(0.1),
        Ratio::from_ratio_const(1.0),
        Ratio::from_ratio_const(0.5),
    );
    let config = super::FloatOutBoyLedsConfig::new(bar, bar, bar, bar, status, bar)
        .enabled()
        .with_headlights_on();
    let mut dynamics = super::FloatOutBoyLedDynamics::new(0.0);

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Startup,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::Left,
        70.0,
        0.0,
    );
    assert_f32_eq!(dynamics.on_off_fade().as_ratio(), 0.0);
    assert!(!dynamics.is_board_upright());

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Ready,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::Left,
        61.0,
        0.0,
    );
    assert_f32_eq!(dynamics.on_off_fade().as_ratio(), 0.1);
    assert_f32_eq!(dynamics.sensor_fades().0.as_ratio(), 1.0 / 3.0);
    assert!(dynamics.is_board_upright());

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Ready,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::None,
        50.0,
        0.0,
    );
    assert!(dynamics.is_board_upright());
    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Ready,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::None,
        49.0,
        0.0,
    );
    assert!(!dynamics.is_board_upright());

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Running,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::Both,
        -1.0,
        0.0,
    );
    assert_eq!(dynamics.direction(), (-1.0, false));
    assert_f32_eq!(dynamics.sensor_fades().0.as_ratio(), 0.0);
    assert_eq!(dynamics.headlights(), (-1.0, false, true));

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Running,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::None,
        -1.0,
        0.0,
    );
    assert!(dynamics.headlights().0 > -1.0);
    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Ready,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::None,
        -1.0,
        0.0,
    );
    assert_eq!(dynamics.headlights(), (-1.0, false, false));

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Running,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::None,
        -1.0,
        0.0,
    );
    for _ in 0..31 {
        update!(
            dynamics,
            config,
            crate::FloatOutBoyRunState::Running,
            crate::FloatOutBoyMode::Normal,
            false,
            crate::FloatOutBoyFootpadState::None,
            -1.0,
            0.0,
        );
    }
    // Refloat advances the split by 2.0 / 30.0 after the first update starts it at -1.0;
    // the next 31 updates therefore pass +1.0 and clamp there.
    assert_eq!(dynamics.headlights(), (1.0, true, false));

    update!(
        dynamics,
        config,
        crate::FloatOutBoyRunState::Running,
        crate::FloatOutBoyMode::Normal,
        true,
        crate::FloatOutBoyFootpadState::None,
        -1.0,
        1.0,
    );
    assert_eq!(dynamics.direction(), (-1.0, false));

    let showing_config = super::FloatOutBoyLedsConfig::new(
        bar,
        bar,
        bar,
        bar,
        status.showing_sensors_while_running(),
        bar,
    )
    .enabled();
    let mut showing_dynamics = super::FloatOutBoyLedDynamics::new(0.0);
    update!(
        showing_dynamics,
        showing_config,
        crate::FloatOutBoyRunState::Running,
        crate::FloatOutBoyMode::Normal,
        false,
        crate::FloatOutBoyFootpadState::Both,
        0.0,
        0.0,
    );
    assert_eq!(
        showing_dynamics.sensor_fades(),
        (Ratio::from_ratio_const(0.0), Ratio::from_ratio_const(0.0))
    );
}
