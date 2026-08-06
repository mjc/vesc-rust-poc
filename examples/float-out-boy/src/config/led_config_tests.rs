use crate::{
    lcm::FloatOutBoyLedMode,
    leds::{
        FloatOutBoyLedAnimationMode, FloatOutBoyLedColor, FloatOutBoyLedColorOrder,
        FloatOutBoyLedPin, FloatOutBoyLedPinConfig, FloatOutBoyLedStripOrder,
        FloatOutBoyLedTransition,
    },
};

use super::{
    FLOAT_OUT_BOY_DEFAULT_CONFIG, FloatOutBoyConfigImage, decode_led_config, validate_led_config,
};

#[test]
fn decodes_pinned_refloat_cutoff_default_led_config() {
    let (hardware, leds) = FloatOutBoyConfigImage::defaults()
        .led_configs()
        .expect("generated default LED fields are valid");

    assert_eq!(hardware.mode(), FloatOutBoyLedMode::Off);
    assert_eq!(hardware.pin(), FloatOutBoyLedPin::B7);
    assert_eq!(hardware.pin_config(), FloatOutBoyLedPinConfig::PullupTo5v);
    assert_eq!(
        (
            hardware.status_strip().order(),
            hardware.status_strip().count(),
            hardware.status_strip().color_order(),
            hardware.status_strip().is_reversed(),
        ),
        (
            FloatOutBoyLedStripOrder::First,
            10,
            FloatOutBoyLedColorOrder::Grb,
            false,
        )
    );
    assert!(leds.is_enabled());
    assert!(leds.are_headlights_on());
    assert_eq!(leds.headlights_transition(), FloatOutBoyLedTransition::Fade);
    assert_eq!(leds.direction_transition(), FloatOutBoyLedTransition::Fade);
    assert!(leds.turns_lights_off_when_lifted());
    assert!(leds.shows_status_on_front_when_lifted());
    assert_eq!(
        leds.front().animation_mode(),
        FloatOutBoyLedAnimationMode::KnightRider
    );
    assert_eq!(leds.front().primary_color(), FloatOutBoyLedColor::Red);
    assert_eq!(leds.rear().primary_color(), FloatOutBoyLedColor::Azure);
    assert_eq!(
        leds.headlights().primary_color(),
        FloatOutBoyLedColor::WhiteFull
    );
    assert_eq!(leds.taillights().primary_color(), FloatOutBoyLedColor::Red);
    assert!(leds.status().shows_sensors_while_running());
    assert_eq!(leds.status().idle_timeout(), 0);
}

#[test]
fn decodes_hardware_mode_from_cutoff_byte_232() {
    let mut bytes = FLOAT_OUT_BOY_DEFAULT_CONFIG;
    bytes[229] = FloatOutBoyLedColor::Fuchsia.id();
    bytes[232] = FloatOutBoyLedMode::Both.id();
    bytes[233] = FloatOutBoyLedPin::C9.id();
    bytes[234] = FloatOutBoyLedPinConfig::NoPullup.id();
    bytes[235] = FloatOutBoyLedStripOrder::Third.id();
    bytes[237] = FloatOutBoyLedColorOrder::Wrgb.id();
    bytes[238] = 1;

    let image = FloatOutBoyConfigImage::from_serialized(&bytes).expect("valid image");
    let (hardware, _) = image.led_configs().expect("valid LED fields");

    let mode: FloatOutBoyLedMode = image.hardware_led_mode();
    assert_eq!(mode, FloatOutBoyLedMode::Both);
    assert_eq!(hardware.mode(), FloatOutBoyLedMode::Both);
    assert_eq!(hardware.pin(), FloatOutBoyLedPin::C9);
    assert_eq!(hardware.pin_config(), FloatOutBoyLedPinConfig::NoPullup);
    assert_eq!(
        hardware.status_strip().order(),
        FloatOutBoyLedStripOrder::Third
    );
    assert_eq!(
        hardware.status_strip().color_order(),
        FloatOutBoyLedColorOrder::Wrgb
    );
    assert!(hardware.status_strip().is_reversed());
}

#[test]
fn serialized_image_rejects_out_of_range_refloat_led_enums() {
    for offset in [182, 186, 189, 232, 233, 234, 235, 237] {
        let mut bytes = FLOAT_OUT_BOY_DEFAULT_CONFIG;
        bytes[offset] = u8::MAX;

        assert!(
            FloatOutBoyConfigImage::from_serialized(&bytes).is_none(),
            "offset {offset}"
        );
    }
}

#[test]
fn led_validation_acceptance_matches_typed_decode_for_every_single_byte_mutation() {
    for offset in 180..247 {
        for value in u8::MIN..=u8::MAX {
            let mut bytes = FLOAT_OUT_BOY_DEFAULT_CONFIG;
            bytes[offset] = value;

            assert_eq!(
                validate_led_config(&bytes).is_some(),
                decode_led_config(&bytes).is_some(),
                "different acceptance at offset {offset} for value {value}"
            );
        }
    }
}
