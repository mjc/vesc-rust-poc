use super::{FloatOutBoyHardwareLedsConfig, FloatOutBoyLedMode};
use crate::{
    FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
    FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
};

#[test]
fn float_out_boy_led_mode_matches_upstream_flag_ids() {
    // C map: Float Out Boy v1.2.1 treats LED mode as flags at
    // `third_party/float-out-boy/src/leds.c:795-830` and external-LCM mode details at
    // `third_party/float-out-boy/src/lcm.c:27-28`; the typed mode IDs mirror
    // `third_party/float-out-boy/src/conf/datatypes.h:36-60`.
    let disabled = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);
    let internal = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal);
    let external = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::External);
    let both = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both);

    assert_eq!(FloatOutBoyLedMode::Off.id(), 0);
    assert_eq!(FloatOutBoyLedMode::Internal.id(), 0x1);
    assert_eq!(FloatOutBoyLedMode::External.id(), 0x2);
    assert_eq!(FloatOutBoyLedMode::Both.id(), 0x3);
    assert!(!disabled.uses_internal_leds());
    assert!(!disabled.uses_external_leds());
    assert!(internal.uses_internal_leds());
    assert!(!internal.uses_external_leds());
    assert!(!external.uses_internal_leds());
    assert!(external.uses_external_leds());
    assert!(both.uses_internal_leds());
    assert!(both.uses_external_leds());
}

#[test]
fn float_out_boy_hardware_leds_default_and_overrides_match_upstream_shape() {
    // C map: Float Out Boy's default hardware LED settings come from
    // `third_party/float-out-boy/src/conf/settings.xml:3560-3863`; the mode/pin/pin-config
    // wiring follows the same flags behavior as `third_party/float-out-boy/src/leds.c:795-830`
    // and `third_party/float-out-boy/src/lcm.c:27-28`.
    let defaults = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);

    assert_eq!(defaults.pin(), FloatOutBoyLedPin::B7);
    assert_eq!(defaults.pin_config(), FloatOutBoyLedPinConfig::PullupTo5v);
    assert_eq!(
        defaults.status_strip().order(),
        FloatOutBoyLedStripOrder::First
    );
    assert_eq!(defaults.status_strip().count(), 10);
    assert_eq!(
        defaults.front_strip().order(),
        FloatOutBoyLedStripOrder::Second
    );
    assert_eq!(defaults.front_strip().count(), 20);
    assert_eq!(
        defaults.rear_strip().order(),
        FloatOutBoyLedStripOrder::Third
    );
    assert_eq!(defaults.rear_strip().count(), 20);

    let status_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        8,
        FloatOutBoyLedColorOrder::Grbw,
    );
    let front_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Second,
        24,
        FloatOutBoyLedColorOrder::Rgb,
    );
    let rear_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Third,
        24,
        FloatOutBoyLedColorOrder::Grb,
    )
    .reversed();

    let hardware_leds = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both)
        .with_pin(FloatOutBoyLedPin::C9)
        .with_pin_config(FloatOutBoyLedPinConfig::NoPullup)
        .with_status_strip(status_strip)
        .with_front_strip(front_strip)
        .with_rear_strip(rear_strip);

    assert_eq!(hardware_leds.mode(), FloatOutBoyLedMode::Both);
    assert_eq!(hardware_leds.pin(), FloatOutBoyLedPin::C9);
    assert_eq!(
        hardware_leds.pin_config(),
        FloatOutBoyLedPinConfig::NoPullup
    );
    assert_eq!(
        hardware_leds.status_strip().color_order(),
        FloatOutBoyLedColorOrder::Grbw
    );
    assert_eq!(
        hardware_leds.front_strip().color_order(),
        FloatOutBoyLedColorOrder::Rgb
    );
    assert!(hardware_leds.rear_strip().is_reversed());
}
